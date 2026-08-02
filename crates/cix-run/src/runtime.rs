use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command, ExitStatus, Output, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use cix_common::Ref;
use serde::Deserialize;

use crate::capabilities::HostCapabilities;
use crate::closed_root::{options_for_unit, prepare};
use crate::config::ResolvedConfig;
use crate::spec::{DataDir, ManifestKind, Service, Spec};
use crate::unit::{
    build_unit, build_unit_with_options, UnitCompileOptions, UnitDefinition, UnitDegradation,
    UnitMode, UnitNaming,
};

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
    pub dirs: Vec<String>,
    pub identity: Option<String>,
    pub detach: bool,
    pub schedule: Option<String>,
    pub closed_root: bool,
    pub user: bool,
}

#[derive(Debug)]
struct Target {
    output: PathBuf,
    requested_service: Option<String>,
}

#[derive(Debug)]
pub(crate) struct ResolvedService {
    pub output: PathBuf,
    pub name: String,
    pub kind: ManifestKind,
    pub service: Service,
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

pub fn run(options: RunOptions) -> Result<()> {
    let mut target = resolve_service(&options.installable)?;
    if !options.user && current_uid()? != 0 {
        bail!(
            "cix run targets the system manager and must run as root; use sudo, or pass --user for explicitly degraded dev mode"
        );
    }
    if options.user {
        if options.closed_root {
            eprintln!(
                "warning: --user is degraded development mode; CIP-84 keeps the sealed root for dev/prod parity, but the user manager may still reject individual namespace controls through the D13 fallback"
            );
        } else {
            eprintln!(
                "warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path"
            );
        }
    }
    match target.kind {
        ManifestKind::Service => {
            if options.schedule.is_some() {
                bail!("cix run --schedule is only valid for manifest kind app");
            }
            let directory_options = materialize_run_directories(&mut target.service, &options)?;
            run_resolved(
                target.output,
                &target.name,
                &target.service,
                &options,
                directory_options,
            )
        }
        ManifestKind::App => match options.schedule.as_deref() {
            Some(schedule) => schedule_app(target, &options, schedule),
            None => run_app(target, &options),
        },
    }
}

fn materialize_run_directories(
    service: &mut Service,
    options: &RunOptions,
) -> Result<UnitCompileOptions> {
    if options.dirs.is_empty() {
        return Ok(UnitCompileOptions::cix_run("service"));
    }
    let mut declarations = declared_directories(service);
    let mut extra_properties = Vec::new();
    let mut unit_properties = Vec::new();
    for argument in &options.dirs {
        let (selector, value) = argument.split_once('=').with_context(|| {
            format!("--dir {argument:?}: expected PATH=host:/path or PATH=as:role")
        })?;
        let path = select_run_directory(selector, &declarations)?;
        if value.starts_with("host-idmap:") {
            bail!(
                "--dir PATH=host-idmap:... was retired by CIP-81; write the same directory materialization in an anonymous compose JSON and run `cix run --compose <file|->`"
            );
        }
        let (_role, writable) = declarations
            .remove(&path)
            .expect("selected declaration exists");
        if let Some(host) = value.strip_prefix("host:") {
            let idmap = false;
            let host = PathBuf::from(host);
            if !host.is_absolute() {
                bail!("--dir {argument:?}: host backing path must be absolute");
            }
            if !options.user && options.identity.is_none() {
                bail!("--dir {argument:?}: host backing requires --identity for a static host identity (D48d)");
            }
            let metadata = fs::metadata(&host).with_context(|| {
                format!(
                    "--dir {argument:?}: host backing {} must pre-exist",
                    host.display()
                )
            })?;
            if !metadata.is_dir() {
                bail!(
                    "--dir {argument:?}: host backing {} must be a directory",
                    host.display()
                );
            }
            extra_properties.push((
                if writable {
                    "BindPaths"
                } else {
                    "BindReadOnlyPaths"
                }
                .into(),
                if idmap {
                    format!("{}:{}:idmap", host.display(), path.display())
                } else {
                    format!("{}:{}", host.display(), path.display())
                },
            ));
            unit_properties.push(("RequiresMountsFor".into(), host.display().to_string()));
            remove_run_directory(service, &path);
        } else if let Some(role) = value.strip_prefix("as:") {
            let role = parse_run_role(role)?;
            remove_run_directory(service, &path);
            insert_run_directory(service, role, path);
        } else if let Some(name) = value.strip_prefix("shared:") {
            if name.is_empty()
                || !name.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
            {
                bail!("--dir {argument:?}: shared name must be lowercase ASCII letters, digits, '.', '_', or '-'");
            }
            if !options.user {
                let (group, host) = prepare_run_shared_directory(name)?;
                extra_properties.push(("SupplementaryGroups".into(), group));
                extra_properties.push(("UMask".into(), "0002".into()));
                extra_properties.push((
                    "BindPaths".into(),
                    format!("{}:{}", host.display(), path.display()),
                ));
                unit_properties.push(("RequiresMountsFor".into(), host.display().to_string()));
                remove_run_directory(service, &path);
            } else {
                bail!("--dir {argument:?}: shared directories require the system manager; --user has no cix identity registry");
            }
        } else {
            bail!("--dir {argument:?}: expected host:/path, shared:name, or as:role");
        }
    }
    if let Some(identity) = &options.identity {
        extra_properties.extend([
            ("DynamicUser".into(), "no".into()),
            ("User".into(), identity.clone()),
            ("Group".into(), identity.clone()),
        ]);
    }
    Ok(UnitCompileOptions {
        naming: UnitNaming::cix_run("service"),
        extra_properties,
        unit_properties,
        log_fields: vec![("CIX_RUN".into(), "cix-run.service".into())],
        probe_binary: None,
        closed_root: None,
    })
}

fn prepare_run_shared_directory(name: &str) -> Result<(String, PathBuf)> {
    let group = format!("cix-rs-{:016x}", run_directory_hash(name));
    let temporary =
        tempfile::NamedTempFile::new().context("creating cix-run shared-group registry")?;
    fs::write(temporary.path(), format!("g {group} - -\n"))?;
    let output = Command::new("systemd-sysusers")
        .arg(temporary.path())
        .output()
        .context("applying cix-run shared-group registry")?;
    if !output.status.success() {
        bail!(
            "creating cix-run shared group {group}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let path = Path::new("/var/lib/cix-run/shared").join(name);
    fs::create_dir_all(&path)
        .with_context(|| format!("creating cix-run shared directory {}", path.display()))?;
    let output = Command::new("chown")
        .arg(format!("root:{group}"))
        .arg(&path)
        .output()
        .with_context(|| format!("setting cix-run shared directory group {}", path.display()))?;
    if !output.status.success() {
        bail!(
            "setting cix-run shared directory group: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o2770))?;
    Ok((group, path))
}

fn run_directory_hash(name: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in b"cix-run\0".iter().copied().chain(name.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn declared_directories(
    service: &Service,
) -> std::collections::BTreeMap<PathBuf, (Option<&'static str>, bool)> {
    let mut paths = std::collections::BTreeMap::new();
    for (role, values) in [
        (Some("state"), service.dirs.state.as_slice()),
        (Some("cache"), service.dirs.cache.as_slice()),
        (Some("logs"), service.dirs.logs.as_slice()),
        (Some("config"), service.dirs.config.as_slice()),
        (Some("run"), service.dirs.run.as_deref().unwrap_or_default()),
    ] {
        for path in values {
            paths.insert(path.clone(), (role, true));
        }
    }
    for DataDir { path, ro } in &service.dirs.data {
        paths.insert(path.clone(), (None, !ro));
    }
    paths
}

fn select_run_directory(
    selector: &str,
    declarations: &std::collections::BTreeMap<PathBuf, (Option<&'static str>, bool)>,
) -> Result<PathBuf> {
    if selector.starts_with('/') {
        let path = PathBuf::from(selector);
        if declarations.contains_key(&path) {
            return Ok(path);
        }
        bail!("--dir {selector}: path is not declared by the item");
    }
    let matching = declarations
        .iter()
        .filter(|(_, (role, _))| *role == Some(selector))
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [path] => Ok((*path).clone()),
        [] => bail!("--dir {selector}: item has no declared {selector} directory"),
        _ => bail!("--dir {selector}: role names are only unambiguous for one declared path; use the absolute app path"),
    }
}

fn remove_run_directory(service: &mut Service, path: &Path) {
    service.dirs.state.retain(|candidate| candidate != path);
    service.dirs.cache.retain(|candidate| candidate != path);
    service.dirs.logs.retain(|candidate| candidate != path);
    service.dirs.config.retain(|candidate| candidate != path);
    if let Some(run) = &mut service.dirs.run {
        run.retain(|candidate| candidate != path);
    }
    service.dirs.data.retain(|candidate| candidate.path != path);
}

fn parse_run_role(role: &str) -> Result<&'static str> {
    match role {
        "state" => Ok("state"),
        "cache" => Ok("cache"),
        "logs" => Ok("logs"),
        "config" => Ok("config"),
        "run" => Ok("run"),
        _ => bail!("--dir as:{role}: expected state, cache, logs, config, or run"),
    }
}

fn insert_run_directory(service: &mut Service, role: &str, path: PathBuf) {
    match role {
        "state" => service.dirs.state.push(path),
        "cache" => service.dirs.cache.push(path),
        "logs" => service.dirs.logs.push(path),
        "config" => service.dirs.config.push(path),
        "run" => service.dirs.run.get_or_insert_with(Vec::new).push(path),
        _ => unreachable!("validated role"),
    }
}

fn run_app(target: ResolvedService, options: &RunOptions) -> Result<()> {
    if options.detach {
        bail!("cix run --detach is not valid for manifest kind app; apps run to completion");
    }
    if !options.port.is_empty() {
        bail!("cix run -p/--port is not valid for manifest kind app (D47)");
    }
    let config = ResolvedConfig::resolve(&target.service, &options.env, &[])?;
    let mode = if options.user {
        UnitMode::UserFull
    } else {
        UnitMode::System
    };
    let definition = build_runtime_unit(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        options.closed_root,
        &format!("cix-run-{}-app.service", target.name),
    )?;
    warn_degradations(&definition.degradations);
    if !options.user {
        let name = format!("cix-run-{}-{}.service", target.name, nonce());
        let result = run_transient_app(&name, false, &target.output, &definition)?;
        if result.status.success() {
            return Ok(());
        }
        let error = with_unit_diagnostics(
            anyhow::anyhow!("app unit {name} failed: {}", result.stderr.trim()),
            &name,
            false,
        );
        if !namespace_failure(&error) {
            return finish_app(result.status);
        }
        eprintln!("warning: the system manager rejected PrivatePIDs isolation ({error:#})");
        eprintln!(
            "warning: retrying without PrivatePIDs; this app shares the host PID namespace (D36 degraded fallback)"
        );
        let fallback = without_properties(&definition, &["PrivatePIDs"]);
        return finish_app(
            run_transient_app(
                &format!("cix-run-{}-{}.service", target.name, nonce()),
                false,
                &target.output,
                &fallback,
            )?
            .status,
        );
    }

    let (status, error) = failed_app_attempt(&target.name, true, &target.output, &definition)?;
    if status.success() {
        return Ok(());
    }
    if capability_failure(&error) {
        eprintln!("warning: user manager rejected capability controls ({error:#})");
        eprintln!(
            "warning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, ProtectKernelLogs, and PrivateDevices"
        );
        let without_capabilities = without_user_capability_controls(&definition);
        let (retry_status, retry_error) =
            failed_app_attempt(&target.name, true, &target.output, &without_capabilities)?;
        if retry_status.success() {
            return Ok(());
        }
        if !namespace_failure(&retry_error) {
            return finish_app(retry_status);
        }
        return run_app_degraded(&target, &config, retry_error);
    }
    if namespace_failure(&error) {
        return run_app_degraded(&target, &config, error);
    }
    finish_app(status)
}

fn schedule_app(target: ResolvedService, options: &RunOptions, schedule: &str) -> Result<()> {
    if schedule.trim().is_empty() {
        bail!("cix run --schedule must not be empty");
    }
    if options.detach {
        bail!("cix run --detach is not valid with --schedule; the timer is already asynchronous");
    }
    if !options.port.is_empty() {
        bail!("cix run -p/--port is not valid for manifest kind app (D47)");
    }
    let config = ResolvedConfig::resolve(&target.service, &options.env, &[])?;
    let mode = if options.user {
        UnitMode::UserFull
    } else {
        UnitMode::System
    };
    let definition = build_runtime_unit(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        options.closed_root,
        &format!("cix-run-{}-scheduled.service", target.name),
    )?;
    warn_degradations(&definition.degradations);
    start_scheduled_app(
        &target.output,
        &target.name,
        options.user,
        schedule,
        &definition,
    )
}

fn start_scheduled_app(
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

fn failed_app_attempt(
    app_name: &str,
    user: bool,
    output: &Path,
    definition: &UnitDefinition,
) -> Result<(ExitStatus, anyhow::Error)> {
    let name = format!("cix-run-{app_name}-{}.service", nonce());
    let result = run_transient_app(&name, user, output, definition)?;
    let error = with_unit_diagnostics(
        anyhow::anyhow!("app unit {name} failed: {}", result.stderr.trim()),
        &name,
        user,
    );
    Ok((result.status, error))
}

fn run_app_degraded(
    target: &ResolvedService,
    config: &ResolvedConfig,
    error: anyhow::Error,
) -> Result<()> {
    eprintln!("warning: the user manager rejected mount-namespace sandboxing ({error:#})");
    eprintln!(
        "warning: retrying without PrivateUsers, PrivatePIDs, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; this is the D13 degraded development path"
    );
    let degraded = build_unit(
        &target.output,
        &target.name,
        &target.service,
        config,
        UnitMode::UserDegraded,
    )?;
    let name = format!("cix-run-{}-{}.service", target.name, nonce());
    finish_app(run_transient_app(&name, true, &target.output, &degraded)?.status)
}

fn finish_app(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    process::exit(status.code().unwrap_or(1));
}

fn run_resolved(
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

fn build_runtime_unit(
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

fn find_path_program_in(
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

fn run_transient_app(
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

pub(crate) fn resolve_service(input: &str) -> Result<ResolvedService> {
    let target = resolve_target(input)?;
    let spec = Spec::load(&target.output)?;
    match spec.select_service(target.requested_service.as_deref()) {
        Ok((name, service)) => Ok(ResolvedService {
            output: target.output,
            name: name.to_owned(),
            kind: spec.kind,
            service: service.clone(),
        }),
        Err(original_error) if target.requested_service.is_none() => {
            let Some((installable, service_name)) = split_single_hash(input) else {
                return Err(original_error);
            };
            let output = resolve_installable(installable)?;
            let fallback_spec = Spec::load(&output)?;
            let (name, service) = fallback_spec.select_service(Some(service_name))?;
            Ok(ResolvedService {
                output,
                name: name.to_owned(),
                kind: fallback_spec.kind,
                service: service.clone(),
            })
        }
        Err(error) => Err(error),
    }
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

    match Ref::parse(installable) {
        Ok(reference) => match cix_index::resolve(installable) {
            Ok(output) => return Ok(PathBuf::from(output.store_path)),
            Err(error) if reference.root_url.is_some() => {
                return Err(error).with_context(|| {
                    format!("failed to resolve qualified cix ref {installable:?}")
                });
            }
            Err(_) => {}
        },
        Err(error) if Ref::looks_like_untagged_ref(installable) => return Err(error),
        Err(_) => {}
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
    nix_command(&["build", "--no-link", "--print-out-paths", "--", installable])
        .with_context(|| format!("failed to invoke nix for installable {installable:?}"))
}

fn nix_command(args: &[&str]) -> Result<Output> {
    let invoke = |program: &Path| Command::new(program).args(args).output();
    match invoke(Path::new("nix")) {
        Ok(output) => Ok(output),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            invoke(Path::new("/nix/var/nix/profiles/default/bin/nix"))
                .context("failed to invoke fallback nix executable")
        }
        Err(error) => Err(error).context("failed to invoke nix"),
    }
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
    let counter = NONCE_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{nanos:x}{counter:x}")
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
    let results = rows
        .iter()
        .map(|(manager, unit)| {
            systemctl_value(*manager == "user", &unit.unit, "Result")
                .map(|result| result_label(&result).to_owned())
                .unwrap_or_else(|_| "-".into())
        })
        .collect::<Vec<_>>();
    let result_width = results.iter().map(String::len).max().unwrap_or(6).max(6);
    println!(
        "{:<manager_width$}  {:<unit_width$}  {:<10}  {:<result_width$}  DESCRIPTION",
        "MANAGER", "UNIT", "STATE", "RESULT"
    );
    for ((manager, unit), result) in rows.into_iter().zip(results) {
        let description = if unit.unit.ends_with(".socket") {
            socket_description(manager == "user", &unit.unit).unwrap_or(unit.description)
        } else {
            unit.description
        };
        println!(
            "{manager:<manager_width$}  {:<unit_width$}  {:<10}  {result:<result_width$}  {}",
            unit.unit,
            format!("{}/{}", unit.active, unit.sub),
            description
        );
    }
    Ok(())
}

fn socket_description(user: bool, unit: &str) -> Result<String> {
    let fragment = systemctl_value(user, unit, "FragmentPath")?;
    let text = fs::read_to_string(&fragment)
        .with_context(|| format!("failed to read socket unit {fragment}"))?;
    let listen = text
        .lines()
        .find_map(|line| line.strip_prefix("ListenStream="))
        .context("socket unit has no ListenStream")?;
    let service = text
        .lines()
        .find_map(|line| line.strip_prefix("Service="))
        .context("socket unit has no Service")?;
    Ok(format!("listening {listen} -> {service}"))
}

fn result_label(result: &str) -> &str {
    if result == "watchdog" {
        "liveness watchdog missed"
    } else {
        result
    }
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
            resolve_installable(store_path.to_str().unwrap()).unwrap(),
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
