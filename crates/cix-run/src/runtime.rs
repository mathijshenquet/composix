use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::config::ResolvedConfig;
use crate::spec::{Service, Spec};
use crate::unit::{build_unit, UnitDefinition, UnitMode};

static NONCE_COUNTER: AtomicU64 = AtomicU64::new(0);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

const SIGINT: i32 = 2;
const SIG_ERR: usize = usize::MAX;

unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
}

pub struct RunOptions {
    pub installable: String,
    pub env: Vec<String>,
    pub port: Vec<String>,
    pub detach: bool,
    pub user: bool,
}

#[derive(Debug)]
struct Target {
    output: PathBuf,
    requested_service: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StartedUnit {
    pub name: String,
    pub user: bool,
    pub degraded: bool,
}

pub fn run(options: RunOptions) -> Result<()> {
    if !options.user && current_uid()? != 0 {
        bail!(
            "cix run targets the system manager and must run as root; use sudo, or pass --user for explicitly degraded dev mode"
        );
    }
    if options.user {
        eprintln!(
            "warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path"
        );
    }

    let target = resolve_target(&options.installable)?;
    let spec = Spec::load(&target.output)?;
    let selected = spec.select_service(target.requested_service.as_deref());
    let (service_name, service) = match selected {
        Ok(selected) => selected,
        Err(original_error) if target.requested_service.is_none() => {
            if let Some((installable, service_name)) = split_single_hash(&options.installable) {
                let output = resolve_installable(installable)?;
                let fallback_spec = Spec::load(&output)?;
                let (selected_name, selected_service) =
                    fallback_spec.select_service(Some(service_name))?;
                return run_resolved(output, selected_name, selected_service, &options);
            }
            return Err(original_error);
        }
        Err(error) => return Err(error),
    };
    run_resolved(target.output, service_name, service, &options)
}

fn run_resolved(
    output: PathBuf,
    service_name: &str,
    service: &Service,
    options: &RunOptions,
) -> Result<()> {
    let config = ResolvedConfig::resolve(service, &options.env, &options.port)?;
    let started = start_service(&output, service_name, service, &config, options.user)?;
    if options.detach {
        println!("{}", started.name);
        return Ok(());
    }
    follow(&started)
}

pub fn start_service(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    user: bool,
) -> Result<StartedUnit> {
    let name = format!("cix-run-{service_name}-{}.service", nonce());
    if !user {
        let definition = build_unit(output, service_name, service, config, UnitMode::System)?;
        start_once(&name, false, &definition)?;
        return Ok(StartedUnit {
            name,
            user: false,
            degraded: false,
        });
    }

    let definition = build_unit(output, service_name, service, config, UnitMode::UserFull)?;
    match start_once(&name, true, &definition) {
        Ok(()) => Ok(StartedUnit {
            name,
            user: true,
            degraded: false,
        }),
        Err(full_error) => {
            let full_error = with_unit_diagnostics(full_error, &name, true);
            let _ = stop_service(&name, true);

            if capability_failure(&full_error) {
                let capability_name = format!("cix-run-{service_name}-{}.service", nonce());
                eprintln!("warning: user manager rejected capability controls ({full_error:#})");
                eprintln!(
                    "warning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, and ProtectKernelLogs"
                );
                let without_capabilities = without_properties(
                    &definition,
                    &[
                        "AmbientCapabilities",
                        "CapabilityBoundingSet",
                        "ProtectKernelModules",
                        "ProtectKernelLogs",
                    ],
                );
                match start_once(&capability_name, true, &without_capabilities) {
                    Ok(()) => {
                        return Ok(StartedUnit {
                            name: capability_name,
                            user: true,
                            degraded: true,
                        });
                    }
                    Err(error) => {
                        let error = with_unit_diagnostics(error, &capability_name, true);
                        let _ = stop_service(&capability_name, true);
                        return namespace_fallback(output, service_name, service, config, error);
                    }
                }
            }

            namespace_fallback(output, service_name, service, config, full_error)
        }
    }
}

fn namespace_fallback(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    error: anyhow::Error,
) -> Result<StartedUnit> {
    if !namespace_failure(&error) {
        return Err(error);
    }
    let fallback_name = format!("cix-run-{service_name}-{}.service", nonce());
    eprintln!("warning: the user manager rejected mount-namespace sandboxing ({error:#})");
    eprintln!(
        "warning: retrying without PrivateUsers, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; managed *Directory persistence remains, but declared app paths will not be remapped"
    );
    let degraded = build_unit(
        output,
        service_name,
        service,
        config,
        UnitMode::UserDegraded,
    )?;
    start_once(&fallback_name, true, &degraded)?;
    Ok(StartedUnit {
        name: fallback_name,
        user: true,
        degraded: true,
    })
}

fn without_properties(definition: &UnitDefinition, names: &[&str]) -> UnitDefinition {
    let mut definition = definition.clone();
    definition
        .properties
        .retain(|(name, _)| !names.contains(&name.as_str()));
    definition.text.clear();
    definition
}

pub fn stop_service(name: &str, user: bool) -> Result<()> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args(["stop", name])
        .output()
        .with_context(|| format!("failed to invoke systemctl to stop {name}"))?;
    if !output.status.success() {
        bail!(
            "failed to stop {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn start_once(name: &str, user: bool, definition: &UnitDefinition) -> Result<()> {
    let mut command = Command::new("systemd-run");
    if user {
        command.arg("--user");
    }
    command
        .arg("--collect")
        .arg("--service-type=exec")
        .arg(format!("--unit={name}"));

    for (property, value) in &definition.properties {
        let value = if user && property == "BindPaths" {
            expand_user_directory_specifiers(value)?
        } else {
            value.clone()
        };
        match property.as_str() {
            "Type" => {}
            "Slice" => {
                command.arg(format!("--slice={value}"));
            }
            _ => {
                command.arg(format!("--property={property}={value}"));
            }
        }
    }
    for (name, value) in &definition.environment {
        command.arg(format!("--setenv={name}={value}"));
    }
    command.arg("--").args(&definition.argv);

    let output = command
        .output()
        .with_context(|| format!("failed to invoke systemd-run for {name}"))?;
    if !output.status.success() {
        let message = command_error(&output);
        bail!("systemd-run failed to start {name}: {message}");
    }
    Ok(())
}

fn expand_user_directory_specifiers(value: &str) -> Result<String> {
    let home = std::env::var("HOME").context("HOME is not set for the user manager")?;
    let state = std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{home}/.local/state"));
    let cache = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"));
    let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
    let logs = format!("{state}/log");
    Ok(value
        .replace("%S", &state)
        .replace("%C", &cache)
        .replace("%L", &logs)
        .replace("%E", &config))
}

fn follow(unit: &StartedUnit) -> Result<()> {
    let mut journal = journal_child(&unit.name, unit.user)?;
    INTERRUPTED.store(false, Ordering::Relaxed);
    let previous = unsafe { signal(SIGINT, handle_interrupt as *const () as usize) };
    if previous == SIG_ERR {
        terminate_child(&mut journal);
        bail!("failed to install the Ctrl-C handler");
    }

    loop {
        if INTERRUPTED.swap(false, Ordering::Relaxed) {
            let stop_result = stop_service(&unit.name, unit.user);
            terminate_child(&mut journal);
            stop_result?;
            return Ok(());
        }
        if !unit_is_running(&unit.name, unit.user)? {
            terminate_child(&mut journal);
            return unit_exit_result(&unit.name, unit.user);
        }
        thread::sleep(Duration::from_millis(250));
    }
}

extern "C" fn handle_interrupt(_signal: i32) {
    INTERRUPTED.store(true, Ordering::Relaxed);
}

fn journal_child(name: &str, user: bool) -> Result<Child> {
    let mut command = Command::new("journalctl");
    if user {
        command.arg(format!("--user-unit={name}"));
    } else {
        command.arg("--unit").arg(name);
    }
    command
        .args(["--follow", "--output=cat", "--no-hostname"])
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("failed to follow the journal for {name}"))
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn unit_is_running(name: &str, user: bool) -> Result<bool> {
    let state = systemctl_value(user, name, "ActiveState")?;
    Ok(matches!(
        state.as_str(),
        "active" | "activating" | "reloading"
    ))
}

fn unit_exit_result(name: &str, user: bool) -> Result<()> {
    let result = systemctl_value(user, name, "Result").unwrap_or_else(|_| "unknown".into());
    if matches!(result.as_str(), "success" | "unknown") {
        Ok(())
    } else {
        bail!("unit {name} stopped with result {result}");
    }
}

fn systemctl_value(user: bool, name: &str, property: &str) -> Result<String> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args(["show", name, "--property", property, "--value"])
        .output()
        .with_context(|| format!("failed to query {name}"))?;
    if !output.status.success() {
        bail!(
            "failed to query {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn resolve_target(input: &str) -> Result<Target> {
    if input.starts_with("/nix/store/") || input.matches('#').count() >= 2 {
        if let Some((installable, service)) = input.rsplit_once('#') {
            return Ok(Target {
                output: resolve_installable(installable)?,
                requested_service: Some(service.to_owned()),
            });
        }
    }
    Ok(Target {
        output: resolve_installable(input)?,
        requested_service: None,
    })
}

fn split_single_hash(input: &str) -> Option<(&str, &str)> {
    if input.matches('#').count() == 1 {
        input.rsplit_once('#')
    } else {
        None
    }
}

pub fn resolve_installable(installable: &str) -> Result<PathBuf> {
    if installable.is_empty() {
        bail!("installable must not be empty");
    }
    let direct_path = PathBuf::from(installable);
    if direct_path.starts_with("/nix/store/") && direct_path.exists() {
        return Ok(direct_path);
    }

    let output = nix_build(installable)?;
    if !output.status.success() {
        bail!(
            "failed to resolve installable {installable:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let paths = String::from_utf8(output.stdout).context("nix emitted a non-UTF-8 store path")?;
    let paths = paths
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if paths.len() != 1 {
        bail!(
            "installable {installable:?} resolved to {} outputs; cix run needs exactly one",
            paths.len()
        );
    }
    let path = PathBuf::from(paths[0]);
    if !path.starts_with("/nix/store/") {
        bail!(
            "installable {installable:?} resolved outside the Nix store: {}",
            path.display()
        );
    }
    Ok(path)
}

fn nix_build(installable: &str) -> Result<Output> {
    let invoke = |program: &Path| {
        Command::new(program)
            .args(["build", "--no-link", "--print-out-paths", "--"])
            .arg(installable)
            .output()
    };

    match invoke(Path::new("nix")) {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            invoke(Path::new("/nix/var/nix/profiles/default/bin/nix"))
                .with_context(|| format!("failed to invoke nix for installable {installable:?}"))
        }
        Err(error) => Err(error)
            .with_context(|| format!("failed to invoke nix for installable {installable:?}")),
    }
}

fn current_uid() -> Result<u32> {
    let output = Command::new("id")
        .arg("-u")
        .output()
        .context("failed to determine the current user id")?;
    if !output.status.success() {
        bail!("id -u failed");
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .context("id -u returned an invalid user id")
}

fn nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{counter:x}")
}

fn namespace_failure(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    [
        "namespace",
        "privateusers",
        "bindpaths",
        "invalid argument",
        "unknown assignment",
        "not supported",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn capability_failure(error: &anyhow::Error) -> bool {
    format!("{error:#}")
        .to_ascii_lowercase()
        .contains("capabilit")
}

fn with_unit_diagnostics(error: anyhow::Error, name: &str, user: bool) -> anyhow::Error {
    match unit_diagnostics(name, user) {
        Ok(diagnostics) if !diagnostics.is_empty() => {
            error.context(format!("unit diagnostics:\n{diagnostics}"))
        }
        _ => error,
    }
}

fn unit_diagnostics(name: &str, user: bool) -> Result<String> {
    let mut command = Command::new("journalctl");
    if user {
        command.arg(format!("--user-unit={name}"));
    } else {
        command.arg("--unit").arg(name);
    }
    let output = command
        .args(["--no-pager", "--lines=20", "--output=cat"])
        .output()
        .with_context(|| format!("failed to read diagnostics for {name}"))?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn command_error(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let message = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    if message.is_empty() {
        format!("exit status {}", output.status)
    } else {
        message.to_owned()
    }
}

#[derive(Debug, Deserialize)]
struct ListedUnit {
    unit: String,
    active: String,
    sub: String,
    description: String,
}

pub fn ps() -> Result<()> {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    for (manager, user) in [("system", false), ("user", true)] {
        match list_units(user) {
            Ok(units) => rows.extend(units.into_iter().map(|unit| (manager, unit))),
            Err(error) => errors.push(format!("{manager}: {error:#}")),
        }
    }
    if rows.is_empty() && !errors.is_empty() {
        bail!("could not query systemd managers: {}", errors.join("; "));
    }
    for error in errors {
        eprintln!("warning: could not list {error}");
    }
    rows.sort_by(|left, right| {
        left.0
            .cmp(right.0)
            .then_with(|| left.1.unit.cmp(&right.1.unit))
    });

    let manager_width = rows
        .iter()
        .map(|(manager, _)| manager.len())
        .max()
        .unwrap_or(7)
        .max(7);
    let unit_width = rows
        .iter()
        .map(|(_, unit)| unit.unit.len())
        .max()
        .unwrap_or(4)
        .max(4);
    println!(
        "{:<manager_width$}  {:<unit_width$}  {:<10}  DESCRIPTION",
        "MANAGER", "UNIT", "STATE"
    );
    for (manager, unit) in rows {
        println!(
            "{manager:<manager_width$}  {:<unit_width$}  {:<10}  {}",
            unit.unit,
            format!("{}/{}", unit.active, unit.sub),
            unit.description
        );
    }
    Ok(())
}

fn list_units(user: bool) -> Result<Vec<ListedUnit>> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .args([
            "list-units",
            "cix-*",
            "--all",
            "--output=json",
            "--no-pager",
            "--no-legend",
        ])
        .output()
        .context("failed to invoke systemctl")?;
    if !output.status.success() {
        bail!("{}", command_error(&output));
    }
    serde_json::from_slice(&output.stdout).context("systemctl emitted invalid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_store_and_double_hash_service_selectors() {
        assert_eq!(split_single_hash(".#package"), Some((".", "package")));
        assert!(split_single_hash("github:owner/repo#package#web").is_none());
    }

    #[test]
    fn identifies_namespace_failures_only() {
        assert!(namespace_failure(&anyhow::anyhow!(
            "Failed at step NAMESPACE: Operation not permitted"
        )));
        assert!(!namespace_failure(&anyhow::anyhow!(
            "executable was not found"
        )));
        assert!(capability_failure(&anyhow::anyhow!(
            "Failed at step CAPABILITIES"
        )));
    }

    #[test]
    fn resolves_an_existing_store_path_without_building_it() {
        let store_path = std::fs::read_dir("/nix/store")
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.is_dir())
            .unwrap();
        assert_eq!(
            resolve_installable(store_path.to_str().unwrap()).unwrap(),
            store_path
        );
    }
}
