use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::codegen::{generate_build_context_nix, generate_offer_build_nix};
use crate::seccomp;
use crate::{BuildStep, Cixfile, FetchPin, LockFile, MemoEntry};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuildContext {
    offers: Vec<String>,
    paths: Vec<String>,
    commands: Vec<String>,
    environment: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct MemoRequest<'a> {
    command: &'a str,
    offered_closure: &'a BTreeSet<String>,
    incoming_nar_hash: &'a str,
    environment: &'a BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunNetwork {
    Namespace,
    SocketFilter,
}

pub(crate) fn execute(
    cixfile: &Cixfile,
    directory: &Path,
    lock: &mut LockFile,
    system: &str,
    update_fetch_pins: bool,
) -> Result<Option<String>> {
    if cixfile.steps.is_empty() {
        return Ok(None);
    }

    let command_count = cixfile
        .steps
        .iter()
        .filter(|step| !matches!(step, BuildStep::Copy { .. }))
        .count();
    let context = if command_count == 0 {
        BuildContext {
            offers: Vec::new(),
            paths: Vec::new(),
            commands: Vec::new(),
            environment: BTreeMap::new(),
        }
    } else {
        resolve_context(cixfile, lock, system)?
    };
    if context.commands.len() != command_count {
        bail!(
            "internal build context mismatch: resolved {} commands for {command_count} steps",
            context.commands.len()
        );
    }

    let offered_closure = if context.offers.is_empty() {
        BTreeSet::new()
    } else {
        realize_offers(cixfile, lock, system)?;
        query_closure(&context.offers)?
    };
    let shell = if command_count == 0 {
        None
    } else {
        Some(find_shell(&context.paths)?)
    };
    let run_network = if cixfile
        .steps
        .iter()
        .any(|step| matches!(step, BuildStep::Run { .. }))
    {
        Some(probe_run_network(
            shell.as_deref().expect("RUN steps have a shell"),
        )?)
    } else {
        None
    };
    let mut environment = context.environment;
    environment.insert("HOME".into(), "/work".into());
    environment.insert("LC_ALL".into(), "C".into());
    environment.insert("PATH".into(), context.paths.join(":"));
    environment.insert("SOURCE_DATE_EPOCH".into(), "1".into());
    environment.insert("TMPDIR".into(), "/tmp".into());
    environment.insert("TZ".into(), "UTC".into());

    let empty = tempfile::Builder::new()
        .prefix("cix-build-empty-")
        .tempdir()
        .context("creating initial build workdir")?;
    let (mut current_snapshot, mut snapshot_hash) = snapshot(empty.path())?;
    let mut command_index = 0;

    for (index, step) in cixfile.steps.iter().enumerate() {
        match step {
            BuildStep::Copy {
                src,
                dst,
                line,
                source,
            } => {
                let work = seeded_workdir(&current_snapshot)?;
                copy_input(directory, src, dst, work.path())
                    .with_context(|| format!("line {line}: COPY failed\n  | {source:?}"))?;
                (current_snapshot, snapshot_hash) = snapshot(work.path())?;
                eprintln!(
                    "step {} COPY {} -> {} snapshot {}",
                    index + 1,
                    src,
                    dst,
                    current_snapshot
                );
            }
            BuildStep::Fetch { line, source, .. } | BuildStep::Run { line, source, .. } => {
                let command = &context.commands[command_index];
                command_index += 1;
                let is_fetch = matches!(step, BuildStep::Fetch { .. });
                let kind = if is_fetch { "FETCH" } else { "RUN" };
                let keyed_command = format!("{kind}\0{command}");
                let key = memo_key(
                    &keyed_command,
                    &offered_closure,
                    &snapshot_hash,
                    &environment,
                )?;
                let fetch_id = is_fetch.then(|| fetch_id(index, command));
                let force = is_fetch && update_fetch_pins;

                if !force {
                    if let Some(entry) = lock.memo.get(&key) {
                        if ensure_store_path(&entry.store_path)? {
                            let actual_hash = nar_hash(Path::new(&entry.store_path))?;
                            if actual_hash == entry.output_nar_hash {
                                if let Some(id) = &fetch_id {
                                    verify_fetch_pin(
                                        lock.fetches.get(id),
                                        &entry.output_nar_hash,
                                    )
                                    .with_context(|| {
                                        format!(
                                            "line {line}: FETCH pin verification failed\n  | {source:?}"
                                        )
                                    })?;
                                }
                                current_snapshot = entry.store_path.clone();
                                snapshot_hash = entry.output_nar_hash.clone();
                                eprintln!(
                                    "step {} {kind} memo hit {} -> {}",
                                    index + 1,
                                    short_key(&key),
                                    current_snapshot
                                );
                                continue;
                            }
                            eprintln!(
                                "step {} {kind} memo stale {} (recorded NAR hash does not match store path)",
                                index + 1,
                                short_key(&key)
                            );
                        }
                    }
                }

                let work = seeded_workdir(&current_snapshot)?;
                let started = Instant::now();
                run_sandbox(
                    work.path(),
                    shell.as_deref().expect("command steps have a shell"),
                    command,
                    &environment,
                    &offered_closure,
                    if is_fetch { None } else { run_network },
                )
                .with_context(|| format!("line {line}: {kind} failed\n  | {source:?}"))?;
                let wall_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                let (output_path, output_hash) = snapshot(work.path())?;

                if let Some(id) = fetch_id {
                    if update_fetch_pins {
                        lock.fetches.insert(
                            id,
                            FetchPin {
                                nar_hash: output_hash.clone(),
                            },
                        );
                    } else if let Some(pin) = lock.fetches.get(&id) {
                        verify_fetch_pin(Some(pin), &output_hash).with_context(|| {
                            format!("line {line}: FETCH output changed\n  | {source:?}")
                        })?;
                    } else {
                        lock.fetches.insert(
                            id,
                            FetchPin {
                                nar_hash: output_hash.clone(),
                            },
                        );
                    }
                }
                lock.memo.insert(
                    key.clone(),
                    MemoEntry {
                        output_nar_hash: output_hash.clone(),
                        store_path: output_path.clone(),
                        wall_ms,
                    },
                );
                current_snapshot = output_path;
                snapshot_hash = output_hash;
                eprintln!(
                    "step {} {kind} memo miss {} ({} ms) -> {}",
                    index + 1,
                    short_key(&key),
                    wall_ms,
                    current_snapshot
                );
            }
        }
    }
    Ok(Some(current_snapshot))
}

fn ensure_store_path(path: &str) -> Result<bool> {
    if Path::new(path).exists() {
        return Ok(true);
    }
    let output = Command::new("nix-store")
        .args(["--realise", path])
        .output()
        .with_context(|| format!("asking substituters for memo output {path}"))?;
    Ok(output.status.success() && Path::new(path).exists())
}

fn resolve_context(cixfile: &Cixfile, lock: &LockFile, system: &str) -> Result<BuildContext> {
    let expression = generate_build_context_nix(cixfile, lock, system)?;
    let raw = cix_common::nix(&["eval", "--impure", "--json", "--expr", &expression])
        .context("resolving RUN/FETCH build context from locked FROM inputs")?;
    serde_json::from_str(&raw).context("parsing resolved RUN/FETCH build context")
}

fn realize_offers(cixfile: &Cixfile, lock: &LockFile, system: &str) -> Result<()> {
    let expression = generate_offer_build_nix(cixfile, lock, system)?;
    cix_common::nix(&[
        "build",
        "--impure",
        "--no-link",
        "--print-out-paths",
        "--expr",
        &expression,
    ])
    .context("realizing offered RUN/FETCH closure")?;
    Ok(())
}

fn query_closure(offers: &[String]) -> Result<BTreeSet<String>> {
    let output = Command::new("nix-store")
        .args(["--query", "--requisites"])
        .args(offers)
        .output()
        .context("executing nix-store to resolve offered closure")?;
    if !output.status.success() {
        bail!(
            "nix-store --query --requisites failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout).context("nix-store returned non-UTF-8 paths")?;
    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

fn find_shell(paths: &[String]) -> Result<String> {
    paths
        .iter()
        .map(|directory| Path::new(directory).join("bash"))
        .find(|candidate| candidate.is_file())
        .map(|candidate| candidate.to_string_lossy().into_owned())
        .context("RUN/FETCH requires bash in a declared PATH directory")
}

fn run_sandbox(
    workdir: &Path,
    shell: &str,
    command: &str,
    environment: &BTreeMap<String, String>,
    offered_closure: &BTreeSet<String>,
    run_network: Option<RunNetwork>,
) -> Result<()> {
    let mut process = Command::new("bwrap");
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
    if run_network == Some(RunNetwork::Namespace) {
        process.arg("--unshare-net");
    }
    let _seccomp_filter = if run_network == Some(RunNetwork::SocketFilter) {
        Some(seccomp::attach_socket_filter(&mut process)?)
    } else {
        None
    };
    process.args(["--dir", "/nix", "--dir", "/nix/store"]);
    for path in offered_closure {
        process.args(["--ro-bind", path, path]);
    }
    process.args(["--bind"]).arg(workdir).arg("/work").args([
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/etc",
        "--chdir",
        "/work",
        "--clearenv",
    ]);
    if run_network.is_none() {
        for path in ["/etc/hosts", "/etc/nsswitch.conf", "/etc/resolv.conf"] {
            if Path::new(path).is_file() {
                process.args(["--ro-bind", path, path]);
            }
        }
    }
    for (name, value) in environment {
        process.args(["--setenv", name, value]);
    }
    let status = process
        .arg(shell)
        .args(["-c", "umask 022; eval \"$1\"", "cix-build", command])
        .status()
        .context(
            "starting bubblewrap sandbox; this host may restrict unprivileged user namespaces",
        )?;
    if !status.success() {
        bail!("{}", sandbox_failure(status, run_network));
    }
    Ok(())
}

fn probe_run_network(shell: &str) -> Result<RunNetwork> {
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
            shell,
            "-c",
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

fn sandbox_failure(status: impl std::fmt::Display, run_network: Option<RunNetwork>) -> String {
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

fn seeded_workdir(snapshot: &str) -> Result<tempfile::TempDir> {
    let work = tempfile::Builder::new()
        .prefix("cix-build-work-")
        .tempdir()
        .context("creating build workdir")?;
    copy_tree(Path::new(snapshot), work.path())?;
    make_writable(work.path())?;
    Ok(work)
}

fn copy_input(directory: &Path, src: &str, dst: &str, workdir: &Path) -> Result<()> {
    let source = directory.join(src);
    if !source.is_file() {
        bail!("COPY source {} is not a regular file", source.display());
    }
    let relative = dst.strip_prefix('/').unwrap_or(dst);
    let destination = workdir.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating COPY destination {}", parent.display()))?;
    }
    fs::copy(&source, &destination).with_context(|| {
        format!(
            "copying {} to build workdir destination {}",
            source.display(),
            destination.display()
        )
    })?;
    fs::set_permissions(&destination, fs::Permissions::from_mode(0o644))?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("reading snapshot directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            symlink(fs::read_link(&source_path)?, &destination_path)?;
        } else if metadata.is_dir() {
            fs::create_dir(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else {
            bail!(
                "unsupported special file in build snapshot: {}",
                source_path.display()
            );
        }
    }
    Ok(())
}

fn make_writable(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.is_dir() {
            make_writable(&entry_path)?;
        }
        if !metadata.file_type().is_symlink() {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0o200);
            fs::set_permissions(&entry_path, permissions)?;
        }
    }
    Ok(())
}

fn snapshot(path: &Path) -> Result<(String, String)> {
    let path_text = path
        .to_str()
        .context("build workdir path is not valid UTF-8")?;
    let nar_hash = nar_hash(path)?;
    let store_path = cix_common::nix(&[
        "store",
        "add",
        "--mode",
        "nar",
        "--name",
        "cix-build-snapshot",
        path_text,
    ])?
    .lines()
    .last()
    .filter(|line| !line.is_empty())
    .map(ToOwned::to_owned)
    .context("nix store add did not return a snapshot store path")?;
    Ok((store_path, nar_hash))
}

fn nar_hash(path: &Path) -> Result<String> {
    let path_text = path.to_str().context("path is not valid UTF-8")?;
    Ok(
        cix_common::nix(&["hash", "path", "--mode", "nar", path_text])?
            .trim()
            .to_owned(),
    )
}

fn memo_key(
    command: &str,
    offered_closure: &BTreeSet<String>,
    incoming_nar_hash: &str,
    environment: &BTreeMap<String, String>,
) -> Result<String> {
    let request = serde_json::to_vec(&MemoRequest {
        command,
        offered_closure,
        incoming_nar_hash,
        environment,
    })?;
    Ok(hex_hash(&request))
}

fn fetch_id(index: usize, command: &str) -> String {
    format!("{index}-{}", short_key(&hex_hash(command.as_bytes())))
}

fn verify_fetch_pin(pin: Option<&FetchPin>, actual: &str) -> Result<()> {
    if let Some(pin) = pin {
        if pin.nar_hash != actual {
            bail!(
                "FETCH hash mismatch: lock pins {}, fetched {}; rerun with --update-lock to accept the new output",
                pin.nar_hash,
                actual
            );
        }
    }
    Ok(())
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn short_key(key: &str) -> &str {
    &key[..12.min(key.len())]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn closure(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| (*path).to_owned()).collect()
    }

    #[test]
    fn memo_key_tracks_command_closure_snapshot_and_environment() {
        let environment = BTreeMap::from([("PATH".into(), "/nix/store/tool/bin".into())]);
        let base = memo_key(
            "RUN\0cargo build",
            &closure(&["/nix/store/tool"]),
            "sha256-input",
            &environment,
        )
        .unwrap();
        assert_eq!(
            base,
            memo_key(
                "RUN\0cargo build",
                &closure(&["/nix/store/tool"]),
                "sha256-input",
                &environment,
            )
            .unwrap()
        );
        assert_ne!(
            base,
            memo_key(
                "RUN\0cargo test",
                &closure(&["/nix/store/tool"]),
                "sha256-input",
                &environment,
            )
            .unwrap()
        );
        let changed_environment =
            BTreeMap::from([("PATH".into(), "/nix/store/other-tool/bin".into())]);
        assert_ne!(
            base,
            memo_key(
                "RUN\0cargo build",
                &closure(&["/nix/store/tool"]),
                "sha256-input",
                &changed_environment,
            )
            .unwrap()
        );
        assert_ne!(
            base,
            memo_key(
                "RUN\0cargo build",
                &closure(&["/nix/store/new-tool"]),
                "sha256-input",
                &environment,
            )
            .unwrap()
        );
        assert_ne!(
            base,
            memo_key(
                "RUN\0cargo build",
                &closure(&["/nix/store/tool"]),
                "sha256-source-edit",
                &environment,
            )
            .unwrap()
        );
    }

    #[test]
    fn source_edits_only_invalidate_steps_after_the_changed_copy() {
        let environment = BTreeMap::new();
        let offered = closure(&["/nix/store/tool"]);
        let cook_before = memo_key("RUN\0cook", &offered, "sha256-recipe", &environment).unwrap();
        let cook_after = memo_key("RUN\0cook", &offered, "sha256-recipe", &environment).unwrap();
        let build_before =
            memo_key("RUN\0build", &offered, "sha256-src-one", &environment).unwrap();
        let build_after = memo_key("RUN\0build", &offered, "sha256-src-two", &environment).unwrap();
        assert_eq!(cook_before, cook_after);
        assert_ne!(build_before, build_after);
    }

    #[test]
    fn fetch_pin_mismatch_is_loud_and_names_update_lock() {
        let error = verify_fetch_pin(
            Some(&FetchPin {
                nar_hash: "sha256-old".into(),
            }),
            "sha256-new",
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("hash mismatch"), "{error}");
        assert!(error.contains("--update-lock"), "{error}");
    }

    #[test]
    fn socket_filter_failure_adds_localhost_hint() {
        let error = sandbox_failure("exit status: 1", Some(RunNetwork::SocketFilter));
        assert!(error.contains("sandboxing was not weakened"), "{error}");
        assert!(error.contains("socket-filter fallback"), "{error}");
        assert!(
            error.contains("localhost networking (127.0.0.1) was unavailable"),
            "{error}"
        );
        assert_eq!(error.lines().count(), 2, "{error}");

        let preferred = sandbox_failure("exit status: 1", Some(RunNetwork::Namespace));
        assert!(!preferred.contains("localhost"), "{preferred}");
    }

    #[test]
    fn socket_filter_is_accepted_by_bubblewrap() {
        let shell = fs::read_dir("/nix/store")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path().join("bin/bash"))
            .find(|candidate| candidate.is_file())
            .expect("the Nix test host provides bash");
        let offer = shell
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let offered_closure = query_closure(&[offer]).unwrap();
        let work = tempfile::tempdir().unwrap();

        run_sandbox(
            work.path(),
            shell.to_str().unwrap(),
            "printf fallback-ok > result",
            &BTreeMap::new(),
            &offered_closure,
            Some(RunNetwork::SocketFilter),
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(work.path().join("result")).unwrap(),
            "fallback-ok"
        );
    }
}
