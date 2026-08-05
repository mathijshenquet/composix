use std::collections::BTreeMap;
use std::ffi::CStr;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::capabilities::HostCapabilities;
use crate::closed_root::ClosedRootOptions;
use crate::config::ResolvedConfig;
use crate::spec::{Protocol, Service};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitMode {
    System,
    UserFull,
    UserDegraded,
}

/// Names used while compiling one service unit.
///
/// `cix run` uses [`UnitNaming::cix_run`] by default. Other callers, notably a compose
/// generator, can provide their own names without duplicating the hardening compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitNaming {
    /// The generated service unit name, for example `cix-mycomp-web.service`.
    pub unit: String,
    /// The slice owning the generated service, for example `cix-mycomp.slice`.
    pub slice: String,
    /// The target that a higher-level generator may use to group generated units.
    pub target: String,
    /// Prefix for systemd managed directory names, for example `cix-mycomp`.
    pub directory_prefix: String,
}

impl UnitNaming {
    /// Return cix-run's established naming scheme for `service_name`.
    pub fn cix_run(service_name: &str) -> Self {
        Self {
            unit: format!("cix-run-{service_name}.service"),
            slice: "cix-run.slice".into(),
            target: "cix-run.target".into(),
            directory_prefix: "cix-run".into(),
        }
    }
}

impl Default for UnitNaming {
    fn default() -> Self {
        Self::cix_run("service")
    }
}

/// Caller-supplied additions to the generated systemd service unit.
///
/// This deliberately accepts ordinary unit properties so generators can add narrowly-scoped
/// composition claims such as `SupplementaryGroups=` or `BindPaths=` without forking cix-run's
/// sandbox compiler.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnitCompileOptions {
    /// Naming scheme for the compiled service.
    pub naming: UnitNaming,
    /// Additional systemd service properties appended after cix-run's own properties.
    pub extra_properties: Vec<(String, String)>,
    /// Additional [Unit] properties such as RequiresMountsFor for host backing.
    pub unit_properties: Vec<(String, String)>,
    /// Indexed journald fields identifying the cix owner of this unit.
    pub log_fields: Vec<(String, String)>,
    /// Exact cix executable used by native readiness/liveness adapters.
    ///
    /// Production callers normally leave this unset so it resolves to the running binary.
    pub probe_binary: Option<PathBuf>,
    /// CIP-84 phase-1 sealed root configuration. None preserves the pre-audit runtime.
    pub closed_root: Option<ClosedRootOptions>,
}

impl UnitCompileOptions {
    /// Return the default cix-run options for `service_name`.
    pub fn cix_run(service_name: &str) -> Self {
        Self {
            naming: UnitNaming::cix_run(service_name),
            extra_properties: Vec::new(),
            unit_properties: Vec::new(),
            log_fields: vec![("CIX_RUN".into(), format!("cix-run-{service_name}.service"))],
            probe_binary: None,
            closed_root: None,
        }
    }
}

/// A service unit compiled from a cix specification.
///
/// The `name` and `target` fields let callers install the generated unit into their own graph;
/// `text` is suitable for a unit file and `properties`/`environment` support transient-unit APIs.
#[derive(Debug, Clone)]
pub struct CompiledUnit {
    /// Unit name selected by [`UnitCompileOptions::naming`].
    pub name: String,
    /// Target name selected by [`UnitCompileOptions::naming`].
    pub target: String,
    /// Rendered service unit file.
    pub text: String,
    /// Service properties in deterministic order.
    pub properties: Vec<(String, String)>,
    /// Environment assignments in deterministic order.
    pub environment: Vec<(String, String)>,
    /// Resolved `ExecStart` argv.
    pub argv: Vec<String>,
    /// Host-specific hardening properties omitted from this unit.
    pub degradations: Vec<UnitDegradation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDegradation {
    pub property: String,
    pub reason: String,
}

impl CompiledUnit {
    pub(crate) fn override_argv(&mut self, argv: Vec<String>) {
        self.argv = argv;
        let replacement = format!("ExecStart={}", exec_command(&self.argv));
        if let Some(start) = self.text.find("ExecStart=") {
            let end = self.text[start..]
                .find('\n')
                .map(|offset| start + offset)
                .unwrap_or(self.text.len());
            self.text.replace_range(start..end, &replacement);
        }
    }
}

pub(crate) type UnitDefinition = CompiledUnit;

pub fn generate_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
) -> Result<String> {
    Ok(compile_unit(
        output,
        service_name,
        service,
        config,
        mode,
        &UnitCompileOptions::cix_run(service_name),
    )?
    .text)
}

/// Compile a cix service into a systemd service unit.
///
/// The supplied [`UnitCompileOptions`] controls generated names and permits extra properties.
/// The default cix-run behavior is available through [`UnitCompileOptions::cix_run`].
pub fn compile_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
    options: &UnitCompileOptions,
) -> Result<CompiledUnit> {
    compile_unit_for_host(
        output,
        service_name,
        service,
        config,
        mode,
        options,
        &HostCapabilities::all_supported(),
    )
}

pub fn compile_unit_for_host(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
    options: &UnitCompileOptions,
    capabilities: &HostCapabilities,
) -> Result<CompiledUnit> {
    build_unit_with_options(
        output,
        service_name,
        service,
        config,
        mode,
        options,
        capabilities,
    )
}

pub(crate) fn build_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
) -> Result<UnitDefinition> {
    build_unit_with_options(
        output,
        service_name,
        service,
        config,
        mode,
        &UnitCompileOptions::cix_run(service_name),
        &HostCapabilities::all_supported(),
    )
}

pub(crate) fn build_unit_with_options(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
    options: &UnitCompileOptions,
    capabilities: &HostCapabilities,
) -> Result<UnitDefinition> {
    if !output.is_absolute() {
        bail!("store output path {} is not absolute", output.display());
    }
    if !service.dirs.data.is_empty() {
        bail!(
            "DIR declares operator-supplied data; materialization arrives with compose (docs/cixfile.md#role-dirs); for a cix-managed dir pick a role: STATEDIR/CACHEDIR/LOGDIR/RUNDIR"
        );
    }

    let item_env = config.item_environment(output)?;
    let argv = resolved_argv(output, "start", &service.start, &item_env)?;
    if options.closed_root.is_some() {
        validate_closed_root_executable(output, &argv[0])?;
        if let Some((name, port)) = config.ports.iter().find(|(_, port)| **port < 1024) {
            bail!(
                "closed root cannot grant {name} port {port}: PrivateUsers isolates capabilities from the host network namespace; use a port >= 1024 or a named LISTENER for systemd socket activation"
            );
        }
    }
    let mut properties = Vec::new();
    properties.push(("Type".into(), "exec".into()));
    if let Some(stop_signal) = &service.stop_signal {
        validate_stop_signal(stop_signal)?;
        properties.push(("KillSignal".into(), stop_signal.clone()));
    }
    properties.push(("Slice".into(), options.naming.slice.clone()));
    let log_fields = options
        .log_fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .chain(std::iter::once(format!("CIX_ITEM={}", output.display())))
        .collect::<Vec<_>>()
        .join(" ");
    properties.push(("LogExtraFields".into(), log_fields));

    if mode == UnitMode::System || options.closed_root.is_some() {
        add_mounts(&mut properties, output, service)?;
    }

    crate::directories::add_properties(
        &mut properties,
        &format!("{}-{service_name}", options.naming.directory_prefix),
        &service.dirs,
        mode != UnitMode::System,
        mode != UnitMode::UserDegraded,
    );

    if mode == UnitMode::System {
        properties.push(("DynamicUser".into(), "yes".into()));
    } else if mode == UnitMode::UserFull {
        properties.push(("PrivateUsers".into(), "yes".into()));
    }

    if let Some(closed_root) = &options.closed_root {
        if mode == UnitMode::UserDegraded {
            bail!("closed root requires mount-namespace support");
        }
        add_closed_root(
            &mut properties,
            service,
            mode,
            options,
            closed_root,
            capabilities.systemd_version,
        )?;
    }

    if mode != UnitMode::UserDegraded {
        properties.push(("ProtectSystem".into(), "strict".into()));
        properties.push(("ProtectHome".into(), "yes".into()));
        properties.push(("PrivateTmp".into(), "yes".into()));
        properties.push(("PrivatePIDs".into(), "yes".into()));
        crate::devices::add_policy(&mut properties, service)?;
    }
    properties.extend([
        ("NoNewPrivileges".into(), "yes".into()),
        ("RestrictSUIDSGID".into(), "yes".into()),
        ("ProtectKernelTunables".into(), "yes".into()),
    ]);
    if mode != UnitMode::UserDegraded {
        properties.extend([
            ("ProtectKernelModules".into(), "yes".into()),
            ("ProtectKernelLogs".into(), "yes".into()),
        ]);
    }
    properties.extend([
        ("ProtectControlGroups".into(), "yes".into()),
        ("LockPersonality".into(), "yes".into()),
    ]);
    if !service.has_claim("jit") {
        properties.push(("MemoryDenyWriteExecute".into(), "yes".into()));
    }
    properties.push(("SystemCallFilter".into(), "@system-service".into()));
    if mode != UnitMode::UserDegraded {
        if config.ports.values().any(|port| *port < 1024) {
            properties.push(("AmbientCapabilities".into(), "CAP_NET_BIND_SERVICE".into()));
            properties.push((
                "CapabilityBoundingSet".into(),
                "CAP_NET_BIND_SERVICE".into(),
            ));
        } else {
            properties.push(("CapabilityBoundingSet".into(), String::new()));
        }
    }
    if service.has_network() {
        properties.push((
            "RestrictAddressFamilies".into(),
            "AF_UNIX AF_INET AF_INET6".into(),
        ));
    } else {
        properties.push(("RestrictAddressFamilies".into(), "AF_UNIX".into()));
        properties.push(("PrivateNetwork".into(), "yes".into()));
    }
    add_socket_bind_restrictions(&mut properties, service, config);
    if let Some(shm) = &service.shm {
        properties.push(("TemporaryFileSystem".into(), format!("/dev/shm:size={shm}")));
    }
    if let Some(start_pre) = &service.start_pre {
        let start_pre = resolved_argv(output, "start_pre", start_pre, &item_env)?;
        if options.closed_root.is_some() {
            validate_closed_root_executable(output, &start_pre[0])?;
        }
        properties.push(("ExecStartPre".into(), exec_command(&start_pre)));
    }
    crate::health::add_properties(&mut properties, service, options)?;

    let mut environment = item_env
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<Vec<_>>();
    for (name, secret) in &service.secrets {
        if let Some(as_env) = &secret.as_env {
            environment.push((as_env.clone(), format!("%d/{name}")));
        }
    }
    crate::directories::add_environment(&mut environment, &service.dirs);
    if mode != UnitMode::System {
        environment.push((
            "CIX_APP".into(),
            output
                .to_str()
                .context("store output path is not valid UTF-8")?
                .to_owned(),
        ));
    }

    properties.extend(options.extra_properties.iter().cloned());
    let degradations = apply_host_capabilities(&mut properties, mode, capabilities);
    let mut text = render(
        service_name,
        &argv,
        &environment,
        &properties,
        &options.unit_properties,
    );
    for secret in service.secrets.values() {
        if let Some(as_env) = &secret.as_env {
            text = text.replace(&format!("{as_env}=%%d/"), &format!("{as_env}=%d/"));
        }
    }
    Ok(CompiledUnit {
        name: options.naming.unit.clone(),
        target: options.naming.target.clone(),
        text,
        properties,
        environment,
        argv,
        degradations,
    })
}

fn validate_stop_signal(signal: &str) -> Result<()> {
    if matches!(
        signal,
        "SIGHUP"
            | "SIGINT"
            | "SIGQUIT"
            | "SIGILL"
            | "SIGTRAP"
            | "SIGABRT"
            | "SIGBUS"
            | "SIGFPE"
            | "SIGKILL"
            | "SIGUSR1"
            | "SIGSEGV"
            | "SIGUSR2"
            | "SIGPIPE"
            | "SIGALRM"
            | "SIGTERM"
            | "SIGSTKFLT"
            | "SIGCHLD"
            | "SIGCONT"
            | "SIGSTOP"
            | "SIGTSTP"
            | "SIGTTIN"
            | "SIGTTOU"
            | "SIGURG"
            | "SIGXCPU"
            | "SIGXFSZ"
            | "SIGVTALRM"
            | "SIGPROF"
            | "SIGWINCH"
            | "SIGIO"
            | "SIGPWR"
            | "SIGSYS"
    ) {
        Ok(())
    } else {
        bail!("stopSignal requires a known signal name, got {signal:?}")
    }
}

fn add_closed_root(
    properties: &mut Vec<(String, String)>,
    service: &Service,
    mode: UnitMode,
    options: &UnitCompileOptions,
    closed_root: &ClosedRootOptions,
    systemd_version: u32,
) -> Result<()> {
    let root = closed_root
        .root_directory()
        .to_str()
        .context("closed-root path is not valid UTF-8")?
        .replace('%', "%%");
    let gc_roots = closed_root
        .gc_root_directory()
        .to_str()
        .context("closed-root GC-root path is not valid UTF-8")?
        .replace('%', "%%");
    properties.extend([
        ("RootDirectory".into(), root.clone()),
        ("MountAPIVFS".into(), "yes".into()),
        ("BindReadOnlyPaths".into(), "/nix/store".into()),
        ("BindPaths".into(), format!("{root}/nss/passwd:/etc/passwd")),
        ("BindPaths".into(), format!("{root}/nss/group:/etc/group")),
        ("BindPaths".into(), format!("{gc_roots}:{gc_roots}")),
    ]);
    let identity = closed_root_identity(mode, options, closed_root)?;
    let identity_directory = format!("cix-nss-{:016x}", identity_hash(&options.naming.unit));
    properties.push(("RuntimeDirectory".into(), identity_directory.clone()));
    properties.push(("RuntimeDirectoryMode".into(), "0700".into()));
    if mode == UnitMode::System
        && !options
            .extra_properties
            .iter()
            .any(|(name, _)| name == "User")
    {
        properties.push(("User".into(), identity.clone()));
    }
    if !properties.iter().any(|(name, _)| name == "PrivateUsers") {
        properties.push(("PrivateUsers".into(), "yes".into()));
    }
    if service.has_claim("egress") {
        let resolver = closed_root
            .resolver_source()
            .to_str()
            .context("closed-root resolver source is not valid UTF-8")?;
        properties.push((
            "BindReadOnlyPaths".into(),
            format!("{resolver}:/etc/resolv.conf"),
        ));
    }
    if systemd_version < 257 {
        // Before v257, MountAPIVFS did not imply BindLogSockets; RootDirectory
        // therefore needs the three journald endpoints named by systemd.exec(5).
        for socket in [
            "/dev/log",
            "/run/systemd/journal/socket",
            "/run/systemd/journal/stdout",
        ] {
            properties.push(("BindReadOnlyPaths".into(), socket.into()));
        }
    }
    let binary = options
        .probe_binary
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_exe)
        .context("resolving the cix binary for closed-root NSS generation")?;
    if !binary.is_absolute() {
        bail!(
            "cix closed-root helper {} is not absolute",
            binary.display()
        );
    }
    properties.push((
        "ExecStartPre".into(),
        format!(
            "+{} closed-root-nss {} {}",
            quote_exec_word(&binary.to_string_lossy()),
            quote_exec_word(&identity),
            quote_exec_word(&format!("/run/{identity_directory}"))
        ),
    ));
    Ok(())
}

fn closed_root_identity(
    mode: UnitMode,
    options: &UnitCompileOptions,
    closed_root: &ClosedRootOptions,
) -> Result<String> {
    if let Some(identity) = closed_root.identity_override() {
        return Ok(identity.to_owned());
    }
    if let Some((_, identity)) = options
        .extra_properties
        .iter()
        .rev()
        .find(|(name, _)| name == "User")
    {
        return Ok(identity.clone());
    }
    if mode == UnitMode::System {
        return Ok(format!("cixr-{:016x}", identity_hash(&options.naming.unit)));
    }
    let uid = unsafe { libc::geteuid() };
    let passwd = unsafe { libc::getpwuid(uid) };
    if passwd.is_null() {
        bail!("current uid {uid} has no passwd identity for closed-root --user mode");
    }
    unsafe { CStr::from_ptr((*passwd).pw_name) }
        .to_str()
        .context("current passwd identity is not valid UTF-8")
        .map(str::to_owned)
}

fn identity_hash(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in value.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn apply_host_capabilities(
    properties: &mut Vec<(String, String)>,
    mode: UnitMode,
    capabilities: &HostCapabilities,
) -> Vec<UnitDegradation> {
    const PERSISTENT_DIRECTORIES: [&str; 4] = [
        "StateDirectory",
        "CacheDirectory",
        "LogsDirectory",
        "ConfigurationDirectory",
    ];

    let mut degradations = Vec::new();

    if mode == UnitMode::System
        && properties
            .iter()
            .any(|(name, value)| name == "DynamicUser" && value == "yes")
        && properties
            .iter()
            .any(|(name, value)| name == "PrivatePIDs" && value == "yes")
        && properties
            .iter()
            .any(|(name, _)| PERSISTENT_DIRECTORIES.contains(&name.as_str()))
    {
        if let Some(reason) = capabilities
            .private_pids_with_persistent_directories
            .unsupported_reason()
        {
            properties.retain(|(name, _)| name != "PrivatePIDs");
            for (name, value) in properties.iter_mut() {
                if matches!(
                    name.as_str(),
                    "StateDirectoryMode" | "CacheDirectoryMode" | "LogsDirectoryMode"
                ) {
                    // Without PrivatePIDs, systemd cannot retain the managed directory's
                    // ID-mapped view. The private host backing still confines this view.
                    *value = "0733".into();
                }
            }
            degradations.push(UnitDegradation {
                property: "PrivatePIDs=yes".into(),
                reason: reason.into(),
            });
        }
    }

    if mode == UnitMode::UserFull
        && properties
            .iter()
            .any(|(name, value)| name == "PrivateDevices" && value == "yes")
    {
        if let Some(reason) = capabilities.user_private_devices.unsupported_reason() {
            properties.retain(|(name, _)| name != "PrivateDevices");
            degradations.push(UnitDegradation {
                property: "PrivateDevices=yes".into(),
                reason: reason.into(),
            });
        }
    }

    for (name, value) in properties.clone() {
        if let Some(reason) = capabilities.unsupported_directive_reason(&name) {
            properties.retain(|(candidate, _)| candidate != &name);
            degradations.push(UnitDegradation {
                property: format!("{name}={value}"),
                reason: reason.into(),
            });
        }
    }

    degradations
}

fn add_socket_bind_restrictions(
    properties: &mut Vec<(String, String)>,
    service: &Service,
    config: &ResolvedConfig,
) {
    if service.ports.is_empty() && service.listeners.is_empty() {
        return;
    }
    for (name, port) in &service.ports {
        let value = config.ports.get(name).expect("resolved declared port");
        let protocol = match port.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        };
        properties.push(("SocketBindAllow".into(), format!("{protocol}:{value}")));
    }
    properties.push(("SocketBindDeny".into(), "any".into()));
}

fn add_mounts(
    properties: &mut Vec<(String, String)>,
    output: &Path,
    service: &Service,
) -> Result<()> {
    for mount in service.mounts.as_deref().unwrap_or_default() {
        let relative = mount
            .strip_prefix("/")
            .expect("validated mount paths are absolute");
        let source = output.join(relative);
        if !source.exists() {
            bail!(
                "declared mount {} is missing from store item at {}",
                mount.display(),
                source.display()
            );
        }
        properties.push((
            "BindReadOnlyPaths".into(),
            format!("{}:{}", source.display(), mount.display()),
        ));
    }
    Ok(())
}

fn resolved_argv(
    output: &Path,
    field: &str,
    command: &[String],
    env: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    let mut argv = command
        .iter()
        .map(|argument| interpolate(field, argument, env))
        .collect::<Result<Vec<_>>>()?;

    let executable = Path::new(&argv[0]);
    if executable == Path::new("/bin/sh") {
        bail!(
            "/bin/sh is not an ambient runtime dependency; name the shell explicitly, for example START ${{pkgs.bash}}/bin/sh (CIP-80)"
        );
    }
    let resolved = if executable.is_absolute() {
        clean_executable(executable)?;
        executable.to_owned()
    } else if executable.components().count() == 1 {
        resolve_item_program(output, executable, env)?
    } else {
        clean_executable(executable)?;
        output.join(executable)
    };
    if !resolved.starts_with("/nix/store") {
        bail!(
            "executable {} does not resolve to an absolute Nix store path",
            resolved.display()
        );
    }
    argv[0] = resolved
        .into_os_string()
        .into_string()
        .map_err(|_| anyhow::anyhow!("executable path is not valid UTF-8"))?;
    Ok(argv)
}

fn validate_closed_root_executable(output: &Path, executable: &str) -> Result<()> {
    let file = match std::fs::File::open(executable) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(error).with_context(|| {
                format!("inspecting closed-root executable {executable} for its interpreter")
            })
        }
    };
    let mut bytes = Vec::with_capacity(256);
    file.take(256)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading closed-root executable {executable} interpreter"))?;
    let first_line = bytes
        .split(|byte| *byte == b'\n')
        .next()
        .unwrap_or_default();
    if first_line == b"#!/bin/sh" || first_line.starts_with(b"#!/bin/sh ") {
        bail!(
            "closed-root executable {executable} names /bin/sh, which is never injected; the shell is a named dependency (CIP-80)"
        );
    }
    if (first_line == b"#!/usr/bin/env" || first_line.starts_with(b"#!/usr/bin/env "))
        && !output.join("bin/env").is_file()
    {
        bail!(
            "closed-root executable {executable} needs /usr/bin/env; COPY ${{pkgs.coreutils}}/bin/env /bin/env or provide another declared env implementation"
        );
    }
    Ok(())
}

fn resolve_item_program(
    output: &Path,
    executable: &Path,
    env: &BTreeMap<String, String>,
) -> Result<PathBuf> {
    clean_executable(executable)?;
    let path = env
        .get("PATH")
        .context("bare START requires a resolved PATH")?;
    let candidates = path
        .split(':')
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            let directory = Path::new(entry);
            if directory.starts_with("/nix/store") {
                directory.join(executable)
            } else {
                output
                    .join(directory.strip_prefix("/").unwrap_or(directory))
                    .join(executable)
            }
        });
    let mut fallback = None;
    for candidate in candidates {
        fallback.get_or_insert_with(|| candidate.clone());
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    fallback.with_context(|| {
        format!(
            "bare executable {} was not found on the service PATH {path:?}",
            executable.display()
        )
    })
}

fn interpolate(field: &str, input: &str, env: &BTreeMap<String, String>) -> Result<String> {
    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut copied_until = 0;
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        output.push_str(&input[copied_until..index]);
        index += 1;
        if index >= bytes.len() || !(bytes[index].is_ascii_alphabetic() || bytes[index] == b'_') {
            bail!("invalid environment interpolation in {field} argument {input:?}");
        }
        let start = index;
        index += 1;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let name = &input[start..index];
        output.push_str(
            env.get(name)
                .with_context(|| format!("environment variable {name:?} has no resolved value"))?,
        );
        copied_until = index;
    }
    output.push_str(&input[copied_until..]);
    Ok(output)
}

fn clean_executable(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        bail!(
            "executable path {} must not be empty or contain '.' or '..'",
            path.display()
        );
    }
    Ok(())
}

fn render(
    service_name: &str,
    argv: &[String],
    environment: &[(String, String)],
    properties: &[(String, String)],
    unit_properties: &[(String, String)],
) -> String {
    let mut output = format!(
        "[Unit]\nDescription={}\n",
        unit_value(&format!("cix run: {service_name}"))
    );
    for (name, value) in unit_properties {
        output.push_str(name);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    for (name, value) in properties
        .iter()
        .filter(|(name, _)| name.starts_with("StartLimit"))
    {
        output.push_str(name);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    output.push_str("\n[Service]\n");
    for (name, value) in properties {
        if name.starts_with("StartLimit") {
            continue;
        }
        output.push_str(name);
        output.push('=');
        output.push_str(value);
        output.push('\n');
    }
    output.push_str("ExecStart=");
    output.push_str(&exec_command(argv));
    output.push('\n');
    for (name, value) in environment {
        output.push_str("Environment=");
        output.push_str(&quote_unit_word(&format!("{name}={value}")));
        output.push('\n');
    }
    output
}

pub(crate) fn exec_command(argv: &[String]) -> String {
    argv.iter()
        .map(|value| quote_exec_word(value))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_exec_word(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '$' => quoted.push_str("$$"),
            '%' => quoted.push_str("%%"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn quote_unit_word(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '%' => quoted.push_str("%%"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

fn unit_value(value: &str) -> String {
    value.replace('%', "%%")
}

#[cfg(test)]
#[path = "unit/tests.rs"]
mod tests;
