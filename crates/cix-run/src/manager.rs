use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

use crate::capabilities::HostCapabilities;
use crate::closed_root::{options_for_unit, prepare};
use crate::config::ResolvedConfig;
use crate::runtime::RunOptions;
use crate::spec::Service;
use crate::unit::{
    build_unit, build_unit_with_options, UnitCompileOptions, UnitDefinition, UnitDegradation,
    UnitMode,
};

const SIGINT: i32 = 2;
const SIG_ERR: usize = usize::MAX;

unsafe extern "C" {
    fn signal(signal: i32, handler: usize) -> usize;
}

#[derive(Debug, Clone)]
pub struct StartedUnit {
    pub name: String,
    pub user: bool,
    pub degraded: bool,
}

pub(crate) struct ForegroundResult {
    pub status: ExitStatus,
    pub stderr: String,
}

pub(crate) fn start_scheduled_app(
    output: &Path,
    app_name: &str,
    user: bool,
    schedule: &str,
    definition: &UnitDefinition,
) -> Result<()> {
    let stem = format!("cix-run-{app_name}-{}", nonce());
    let service = format!("{stem}.service");
    let timer = format!("{stem}.timer");
    let root_service = format!("{stem}-root.service");
    let root = gc_root_link(&timer, user)?;
    register_gc_root(&root, output)?;

    let directory = socket_unit_directory(user)?;
    fs::create_dir_all(&directory).with_context(|| {
        format!(
            "creating runtime unit directory {} for scheduled app",
            directory.display()
        )
    })?;
    let cleanup = gc_root_cleanup_command(&root, user, false)?;
    let root_text = format!(
        "[Unit]\nDescription=cix scheduled app GC root: {app_name}\nPartOf={timer}\nBefore={timer}\n\n[Service]\nType=oneshot\nRemainAfterExit=yes\nExecStart=/bin/sh -c true\nExecStopPost={cleanup}\n"
    );
    let timer_text = format!(
        "[Unit]\nDescription=cix scheduled app: {app_name}\nRequires={root_service}\nAfter={root_service}\n\n[Timer]\nOnCalendar={schedule}\nUnit={service}\n"
    );
    let definition = with_log_unit_name(definition, &service);
    let service_text = definition.text.replacen(
        "\n\n[Service]\n",
        &format!("\nPartOf={timer}\n\n[Service]\n"),
        1,
    );
    let paths = [
        (directory.join(&service), service_text),
        (directory.join(&timer), timer_text),
        (directory.join(&root_service), root_text),
    ];
    for (path, text) in &paths {
        if let Err(error) = fs::write(path, text)
            .with_context(|| format!("writing scheduled app unit {}", path.display()))
        {
            remove_gc_root(Some(&root));
            return Err(error);
        }
    }
    if let Err(error) = daemon_reload(user).and_then(|()| systemctl_action(user, "start", &timer)) {
        let _ = systemctl_action(user, "stop", &timer);
        for (path, _) in &paths {
            let _ = fs::remove_file(path);
        }
        let _ = daemon_reload(user);
        remove_gc_root(Some(&root));
        return Err(error);
    }
    println!("{timer}");
    Ok(())
}

pub(crate) fn run_resolved(
    output: PathBuf,
    service_name: &str,
    service: &Service,
    options: &RunOptions,
    unit_options: UnitCompileOptions,
) -> Result<()> {
    let config = ResolvedConfig::resolve(service, &options.env, &options.port)?;
    let started = start_service_with_options(
        &output,
        service_name,
        service,
        &config,
        options.user,
        &unit_options,
        options.closed_root,
    )?;
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
    start_service_with_options(
        output,
        service_name,
        service,
        config,
        user,
        &UnitCompileOptions::cix_run(service_name),
        false,
    )
}

fn start_service_with_options(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    user: bool,
    unit_options: &UnitCompileOptions,
    closed_root: bool,
) -> Result<StartedUnit> {
    let name = format!("cix-run-{service_name}-{}.service", nonce());
    let mut unit_options = unit_options.clone();
    if closed_root {
        let root = options_for_unit(&name, user)?;
        prepare(&root)?;
        unit_options.closed_root = Some(root);
    }
    if !user {
        let capabilities = HostCapabilities::probe()?;
        let definition = build_unit_with_options(
            output,
            service_name,
            service,
            config,
            UnitMode::System,
            &unit_options,
            &capabilities,
        )?;
        warn_degradations(&definition.degradations);
        return match start_with_listeners(&name, false, output, config, &definition) {
            Ok(()) => Ok(StartedUnit {
                name,
                user: false,
                degraded: !definition.degradations.is_empty(),
            }),
            Err(error) => {
                let error = with_unit_diagnostics(error, &name, false);
                let _ = stop_service(&name, false);
                private_pids_fallback(output, service_name, config, &definition, error)
            }
        };
    }

    let capabilities = user_capabilities(service)?;
    let definition = build_unit_with_options(
        output,
        service_name,
        service,
        config,
        UnitMode::UserFull,
        &unit_options,
        &capabilities,
    )?;
    warn_degradations(&definition.degradations);
    match start_with_listeners(&name, true, output, config, &definition) {
        Ok(()) => Ok(StartedUnit {
            name,
            user: true,
            degraded: !definition.degradations.is_empty(),
        }),
        Err(full_error) => {
            let full_error = with_unit_diagnostics(full_error, &name, true);
            let _ = stop_service(&name, true);

            if capability_failure(&full_error) {
                let capability_name = format!("cix-run-{service_name}-{}.service", nonce());
                eprintln!("warning: user manager rejected capability controls ({full_error:#})");
                eprintln!(
                    "warning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, ProtectKernelLogs, and PrivateDevices"
                );
                let without_capabilities = without_user_capability_controls(&definition);
                match start_with_listeners(
                    &capability_name,
                    true,
                    output,
                    config,
                    &without_capabilities,
                ) {
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
        "warning: retrying without PrivateUsers, PrivatePIDs, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; managed *Directory persistence remains, but declared app paths will not be remapped"
    );
    let degraded = build_unit(
        output,
        service_name,
        service,
        config,
        UnitMode::UserDegraded,
    )?;
    start_with_listeners(&fallback_name, true, output, config, &degraded)?;
    Ok(StartedUnit {
        name: fallback_name,
        user: true,
        degraded: true,
    })
}

fn private_pids_fallback(
    output: &Path,
    service_name: &str,
    config: &ResolvedConfig,
    definition: &UnitDefinition,
    error: anyhow::Error,
) -> Result<StartedUnit> {
    if !namespace_failure(&error) {
        return Err(error);
    }
    let fallback_name = format!("cix-run-{service_name}-{}.service", nonce());
    eprintln!("warning: the system manager rejected PrivatePIDs isolation ({error:#})");
    eprintln!(
        "warning: retrying without PrivatePIDs; this service shares the host PID namespace (D36 degraded fallback)"
    );
    let fallback = without_properties(definition, &["PrivatePIDs"]);
    start_with_listeners(&fallback_name, false, output, config, &fallback)?;
    Ok(StartedUnit {
        name: fallback_name,
        user: false,
        degraded: true,
    })
}

pub(crate) fn without_properties(definition: &UnitDefinition, names: &[&str]) -> UnitDefinition {
    let mut definition = definition.clone();
    definition
        .properties
        .retain(|(name, _)| !names.contains(&name.as_str()));
    definition.text = definition
        .text
        .lines()
        .filter(|line| {
            line.split_once('=')
                .is_none_or(|(name, _)| !names.contains(&name))
        })
        .collect::<Vec<_>>()
        .join("\n");
    definition.text.push('\n');
    definition
}

pub(crate) fn build_runtime_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
    closed_root: bool,
    unit_name: &str,
) -> Result<UnitDefinition> {
    let capabilities = if mode == UnitMode::UserFull {
        user_capabilities(service)?
    } else {
        HostCapabilities::all_supported()
    };
    let mut options = UnitCompileOptions::cix_run(service_name);
    if closed_root {
        let root = options_for_unit(unit_name, mode != UnitMode::System)?;
        prepare(&root)?;
        options.closed_root = Some(root);
    }
    build_unit_with_options(
        output,
        service_name,
        service,
        config,
        mode,
        &options,
        &capabilities,
    )
}

fn user_capabilities(service: &Service) -> Result<HostCapabilities> {
    if service.has_device_claim() {
        Ok(HostCapabilities::all_supported())
    } else {
        HostCapabilities::probe_user()
    }
}

pub(crate) fn warn_degradations(degradations: &[UnitDegradation]) {
    for degradation in degradations {
        match degradation.property.as_str() {
            "PrivatePIDs=yes" => eprintln!(
                "warning: dropped {}: {}; this service shares the host PID namespace (D36 degraded fallback)",
                degradation.property, degradation.reason
            ),
            "PrivateDevices=yes" => {
                eprintln!(
                    "warning: user manager rejected PrivateDevices isolation ({})",
                    degradation.reason
                );
                eprintln!(
                    "warning: retrying without PrivateDevices; this --user service can access the host device namespace (D13 degraded fallback)"
                );
            }
            property => eprintln!("warning: dropped {property}: {}", degradation.reason),
        }
    }
}

pub(crate) fn without_user_capability_controls(definition: &UnitDefinition) -> UnitDefinition {
    without_properties(
        definition,
        &[
            "AmbientCapabilities",
            "CapabilityBoundingSet",
            "ProtectKernelModules",
            "ProtectKernelLogs",
            "PrivateDevices",
        ],
    )
}

pub fn stop_service(name: &str, user: bool) -> Result<()> {
    let sockets = listener_sockets(name, user).unwrap_or_default();
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    let output = command
        .arg("stop")
        .args(&sockets)
        .arg(name)
        .output()
        .with_context(|| format!("failed to invoke systemctl to stop {name}"))?;
    for socket in &sockets {
        let _ = remove_socket_unit(socket, user);
    }
    let _ = remove_service_unit(name, user);
    if !output.status.success() {
        bail!(
            "failed to stop {name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

fn start_with_listeners(
    name: &str,
    user: bool,
    output: &Path,
    config: &ResolvedConfig,
    definition: &UnitDefinition,
) -> Result<()> {
    let definition = with_log_unit_name(definition, name);
    let (definition, gc_root) = definition_with_gc_root(name, user, output, &definition)?;
    if config.listeners.is_empty() {
        let result = start_once(name, user, &definition);
        if result.is_err() {
            remove_gc_root(gc_root.as_ref());
        }
        return result;
    }
    let sockets = listener_socket_names(name, config);
    let definition = with_listener_dependencies(&definition, &sockets);
    if let Err(error) = write_service_unit(name, user, &definition) {
        remove_gc_root(gc_root.as_ref());
        return Err(error);
    }
    match create_listener_sockets(name, user, config).and_then(|created| {
        debug_assert_eq!(created, sockets);
        systemctl_action(user, "start", name)
    }) {
        Ok(()) => Ok(()),
        Err(error) => {
            for socket in &sockets {
                let _ = remove_socket_unit(socket, user);
            }
            let _ = remove_service_unit(name, user);
            remove_gc_root(gc_root.as_ref());
            Err(error)
        }
    }
}

fn definition_with_gc_root(
    name: &str,
    user: bool,
    output: &Path,
    definition: &UnitDefinition,
) -> Result<(UnitDefinition, Option<PathBuf>)> {
    let link = match gc_root_link(name, user) {
        Ok(link) => link,
        Err(error) if user => {
            eprintln!(
                "warning: could not create the user GC-root directory; this run is not GC-protected ({error:#})"
            );
            return Ok((definition.clone(), None));
        }
        Err(error) => return Err(error),
    };
    let closed_root = definition
        .properties
        .iter()
        .any(|(name, _)| name == "RootDirectory");
    let cleanup = match gc_root_cleanup_command(&link, user, closed_root) {
        Ok(cleanup) => cleanup,
        Err(error) if user => {
            eprintln!(
                "warning: could not prepare user GC-root cleanup; this run is not GC-protected ({error:#})"
            );
            return Ok((definition.clone(), None));
        }
        Err(error) => return Err(error),
    };
    match register_gc_root(&link, output) {
        Ok(()) => {
            let mut definition = definition.clone();
            definition
                .properties
                .push(("ExecStopPost".into(), cleanup.clone()));
            definition.text = definition.text.replacen(
                "ExecStart=",
                &format!("ExecStopPost={cleanup}\nExecStart="),
                1,
            );
            Ok((definition, Some(link)))
        }
        Err(error) if user => {
            eprintln!(
                "warning: could not register the user GC root at {}; this run is not GC-protected ({error:#})",
                link.display()
            );
            Ok((definition.clone(), None))
        }
        Err(error) => Err(error),
    }
}

fn gc_root_link(name: &str, user: bool) -> Result<PathBuf> {
    let directory = if user {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .context("XDG_RUNTIME_DIR is not set for the user manager")?;
        PathBuf::from(runtime).join("cix/gcroots")
    } else {
        PathBuf::from("/run/cix/gcroots")
    };
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating GC root directory {}", directory.display()))?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("setting GC root directory mode on {}", directory.display()))?;
    Ok(directory.join(format!("{name}.root")))
}

fn register_gc_root(link: &Path, output: &Path) -> Result<()> {
    if fs::symlink_metadata(link).is_ok() {
        fs::remove_file(link).with_context(|| format!("replacing GC root {}", link.display()))?;
    }
    let link = link
        .to_str()
        .context("GC root path is not valid UTF-8")?
        .to_owned();
    let output = output
        .to_str()
        .context("store output path is not valid UTF-8")?
        .to_owned();
    let result = nix_store_command(&["--add-root", &link, "--indirect", "--realise", &output])?;
    if !result.status.success() {
        bail!(
            "failed to register GC root {}: {}",
            link,
            command_error(&result)
        );
    }
    if !fs::symlink_metadata(link.as_str())
        .with_context(|| format!("Nix did not create GC root {link}"))?
        .file_type()
        .is_symlink()
    {
        bail!("Nix created a non-symlink GC root at {link}");
    }
    Ok(())
}

fn gc_root_cleanup_command(link: &Path, user: bool, store_only: bool) -> Result<String> {
    let rm = find_path_program("rm", store_only)?;
    let rm = rm
        .to_str()
        .context("rm path is not valid UTF-8")?
        .to_owned();
    let link = link
        .to_str()
        .context("GC root path is not valid UTF-8")?
        .to_owned();
    let prefix = if user { "" } else { "+" };
    Ok(format!("{prefix}{rm} -f {link}"))
}

fn find_path_program(name: &str, store_only: bool) -> Result<PathBuf> {
    let path = std::env::var_os("PATH").context("PATH is not set")?;
    find_path_program_in(
        name,
        store_only.then_some(Path::new("/nix/store")),
        std::env::split_paths(&path),
    )
}

pub(crate) fn find_path_program_in(
    name: &str,
    required_prefix: Option<&Path>,
    directories: impl IntoIterator<Item = PathBuf>,
) -> Result<PathBuf> {
    for directory in directories {
        let candidate = directory.join(name);
        if candidate.is_file() {
            let Some(prefix) = required_prefix else {
                return Ok(candidate);
            };
            let resolved_directory = fs::canonicalize(&directory)
                .with_context(|| format!("resolving {}", directory.display()))?;
            let resolved = resolved_directory.join(name);
            if resolved.starts_with(prefix) && resolved.is_file() {
                return Ok(resolved);
            }
        }
    }
    let location = match required_prefix {
        Some(prefix) => format!(" in {}", prefix.display()),
        None => String::new(),
    };
    bail!("could not find {name}{location} on PATH for GC-root cleanup")
}

fn remove_gc_root(link: Option<&PathBuf>) {
    let Some(link) = link else {
        return;
    };
    if let Err(error) = fs::remove_file(link) {
        if error.kind() != ErrorKind::NotFound {
            eprintln!(
                "warning: failed to remove GC root {}: {error}",
                link.display()
            );
        }
    }
}

fn with_listener_dependencies(definition: &UnitDefinition, sockets: &[String]) -> UnitDefinition {
    let mut definition = definition.clone();
    let sockets = sockets.join(" ");
    definition.properties.extend([
        ("Requires".into(), sockets.clone()),
        ("After".into(), sockets.clone()),
        ("Sockets".into(), sockets.clone()),
    ]);
    let unit_properties = format!("Requires={sockets}\nAfter={sockets}\n");
    definition.text = definition.text.replacen(
        "\n\n[Service]\n",
        &format!("\n{unit_properties}\n[Service]\n"),
        1,
    );
    definition.text.push_str(&format!("Sockets={sockets}\n"));
    definition
}

fn with_log_unit_name(definition: &UnitDefinition, unit: &str) -> UnitDefinition {
    let mut definition = definition.clone();
    let replacement = format!("CIX_RUN={unit}");
    if let Some((_, value)) = definition
        .properties
        .iter_mut()
        .find(|(property, _)| property == "LogExtraFields")
    {
        *value = value
            .split_whitespace()
            .map(|field| {
                if field.starts_with("CIX_RUN=") {
                    replacement.as_str()
                } else {
                    field
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    if let Some(start) = definition.text.find("LogExtraFields=") {
        let end = definition.text[start..]
            .find('\n')
            .map(|offset| start + offset)
            .unwrap_or(definition.text.len());
        let value = definition
            .properties
            .iter()
            .find(|(property, _)| property == "LogExtraFields")
            .map(|(_, value)| value)
            .expect("LogExtraFields property exists");
        definition
            .text
            .replace_range(start..end, &format!("LogExtraFields={value}"));
    }
    definition
}

fn write_service_unit(name: &str, user: bool, definition: &UnitDefinition) -> Result<()> {
    let path = socket_unit_directory(user)?.join(name);
    fs::create_dir_all(socket_unit_directory(user)?)?;
    fs::write(&path, &definition.text)
        .with_context(|| format!("failed to write transient service unit {}", path.display()))?;
    if let Err(error) = daemon_reload(user) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(())
}

fn create_listener_sockets(
    service: &str,
    user: bool,
    config: &ResolvedConfig,
) -> Result<Vec<String>> {
    let mut sockets: Vec<String> = Vec::new();
    for ((listener, address), socket) in config
        .listeners
        .iter()
        .zip(listener_socket_names(service, config))
    {
        if let Err(error) =
            write_socket_unit(&socket, service, listener, &address.to_string(), user)
                .and_then(|()| systemctl_action(user, "start", &socket))
        {
            for socket in &sockets {
                let _ = remove_socket_unit(socket, user);
            }
            return Err(error);
        }
        sockets.push(socket);
    }
    Ok(sockets)
}

fn listener_socket_names(service: &str, config: &ResolvedConfig) -> Vec<String> {
    config
        .listeners
        .keys()
        .map(|listener| format!("{}-{listener}.socket", service.trim_end_matches(".service")))
        .collect()
}

fn write_socket_unit(
    socket: &str,
    service: &str,
    listener: &str,
    address: &str,
    user: bool,
) -> Result<()> {
    let directory = socket_unit_directory(user)?;
    fs::create_dir_all(&directory).with_context(|| {
        format!(
            "failed to create runtime unit directory {}",
            directory.display()
        )
    })?;
    let path = directory.join(socket);
    let text = format!(
        "[Unit]\nDescription=cix listener: {listener} for {service}\nPartOf={service}\nBefore={service}\n\n[Socket]\nListenStream={address}\nFileDescriptorName={listener}\nService={service}\n"
    );
    fs::write(&path, text)
        .with_context(|| format!("failed to write transient socket unit {}", path.display()))?;
    if let Err(error) = daemon_reload(user) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(())
}

fn socket_unit_directory(user: bool) -> Result<PathBuf> {
    if !user {
        return Ok(PathBuf::from("/run/systemd/system"));
    }
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR is not set for the user manager")?;
    Ok(PathBuf::from(runtime).join("systemd/user"))
}

fn listener_sockets(service: &str, user: bool) -> Result<Vec<String>> {
    let configured = systemctl_value(user, service, "Sockets")?;
    let configured = if configured.is_empty() {
        let fragment = systemctl_value(user, service, "FragmentPath")?;
        fs::read_to_string(&fragment)
            .with_context(|| format!("failed to read service unit {fragment}"))?
            .lines()
            .find_map(|line| line.strip_prefix("Sockets="))
            .unwrap_or_default()
            .to_owned()
    } else {
        configured
    };
    Ok(configured
        .split_whitespace()
        .filter(|name| name.starts_with("cix-run-") && name.ends_with(".socket"))
        .map(str::to_owned)
        .collect())
}

fn remove_socket_unit(socket: &str, user: bool) -> Result<()> {
    let _ = systemctl_action(user, "stop", socket);
    let path = socket_unit_directory(user)?.join(socket);
    if path.exists() {
        fs::remove_file(&path).with_context(|| {
            format!("failed to remove transient socket unit {}", path.display())
        })?;
        daemon_reload(user)?;
    }
    Ok(())
}

fn remove_service_unit(service: &str, user: bool) -> Result<()> {
    let path = socket_unit_directory(user)?.join(service);
    if path.exists() {
        fs::remove_file(&path).with_context(|| {
            format!("failed to remove transient service unit {}", path.display())
        })?;
        daemon_reload(user)?;
    }
    Ok(())
}

fn daemon_reload(user: bool) -> Result<()> {
    systemctl_action(user, "daemon-reload", "")
}

fn systemctl_action(user: bool, action: &str, unit: &str) -> Result<()> {
    let mut command = Command::new("systemctl");
    if user {
        command.arg("--user");
    }
    command.arg(action);
    if !unit.is_empty() {
        command.arg(unit);
    }
    let output = command
        .output()
        .with_context(|| format!("failed to invoke systemctl {action}"))?;
    if !output.status.success() {
        bail!("systemctl {action} failed: {}", command_error(&output));
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

pub(crate) fn run_transient_foreground(
    name: &str,
    user: bool,
    definition: &UnitDefinition,
    interactive: bool,
) -> Result<ForegroundResult> {
    run_transient_foreground_with_type(name, user, definition, interactive, "exec")
}

pub(crate) fn run_transient_app(
    name: &str,
    user: bool,
    output: &Path,
    definition: &UnitDefinition,
) -> Result<ForegroundResult> {
    let definition = with_log_unit_name(definition, name);
    let (definition, gc_root) = definition_with_gc_root(name, user, output, &definition)?;
    let result = run_transient_foreground_with_type(name, user, &definition, false, "oneshot");
    remove_gc_root(gc_root.as_ref());
    result
}

fn run_transient_foreground_with_type(
    name: &str,
    user: bool,
    definition: &UnitDefinition,
    interactive: bool,
    service_type: &str,
) -> Result<ForegroundResult> {
    let mut command = Command::new("systemd-run");
    if user {
        command.arg("--user");
    }
    command
        .arg("--quiet")
        .arg("--collect")
        .arg("--wait")
        .arg(if interactive { "--pty" } else { "--pipe" })
        .arg(format!("--service-type={service_type}"))
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
    command.stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .with_context(|| format!("failed to invoke systemd-run for {name}"))?;
    let stderr = child
        .stderr
        .take()
        .context("failed to capture systemd-run diagnostics")?;
    let diagnostics = thread::spawn(move || {
        let mut captured = Vec::new();
        let mut stderr = BufReader::new(stderr);
        loop {
            let mut line = Vec::new();
            match stderr.read_until(b'\n', &mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if should_stream_systemd_diagnostic(&line) {
                        let _ = std::io::stderr().write_all(&line);
                    }
                    captured.extend_from_slice(&line);
                }
                Err(_) => break,
            }
        }
        captured
    });
    let status = child
        .wait()
        .with_context(|| format!("failed to wait for systemd-run for {name}"))?;
    let stderr = diagnostics
        .join()
        .map_err(|_| anyhow::anyhow!("systemd-run diagnostics thread panicked"))?;
    Ok(ForegroundResult {
        status,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn should_stream_systemd_diagnostic(line: &[u8]) -> bool {
    let line = String::from_utf8_lossy(line);
    line.trim_end_matches(['\r', '\n'])
        .strip_prefix("Unknown assignment: ")
        .is_none_or(|property| property.is_empty())
}

fn expand_user_directory_specifiers(value: &str) -> Result<String> {
    let home = std::env::var("HOME").context("HOME is not set for the user manager")?;
    let state = std::env::var("XDG_STATE_HOME").unwrap_or_else(|_| format!("{home}/.local/state"));
    let cache = std::env::var("XDG_CACHE_HOME").unwrap_or_else(|_| format!("{home}/.cache"));
    let config = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| format!("{home}/.config"));
    let logs = format!("{state}/log");
    let runtime = std::env::var("XDG_RUNTIME_DIR")
        .context("XDG_RUNTIME_DIR is not set for the user manager")?;
    Ok(value
        .replace("%S", &state)
        .replace("%C", &cache)
        .replace("%L", &logs)
        .replace("%E", &config)
        .replace("%t", &runtime))
}

fn follow(unit: &StartedUnit) -> Result<()> {
    let mut journal = journal_child(&unit.name, unit.user)?;
    cix_common::INTERRUPTED.store(false, Ordering::Relaxed);
    let previous = unsafe { signal(SIGINT, handle_interrupt as *const () as usize) };
    if previous == SIG_ERR {
        terminate_child(&mut journal);
        bail!("failed to install the Ctrl-C handler");
    }

    loop {
        if cix_common::INTERRUPTED.swap(false, Ordering::Relaxed) {
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
    cix_common::INTERRUPTED.store(true, Ordering::Relaxed);
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

fn nix_store_command(args: &[&str]) -> Result<Output> {
    let invoke = |program: &Path| Command::new(program).args(args).output();
    match invoke(Path::new("nix-store")) {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            invoke(Path::new("/nix/var/nix/profiles/default/bin/nix-store"))
                .context("failed to invoke fallback nix-store executable")
        }
        Err(error) => Err(error).context("failed to invoke nix-store"),
    }
}

pub(crate) fn current_uid() -> Result<u32> {
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

pub(crate) fn nonce() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Fixed width: unit-name length must be host-independent, or every
    // rendered table column (and the drift-checked tour) varies with pid
    // digit count.
    format!("{nanos:016x}{:08x}", std::process::id())
}

pub(crate) fn namespace_failure(error: &anyhow::Error) -> bool {
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

pub(crate) fn capability_failure(error: &anyhow::Error) -> bool {
    format!("{error:#}")
        .to_ascii_lowercase()
        .contains("capabilit")
}

pub(crate) fn with_unit_diagnostics(error: anyhow::Error, name: &str, user: bool) -> anyhow::Error {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{materialize_run_directories, RunOptions};
    use crate::spec::Spec;
    use crate::target::{resolve_installable, split_single_hash};

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
        assert!(namespace_failure(&anyhow::anyhow!(
            "PrivatePIDs is not supported"
        )));
        assert!(!namespace_failure(&anyhow::anyhow!(
            "executable was not found"
        )));
        assert!(capability_failure(&anyhow::anyhow!(
            "Failed at step CAPABILITIES"
        )));
    }

    #[test]
    fn synchronous_user_capability_fallback_drops_private_devices() {
        let definition = UnitDefinition {
            name: "cix-run-web.service".into(),
            target: "cix-run.target".into(),
            text: "[Service]\nPrivateDevices=yes\nCapabilityBoundingSet=\n".into(),
            properties: vec![
                ("PrivateDevices".into(), "yes".into()),
                ("CapabilityBoundingSet".into(), String::new()),
            ],
            environment: Vec::new(),
            argv: Vec::new(),
            degradations: Vec::new(),
        };

        let fallback = without_user_capability_controls(&definition);
        assert!(!fallback.text.contains("PrivateDevices="));
        assert!(!fallback
            .properties
            .iter()
            .any(|(name, _)| name == "PrivateDevices"));
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
            resolve_installable(std::path::Path::new("/"), store_path.to_str().unwrap()).unwrap(),
            store_path
        );
    }

    #[test]
    fn resolves_store_cleanup_tools_for_closed_roots() {
        // Hermetic: fabricate a store-like prefix so the assertion does not
        // depend on the ambient PATH carrying nix-store coreutils (CI hosts
        // resolve rm to /usr/bin/rm).
        let store = tempfile::tempdir().unwrap();
        let bin = store.path().join("fakepkg/bin");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("rm"), b"#!/bin/sh\n").unwrap();
        let rm = find_path_program_in("rm", Some(store.path()), vec![bin.clone()]).unwrap();
        assert!(rm.starts_with(store.path()), "{}", rm.display());
        assert_eq!(rm.file_name().unwrap(), "rm");
    }

    #[test]
    fn captures_old_systemd_unknown_property_diagnostics_without_streaming_them() {
        assert!(!should_stream_systemd_diagnostic(
            b"Unknown assignment: PrivatePIDs=yes\n"
        ));
        assert!(!should_stream_systemd_diagnostic(
            b"Unknown assignment: PrivatePIDs=yes\r\n"
        ));
        assert!(should_stream_systemd_diagnostic(b"Failed to start unit\n"));
        assert!(should_stream_systemd_diagnostic(b"Unknown assignment: \n"));
    }

    #[test]
    fn run_dir_flags_project_host_backing_and_reclassify_roles() {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host");
        fs::create_dir(&host).unwrap();
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"dirs":{"state":["/var/lib/app"],"cache":["/var/cache/app"]}}"#,
        )
        .unwrap();
        let mut service = spec.select_service(None).unwrap().1.clone();
        let options = RunOptions {
            installable: "unused".into(),
            env: Vec::new(),
            port: Vec::new(),
            dirs: vec![
                format!("/var/lib/app=host:{}", host.display()),
                "/var/cache/app=as:state".into(),
            ],
            identity: Some("operator".into()),
            detach: true,
            schedule: None,
            closed_root: false,
            user: false,
            state_directory: directory.path().join("state"),
        };
        let compiled = materialize_run_directories(&mut service, &options).unwrap();
        assert!(service
            .dirs
            .state
            .contains(&PathBuf::from("/var/cache/app")));
        assert!(!service.dirs.state.contains(&PathBuf::from("/var/lib/app")));
        assert!(compiled.extra_properties.iter().any(|(name, value)| {
            name == "BindPaths" && value == &format!("{}:/var/lib/app", host.display())
        }));
        assert!(compiled.unit_properties.iter().any(|(name, value)| {
            name == "RequiresMountsFor" && value == &host.display().to_string()
        }));
    }
}
