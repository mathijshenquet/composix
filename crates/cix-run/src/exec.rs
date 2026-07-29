use std::collections::BTreeMap;
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::runtime::current_uid;
use crate::shell::{resolve_shell, ShellSource};

pub struct ExecOptions {
    pub target: String,
    pub root: bool,
    pub user: bool,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    unit: String,
    service: String,
}

#[derive(Debug)]
struct UnitState {
    pid: u32,
    uid: u32,
    gid: u32,
    env: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ListedUnit {
    unit: String,
    active: String,
    sub: String,
    description: String,
}

pub fn exec(options: ExecOptions) -> Result<()> {
    if options.root && options.user {
        bail!("--root and --user cannot be combined");
    }
    if !options.user && current_uid()? != 0 {
        bail!(
            "cix exec must run as root to join a system service's namespaces; retry with sudo, or pass --user for the explicitly degraded no-join variant"
        );
    }

    let candidates = running_candidates(options.user)?;
    let unit = select_target(&options.target, &candidates)?;
    let mut state = inspect_unit(options.user, &unit)?;
    if let Ok(term) = std::env::var("TERM") {
        state.env.insert("TERM".into(), term);
    }

    let argv = if options.command.is_empty() {
        let shell = resolve_shell(&state.env)?;
        let source = match shell.source {
            ShellSource::ServicePath => "unit PATH",
            ShellSource::BinSh => "/bin/sh fallback",
        };
        eprintln!("cix exec: using shell {} ({source})", shell.path.display());
        vec![shell.path.to_string_lossy().into_owned()]
    } else {
        options.command
    };

    if options.user {
        eprintln!(
            "=== warning: cix exec --user is degraded; namespaces are not joined and the command runs as the caller ==="
        );
        return spawn_user_command(&argv, &state.env);
    }

    eprintln!(
        "=== warning: cix exec is operator surgery; joining {unit} namespaces without the service seccomp, capability, or cgroup confinement; identity={} ===",
        if options.root {
            "root (--root)"
        } else {
            "service runtime UID/GID"
        }
    );
    join_and_exec(
        state.pid,
        state.uid,
        state.gid,
        options.root,
        &argv,
        &state.env,
    )
}

fn running_candidates(user: bool) -> Result<Vec<Candidate>> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args([
            "list-units",
            "cix-*.service",
            "--all",
            "--output=json",
            "--no-pager",
            "--no-legend",
        ])
        .output()
        .context("failed to invoke systemctl")?;
    if !output.status.success() {
        bail!(
            "failed to list running cix services: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let listed: Vec<ListedUnit> =
        serde_json::from_slice(&output.stdout).context("systemctl emitted invalid JSON")?;
    Ok(listed
        .into_iter()
        .filter(|unit| unit.active == "active" && unit.sub == "running")
        .filter_map(|unit| {
            let service = unit
                .description
                .strip_prefix("cix run: ")
                .map(str::to_owned)
                .or_else(|| service_from_transient_name(&unit.unit));
            service.map(|service| Candidate {
                unit: unit.unit,
                service,
            })
        })
        .collect())
}

fn service_from_transient_name(unit: &str) -> Option<String> {
    let stem = unit.strip_suffix(".service")?;
    for prefix in ["cix-run-", "cix-debug-"] {
        if let Some(rest) = stem.strip_prefix(prefix) {
            return rest.rsplit_once('-').map(|(service, _)| service.to_owned());
        }
    }
    None
}

fn select_target(target: &str, candidates: &[Candidate]) -> Result<String> {
    if let Some(candidate) = candidates.iter().find(|candidate| candidate.unit == target) {
        return Ok(candidate.unit.clone());
    }
    let matching = candidates
        .iter()
        .filter(|candidate| candidate.service == target)
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [candidate] => Ok(candidate.unit.clone()),
        [] => bail!(
            "no running cix service matches {target:?}; use the exact unit from `cix ps`, or use `cix debug` when the service is not running"
        ),
        _ => {
            let units = matching
                .iter()
                .map(|candidate| candidate.unit.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            bail!("service name {target:?} is ambiguous; matching units: {units}")
        }
    }
}

fn inspect_unit(user: bool, unit: &str) -> Result<UnitState> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .arg("show")
        .arg(unit)
        .args([
            "--property=MainPID",
            "--property=Environment",
            "--property=UID",
            "--property=GID",
            "--no-pager",
        ])
        .output()
        .with_context(|| format!("failed to inspect {unit}"))?;
    if !output.status.success() {
        bail!(
            "failed to inspect {unit}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let properties =
        String::from_utf8(output.stdout).context("systemctl emitted non-UTF-8 data")?;
    let mut values = BTreeMap::new();
    for line in properties.lines() {
        if let Some((name, value)) = line.split_once('=') {
            values.insert(name, value);
        }
    }
    let pid = parse_numeric_property(&values, "MainPID", unit)?;
    if pid == 0 {
        bail!("unit {unit} has no running main process; use `cix debug` for a stopped service");
    }
    Ok(UnitState {
        pid,
        uid: if user {
            0
        } else {
            parse_numeric_property(&values, "UID", unit)?
        },
        gid: if user {
            0
        } else {
            parse_numeric_property(&values, "GID", unit)?
        },
        env: parse_environment(values.get("Environment").copied().unwrap_or_default())?,
    })
}

fn parse_numeric_property(
    values: &BTreeMap<&str, &str>,
    property: &str,
    unit: &str,
) -> Result<u32> {
    values
        .get(property)
        .with_context(|| format!("systemctl did not report {property} for {unit}"))?
        .parse()
        .with_context(|| format!("systemctl reported an invalid {property} for {unit}"))
}

fn parse_environment(input: &str) -> Result<BTreeMap<String, String>> {
    let words = parse_systemd_words(input)?;
    let mut env = BTreeMap::new();
    for word in words {
        let (name, value) = word
            .split_once('=')
            .with_context(|| format!("invalid Environment entry {word:?} from systemd"))?;
        if name.is_empty() || name.contains('\0') || value.contains('\0') {
            bail!("invalid Environment entry {word:?} from systemd");
        }
        env.insert(name.to_owned(), value.to_owned());
    }
    Ok(env)
}

fn parse_systemd_words(input: &str) -> Result<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut in_word = false;
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        match (quote, character) {
            (None, character) if character.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            (None, '"' | '\'') => {
                quote = Some(character);
                in_word = true;
            }
            (Some(delimiter), character) if character == delimiter => {
                quote = None;
                in_word = true;
            }
            (_, '\\') => {
                word.push(parse_escape(&mut chars)?);
                in_word = true;
            }
            _ => {
                word.push(character);
                in_word = true;
            }
        }
    }
    if quote.is_some() {
        bail!("unterminated quote in Environment value from systemd");
    }
    if in_word {
        words.push(word);
    }
    Ok(words)
}

fn parse_escape<I>(chars: &mut std::iter::Peekable<I>) -> Result<char>
where
    I: Iterator<Item = char>,
{
    let escaped = chars
        .next()
        .context("trailing backslash in Environment value from systemd")?;
    Ok(match escaped {
        'a' => '\u{7}',
        'b' => '\u{8}',
        'f' => '\u{c}',
        'n' => '\n',
        'r' => '\r',
        's' => ' ',
        't' => '\t',
        'v' => '\u{b}',
        other => other,
    })
}

fn spawn_user_command(argv: &[String], env: &BTreeMap<String, String>) -> Result<()> {
    let program = resolve_program(&argv[0], env)?;
    let status = Command::new(program)
        .args(&argv[1..])
        .env_clear()
        .envs(env)
        .status()
        .context("failed to run degraded exec command")?;
    if status.success() {
        return Ok(());
    }
    process::exit(status.code().unwrap_or(1));
}

fn join_and_exec(
    pid: u32,
    uid: u32,
    gid: u32,
    keep_root: bool,
    argv: &[String],
    env: &BTreeMap<String, String>,
) -> Result<()> {
    let namespaces = open_namespaces(pid)?;
    for namespace in &namespaces {
        let result = unsafe { libc::setns(namespace.file.as_raw_fd(), namespace.kind) };
        if result != 0 {
            return Err(std::io::Error::last_os_error()).with_context(|| {
                format!("failed to join {} namespace of PID {pid}", namespace.name)
            });
        }
    }

    let program = resolve_program(&argv[0], env)?;
    let program = c_string(program.as_os_str().as_encoded_bytes(), "command path")?;
    let arguments = argv
        .iter()
        .map(|argument| c_string(argument.as_bytes(), "command argument"))
        .collect::<Result<Vec<_>>>()?;
    let environment = env
        .iter()
        .map(|(name, value)| c_string(format!("{name}={value}").as_bytes(), "environment value"))
        .collect::<Result<Vec<_>>>()?;
    let argument_pointers = arguments
        .iter()
        .map(|value| value.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect::<Vec<_>>();
    let environment_pointers = environment
        .iter()
        .map(|value| value.as_ptr())
        .chain(std::iter::once(std::ptr::null()))
        .collect::<Vec<_>>();

    let child = unsafe { libc::fork() };
    if child < 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to fork after joining PID namespace");
    }
    if child == 0 {
        if !keep_root {
            let identity_result = unsafe {
                libc::setgroups(0, std::ptr::null()) != 0
                    || libc::setgid(gid) != 0
                    || libc::setuid(uid) != 0
            };
            if identity_result {
                child_error("cix exec: failed to adopt service UID/GID\n", 126);
            }
        }
        unsafe {
            libc::execve(
                program.as_ptr(),
                argument_pointers.as_ptr(),
                environment_pointers.as_ptr(),
            );
        }
        let code = if std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            127
        } else {
            126
        };
        child_error("cix exec: failed to execute command\n", code);
    }

    let mut status = 0;
    loop {
        let waited = unsafe { libc::waitpid(child, &mut status, 0) };
        if waited == child {
            break;
        }
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() != Some(libc::EINTR) {
            return Err(error).context("failed to wait for exec command");
        }
    }
    if libc::WIFEXITED(status) {
        let code = libc::WEXITSTATUS(status);
        if code == 0 {
            return Ok(());
        }
        process::exit(code);
    }
    if libc::WIFSIGNALED(status) {
        process::exit(128 + libc::WTERMSIG(status));
    }
    bail!("exec command ended in an unexpected process state")
}

struct Namespace {
    name: &'static str,
    kind: libc::c_int,
    file: File,
}

fn open_namespaces(pid: u32) -> Result<Vec<Namespace>> {
    [
        ("mount", "mnt", libc::CLONE_NEWNS),
        ("network", "net", libc::CLONE_NEWNET),
        ("IPC", "ipc", libc::CLONE_NEWIPC),
        ("UTS", "uts", libc::CLONE_NEWUTS),
        ("PID", "pid", libc::CLONE_NEWPID),
    ]
    .into_iter()
    .map(|(name, entry, kind)| {
        let path = format!("/proc/{pid}/ns/{entry}");
        let file = File::open(&path)
            .with_context(|| format!("failed to open {name} namespace at {path}"))?;
        Ok(Namespace { name, kind, file })
    })
    .collect()
}

fn resolve_program(program: &str, env: &BTreeMap<String, String>) -> Result<PathBuf> {
    let path = Path::new(program);
    if program.contains('/') {
        if is_executable(path) {
            return Ok(path.to_owned());
        }
        bail!("command {program:?} is not an executable file");
    }
    if let Some(path) = env.get("PATH") {
        for directory in std::env::split_paths(path) {
            let candidate = directory.join(program);
            if is_executable(&candidate) {
                return Ok(candidate);
            }
        }
    }
    bail!("command {program:?} was not found on the unit's recorded PATH")
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

fn c_string(bytes: &[u8], description: &str) -> Result<CString> {
    CString::new(bytes).with_context(|| format!("{description} contains a NUL byte"))
}

fn child_error(message: &'static str, code: i32) -> ! {
    unsafe {
        libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
        libc::_exit(code);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(unit: &str, service: &str) -> Candidate {
        Candidate {
            unit: unit.into(),
            service: service.into(),
        }
    }

    #[test]
    fn target_selection_supports_exact_unique_ambiguous_and_none() {
        let candidates = vec![
            candidate("cix-run-web-a.service", "web"),
            candidate("cix-stack-api.service", "api"),
        ];
        assert_eq!(
            select_target("cix-run-web-a.service", &candidates).unwrap(),
            "cix-run-web-a.service"
        );
        assert_eq!(
            select_target("api", &candidates).unwrap(),
            "cix-stack-api.service"
        );

        let ambiguous = vec![
            candidate("cix-run-web-a.service", "web"),
            candidate("cix-run-web-b.service", "web"),
        ];
        let error = select_target("web", &ambiguous).unwrap_err().to_string();
        assert!(error.contains("ambiguous"));
        assert!(error.contains("cix-run-web-a.service"));
        assert!(error.contains("cix-run-web-b.service"));

        let error = select_target("missing", &candidates)
            .unwrap_err()
            .to_string();
        assert!(error.contains("no running cix service"));
        assert!(error.contains("cix debug"));
    }

    #[test]
    fn parses_systemd_quoted_environment_values() {
        let env = parse_environment(
            r#"PLAIN=value "SPACED=a b" "QUOTED=a\"b" 'SINGLE=x y' "BACKSLASH=a\\b" EMPTY="#,
        )
        .unwrap();
        assert_eq!(env["PLAIN"], "value");
        assert_eq!(env["SPACED"], "a b");
        assert_eq!(env["QUOTED"], "a\"b");
        assert_eq!(env["SINGLE"], "x y");
        assert_eq!(env["BACKSLASH"], r"a\b");
        assert_eq!(env["EMPTY"], "");
    }

    #[test]
    fn derives_service_names_from_transient_units() {
        assert_eq!(
            service_from_transient_name("cix-run-my-service-deadbeef.service").as_deref(),
            Some("my-service")
        );
        assert_eq!(
            service_from_transient_name("cix-debug-web-cafe.service").as_deref(),
            Some("web")
        );
        assert_eq!(service_from_transient_name("cix-stack-web.service"), None);
    }
}
