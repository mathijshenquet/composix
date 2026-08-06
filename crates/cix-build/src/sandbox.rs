//! Request/result boundary for traced build sandboxes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::evaluation::ResolvedCommand;
use crate::fetch::CredentialMount;
use crate::{fhs, seccomp, trace, ScratchDir};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RunNetwork {
    Namespace,
    SocketFilter,
}

pub(crate) struct SandboxRequest<'a> {
    pub(crate) workdir: &'a Path,
    pub(crate) command: &'a ResolvedCommand,
    pub(crate) environment: &'a BTreeMap<String, String>,
    pub(crate) export_prelude: &'a BTreeMap<String, String>,
    pub(crate) offered_closure: &'a BTreeSet<String>,
    pub(crate) imports: &'a [String],
    pub(crate) run_network: Option<RunNetwork>,
    pub(crate) credentials: &'a [&'a CredentialMount],
}

pub(crate) struct Sandbox;

impl Sandbox {
    pub(crate) fn shell(imports: &[String]) -> Result<String> {
        imports
            .iter()
            .map(|package| Path::new(package).join("bin/bash"))
            .find(|candidate| candidate.is_file())
            .map(|candidate| candidate.to_string_lossy().into_owned())
            .context("RUN/FETCH requires bash in an IMPORTed package")
    }

    pub(crate) fn run_network() -> Result<RunNetwork> {
        let output = Command::new("bwrap")
            .args([
                "--die-with-parent",
                "--new-session",
                "--unshare-user",
                "--uid",
                "0",
                "--gid",
                "0",
                "--unshare-net",
                "--ro-bind",
                "/",
                "/",
                "--",
                "true",
            ])
            .output()
            .context(
                "probing bubblewrap network isolation; this host may restrict unprivileged user namespaces",
            )?;
        Ok(if output.status.success() {
            RunNetwork::Namespace
        } else {
            RunNetwork::SocketFilter
        })
    }

    pub(crate) fn execute(request: SandboxRequest<'_>) -> Result<trace::Capture> {
        let import_union = prepare_import_union(request.imports, request.run_network.is_none())?;
        let loader_surface = fhs::LoaderSurface::new(request.imports)?;
        let env_is_missing = !import_union.path().join("bin/env").is_file();
        let trace_directory =
            ScratchDir::new("cix-read-trace-").context("creating read trace directory")?;
        let trace_path = trace_directory.path().join("syscalls");
        let heredoc_path = match request.command {
            ResolvedCommand::Heredoc { body, .. } => {
                let path = trace_directory.path().join("heredoc");
                fs::write(&path, body).context("writing RUN/FETCH heredoc body")?;
                Some(path)
            }
            ResolvedCommand::Legacy { .. } | ResolvedCommand::Argv { .. } => None,
        };
        let mut process = Command::new("strace");
        process
            .args([
                "-f",
                "--seccomp-bpf",
                "--decode-pids=pidns",
                "-qq",
                "-yy",
                "-s",
                "0",
                "-e",
            ])
            .arg("trace=%file,getdents,getdents64,chdir,fchdir,clone,clone3,fork,vfork")
            .arg("-o")
            .arg(&trace_path)
            .args(["--", "bwrap"]);
        process.args([
            "--die-with-parent",
            "--new-session",
            "--unshare-user",
            "--uid",
            "0",
            "--gid",
            "0",
            "--unshare-pid",
            "--unshare-uts",
            "--unshare-ipc",
            "--unshare-cgroup",
        ]);
        if request.run_network == Some(RunNetwork::Namespace) {
            process.arg("--unshare-net");
        }
        let seccomp_filter = if request.run_network == Some(RunNetwork::SocketFilter) {
            Some(seccomp::prepare_socket_filter(&mut process)?)
        } else {
            None
        };
        if let Some(filter) = &seccomp_filter {
            process.arg("--seccomp").arg(filter.as_raw_fd().to_string());
        }
        process.args(["--dir", "/nix", "--dir", "/nix/store"]);
        process.args(["--dir", "/usr", "--dir", "/usr/bin"]);
        process.args(["--symlink", "/bin/env", "/usr/bin/env"]);
        loader_surface.mount(&mut process);
        for path in request.offered_closure {
            process.args(["--ro-bind", path, path]);
        }
        if heredoc_path.is_some() || !request.credentials.is_empty() {
            process.args(["--dir", "/run"]);
        }
        if let Some(path) = &heredoc_path {
            process.arg("--ro-bind").arg(path).arg("/run/cix-heredoc");
        }
        for credential in request.credentials {
            let destination = format!("/run/cix-credentials/{}", credential.name);
            process.args(["--dir", "/run/cix-credentials"]);
            process
                .arg("--ro-bind")
                .arg(&credential.source)
                .arg(&destination);
        }
        for subtree in ["bin", "etc", "share"] {
            let source = import_union.path().join(subtree);
            if source.is_dir() {
                process
                    .arg("--ro-bind")
                    .arg(&source)
                    .arg(Path::new("/").join(subtree));
            }
        }
        process
            .args(["--bind"])
            .arg(request.workdir)
            .arg("/work")
            .args([
                "--proc",
                "/proc",
                "--dev",
                "/dev",
                "--tmpfs",
                "/tmp",
                "--chdir",
                "/work",
                "--clearenv",
            ]);
        if !import_union.path().join("etc").is_dir() {
            process.args(["--dir", "/etc"]);
        }
        for (name, value) in request.environment {
            process.args(["--setenv", name, value]);
        }
        if let Some(credential) = request.credentials.first() {
            process
                .arg("--setenv")
                .arg("CIX_FETCH_CREDENTIAL_FILE")
                .arg(format!("/run/cix-credentials/{}", credential.name));
            process
                .arg("--setenv")
                .arg("CIX_FETCH_TOKEN")
                .arg(&credential.name);
        }
        match request.command {
            ResolvedCommand::Legacy { command } => {
                let exports = request
                    .export_prelude
                    .iter()
                    .map(|(name, value)| format!("export {name}={value};"))
                    .collect::<String>();
                let shell_program = format!("umask 022; {exports}eval \"$1\"");
                process
                    .arg("/bin/bash")
                    .args(["-c", &shell_program, "cix-build", command]);
            }
            ResolvedCommand::Argv { argv } => {
                let (program, arguments) = argv
                    .split_first()
                    .context("internal RUN/FETCH argv is empty")?;
                process.arg(program).args(arguments);
            }
            ResolvedCommand::Heredoc { interpreter, .. } => {
                process.arg(interpreter).arg("/run/cix-heredoc");
            }
        }
        let output = process
            .output()
            .context(
                "starting traced bubblewrap sandbox; this host must permit ptrace and unprivileged user namespaces",
            )?;
        io::stderr().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
        if !output.status.success() {
            let mut failure = failure_message(output.status, request.run_network);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let trace_text = fs::read_to_string(&trace_path).ok();
            if env_is_missing {
                failure.push_str(
                    "\nhint: /usr/bin/env is a fixed alias to /bin/env; IMPORT ${pkgs.coreutils} or another package that supplies env",
                );
            }
            if let Some(trace_text) = &trace_text {
                if let Some(hint) = fhs::failure_hint(
                    request.workdir,
                    request.imports,
                    &trace::parse_failure(trace_text),
                ) {
                    failure.push('\n');
                    failure.push_str(&hint);
                }
            }
            for hint in failure_problem_hints(
                output.status.code(),
                request.run_network.is_none(),
                &stdout,
                &stderr,
                trace_text.as_deref(),
            ) {
                failure.push('\n');
                failure.push_str(hint);
            }
            if !stderr.trim().is_empty() {
                failure.push_str("\ncommand stderr:\n");
                failure.push_str(stderr.trim());
            }
            bail!("{failure}");
        }
        let trace_text = fs::read_to_string(&trace_path)
            .with_context(|| format!("reading syscall trace {}", trace_path.display()))?;
        Ok(trace::parse(&trace_text))
    }
}

pub(crate) fn prepare_import_union(
    imports: &[String],
    include_network_configuration: bool,
) -> Result<ScratchDir> {
    let union = ScratchDir::new("cix-import-union-").context("creating IMPORT package union")?;
    for package in imports {
        let package = Path::new(package);
        if !package.is_absolute() {
            bail!(
                "IMPORT resolved to non-absolute package path {}",
                package.display()
            );
        }
        for subtree in ["bin", "etc", "share"] {
            let source = package.join(subtree);
            if !source.is_dir() {
                continue;
            }
            let destination = union.path().join(subtree);
            fs::create_dir_all(&destination)?;
            merge_import_directory(&source, &destination)?;
        }
    }
    if include_network_configuration {
        let etc = union.path().join("etc");
        fs::create_dir_all(&etc)?;
        for source in ["/etc/hosts", "/etc/nsswitch.conf", "/etc/resolv.conf"] {
            let source = Path::new(source);
            if !source.is_file() {
                continue;
            }
            let destination = etc.join(source.file_name().expect("network file has a name"));
            remove_path_if_present(&destination)?;
            fs::copy(source, &destination)?;
        }
    }
    Ok(union)
}

fn merge_import_directory(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("reading IMPORT subtree {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let source_metadata = fs::symlink_metadata(&source_path)?;
        let destination_metadata = fs::symlink_metadata(&destination_path).ok();
        if let Some(destination_metadata) = destination_metadata {
            if source_metadata.is_dir()
                && !source_metadata.file_type().is_symlink()
                && destination_metadata.is_dir()
                && !destination_metadata.file_type().is_symlink()
            {
                merge_import_directory(&source_path, &destination_path)?;
            }
            continue;
        }
        if source_metadata.is_dir() && !source_metadata.file_type().is_symlink() {
            fs::create_dir(&destination_path)?;
            merge_import_directory(&source_path, &destination_path)?;
        } else {
            symlink(&source_path, &destination_path)?;
        }
    }
    Ok(())
}

fn remove_path_if_present(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        crate::workspace::make_writable(path)?;
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("removing {}", path.display()))
}

pub(crate) fn failure_message(
    status: impl std::fmt::Display,
    run_network: Option<RunNetwork>,
) -> String {
    let mut message = format!(
        "bubblewrap sandbox or command exited {status}; sandboxing was not weakened (enable unprivileged user namespaces if bwrap reported a namespace permission error)"
    );
    if run_network == Some(RunNetwork::SocketFilter) {
        message.push_str(
            "\nhint: this RUN used the socket-filter fallback because the host rejected bubblewrap's network namespace (often an AppArmor restriction); localhost networking (127.0.0.1) was unavailable",
        );
    }
    message
}

pub(crate) fn failure_problem_hints(
    exit_code: Option<i32>,
    fetch: bool,
    stdout: &str,
    stderr: &str,
    trace: Option<&str>,
) -> Vec<&'static str> {
    let mut hints = Vec::new();
    if fetch
        && exit_code == Some(124)
        && trace.is_some_and(|trace| {
            trace
                .lines()
                .rev()
                .take(256)
                .filter(|line| hashed_certificate_probe_miss(line))
                .take(3)
                .count()
                == 3
        })
    {
        hints.push(
            "hint: TLS-trust masquerade: this FETCH timed out after repeated failed certificate probes; IMPORT ${pkgs.cacert} (or another declared CA bundle); see docs/cixfile.md#fetch-tls-trust",
        );
    }
    if [stdout, stderr].iter().any(|output| {
        output.contains("ERR_PNPM_NO_OFFLINE_TARBALL") || output.contains("ERR_PNPM_FROZEN_STORE_")
    }) {
        hints.push(
            "hint: pnpm offline/store wall: seal the complete fetched store and install with frozen-store=true, --offline, and --frozen-lockfile using pnpm >=11.7 and Node >=22.15; see docs/cixfile.md#pnpm-frozen-store",
        );
    }
    hints
}

fn hashed_certificate_probe_miss(line: &str) -> bool {
    if !line.contains("ENOENT") {
        return false;
    }
    let Some(after_certs) = line.split("/ssl/certs/").nth(1) else {
        return false;
    };
    let name = after_certs
        .split(['"', '/', ' ', ','])
        .next()
        .unwrap_or_default();
    let Some(hash) = name.strip_suffix(".0") else {
        return false;
    };
    hash.len() == 8 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}
