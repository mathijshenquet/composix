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
    properties.push(("Slice".into(), options.naming.slice.clone()));
    let log_fields = options
        .log_fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .chain(std::iter::once(format!("CIX_ITEM={}", output.display())))
        .collect::<Vec<_>>()
        .join(" ");
    properties.push(("LogExtraFields".into(), log_fields));

    crate::directories::add_properties(
        &mut properties,
        &format!("{}-{service_name}", options.naming.directory_prefix),
        &service.dirs,
        mode != UnitMode::System,
        mode != UnitMode::UserDegraded,
    );

    if mode == UnitMode::System || options.closed_root.is_some() {
        add_mounts(&mut properties, output, service)?;
    }

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
            "closed-root executable {executable} needs /usr/bin/env; LINK ${{pkgs.coreutils}}/bin/env /bin/env or provide another declared env implementation"
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
mod tests {
    use crate::spec::{Service, Spec};

    use super::*;

    fn service(spec: &Spec) -> &Service {
        spec.select_service(None).unwrap().1
    }

    fn fixture() -> (Spec, ResolvedConfig) {
        let spec = Spec::from_slice(include_bytes!("../tests/fixtures/full-spec.json")).unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(
            service,
            &["DB_URL=postgres://db/a b".into(), "ENABLED=true".into()],
            &["http=9090".into()],
        )
        .unwrap();
        (spec, config)
    }

    #[test]
    fn closed_root_snapshots_cover_claims_dirs_materializations_and_modes() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"dirs":{"state":["/var/lib/app"],"cache":["/cache"],"logs":["/var/log/app"],"config":["/etc/app"],"run":["/run/app"]},"claims":["egress","gpu",{"device":"/dev/cix-device"}],"shm":"64M"}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        for mode in [UnitMode::System, UnitMode::UserFull] {
            let mut options = UnitCompileOptions::cix_run("audit");
            options.naming.unit = format!(
                "cix-audit-{}.service",
                if mode == UnitMode::System {
                    "system"
                } else {
                    "user"
                }
            );
            options.extra_properties = vec![
                ("SupplementaryGroups".into(), "cix-shared".into()),
                ("BindPaths".into(), "/srv/shared:/data".into()),
                ("BindReadOnlyPaths".into(), "/srv/input:/input".into()),
            ];
            if mode == UnitMode::System {
                options.extra_properties.extend([
                    ("DynamicUser".into(), "no".into()),
                    ("User".into(), "operator".into()),
                    ("Group".into(), "operator".into()),
                ]);
            }
            options.probe_binary =
                Some("/nix/store/11111111111111111111111111111111-cix/bin/cix".into());
            options.closed_root = Some(
                crate::closed_root::options_for_unit(&options.naming.unit, false)
                    .unwrap()
                    .with_identity_override("operator"),
            );
            let compiled = compile_unit_for_host(
                Path::new("/nix/store/00000000000000000000000000000000-app"),
                "audit",
                service,
                &config,
                mode,
                &options,
                &HostCapabilities::all_supported(),
            )
            .unwrap();
            let expected = match mode {
                UnitMode::System => include_str!("../tests/fixtures/closed-root-system.unit"),
                UnitMode::UserFull => include_str!("../tests/fixtures/closed-root-user.unit"),
                UnitMode::UserDegraded => unreachable!(),
            };
            assert_eq!(compiled.text, expected);
        }
    }

    #[test]
    fn pre_v257_closed_root_adds_explicit_journal_socket_binds() {
        let spec = Spec::from_slice(br#"{"cixManifest":0,"start":["bin/app"]}"#).unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let mut options = UnitCompileOptions::cix_run("compat");
        options.probe_binary =
            Some("/nix/store/11111111111111111111111111111111-cix/bin/cix".into());
        options.closed_root =
            Some(crate::closed_root::options_for_unit("cix-compat.service", false).unwrap());
        let compiled = compile_unit_for_host(
            Path::new("/nix/store/00000000000000000000000000000000-app"),
            "compat",
            service,
            &config,
            UnitMode::System,
            &options,
            &HostCapabilities::for_systemd_version(256),
        )
        .unwrap();
        for socket in [
            "/dev/log",
            "/run/systemd/journal/socket",
            "/run/systemd/journal/stdout",
        ] {
            assert!(compiled
                .properties
                .contains(&("BindReadOnlyPaths".into(), socket.into(),)));
        }
    }

    #[test]
    fn closed_root_teaches_explicit_shell_and_env_dependencies() {
        let error = resolved_argv(
            Path::new("/nix/store/00000000000000000000000000000000-app"),
            "start",
            &["/bin/sh".into()],
            &BTreeMap::new(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("name the shell explicitly"), "{error}");

        let output = tempfile::tempdir().unwrap();
        let executable = output.path().join("start");
        std::fs::write(&executable, "#!/usr/bin/env bash\n").unwrap();
        let error = validate_closed_root_executable(output.path(), executable.to_str().unwrap())
            .unwrap_err()
            .to_string();
        assert!(error.contains("LINK ${pkgs.coreutils}/bin/env"), "{error}");
        std::fs::create_dir_all(output.path().join("bin")).unwrap();
        std::fs::write(output.path().join("bin/env"), "env").unwrap();
        validate_closed_root_executable(output.path(), executable.to_str().unwrap()).unwrap();
    }

    #[test]
    fn closed_root_refuses_host_dependent_low_port_capabilities() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"ports":{"http":{"value":80,"protocol":"tcp"}}}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let mut options = UnitCompileOptions::cix_run("low-port");
        options.closed_root =
            Some(crate::closed_root::options_for_unit("cix-low-port.service", false).unwrap());
        let error = build_unit_with_options(
            Path::new("/nix/store/00000000000000000000000000000000-app"),
            "app",
            service,
            &config,
            UnitMode::System,
            &options,
            &HostCapabilities::all_supported(),
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "closed root cannot grant http port 80: PrivateUsers isolates capabilities from the host network namespace; use a port >= 1024 or a named LISTENER for systemd socket activation"
        );
    }

    #[test]
    fn bare_start_resolves_through_the_item_path() {
        let output = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output.path().join("bin")).unwrap();
        std::fs::write(output.path().join("bin/app"), "app").unwrap();
        let env = BTreeMap::from([("PATH".into(), "bin:/tools/bin".into())]);
        assert_eq!(
            resolve_item_program(output.path(), Path::new("app"), &env).unwrap(),
            output.path().join("bin/app")
        );
    }

    #[test]
    fn full_system_unit_matches_golden_file() {
        let (spec, config) = fixture();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service(&spec),
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert_eq!(actual, include_str!("../tests/fixtures/full-system.unit"));
    }

    #[test]
    fn secret_paths_are_projected_in_system_and_user_units() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"secrets":{"db-password":{"as":"DB_PASSWORD_FILE"},"api-key":{}}}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        for mode in [UnitMode::System, UnitMode::UserFull] {
            let unit = compile_unit(
                Path::new("/nix/store/00000000000000000000000000000000-app"),
                "app",
                service,
                &config,
                mode,
                &UnitCompileOptions::cix_run("app"),
            )
            .unwrap();
            assert!(
                unit.text
                    .contains("Environment=\"DB_PASSWORD_FILE=%d/db-password\""),
                "{}",
                unit.text
            );
            assert!(!unit.text.contains("DB_PASSWORD="), "{}", unit.text);
        }
    }

    #[test]
    fn health_property_snapshots_cover_every_probe_consumer_and_mode() {
        for consumer in ["readiness", "liveness"] {
            for probe_type in ["notify", "http", "tcp"] {
                for mode in [UnitMode::System, UnitMode::UserFull] {
                    let target = match probe_type {
                        "http" => r#", "target": ":8080/healthz""#,
                        "tcp" => r#", "target": ":5432""#,
                        "notify" => "",
                        _ => unreachable!(),
                    };
                    let duration = if consumer == "readiness" {
                        r#""timeout": "90s""#
                    } else {
                        r#""interval": "10s""#
                    };
                    let spec = Spec::from_slice(
                        format!(
                            r#"{{"cixManifest":0,"start":["bin/app"],"{consumer}":{{"type":"{probe_type}"{target},{duration}}}}}"#
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                    let service = service(&spec);
                    let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
                    let mut options = UnitCompileOptions::cix_run("app");
                    options.probe_binary =
                        Some("/nix/store/11111111111111111111111111111111-cix/bin/cix".into());
                    let compiled = compile_unit(
                        Path::new("/nix/store/00000000000000000000000000000000-app"),
                        "app",
                        service,
                        &config,
                        mode,
                        &options,
                    )
                    .unwrap();
                    let actual = compiled
                        .properties
                        .iter()
                        .filter(|(name, _)| {
                            matches!(
                                name.as_str(),
                                "Type"
                                    | "ExecStartPost"
                                    | "TimeoutStartSec"
                                    | "TimeoutStopSec"
                                    | "WatchdogSec"
                                    | "NotifyAccess"
                                    | "Restart"
                                    | "RestartSec"
                                    | "StartLimitIntervalSec"
                                    | "StartLimitBurst"
                            )
                        })
                        .map(|(name, value)| format!("{name}={value}\n"))
                        .collect::<String>();
                    let expected = health_snapshot(consumer, probe_type, mode);
                    assert_eq!(actual, expected, "{consumer}/{probe_type}/{mode:?}");
                }
            }
        }
    }

    fn health_snapshot(consumer: &str, probe_type: &str, mode: UnitMode) -> &'static str {
        match (consumer, probe_type, mode) {
            ("readiness", "notify", UnitMode::System) => {
                include_str!("../tests/fixtures/health-readiness-notify-system.unit")
            }
            ("readiness", "notify", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-readiness-notify-user.unit")
            }
            ("readiness", "http", UnitMode::System) => {
                include_str!("../tests/fixtures/health-readiness-http-system.unit")
            }
            ("readiness", "http", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-readiness-http-user.unit")
            }
            ("readiness", "tcp", UnitMode::System) => {
                include_str!("../tests/fixtures/health-readiness-tcp-system.unit")
            }
            ("readiness", "tcp", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-readiness-tcp-user.unit")
            }
            ("liveness", "notify", UnitMode::System) => {
                include_str!("../tests/fixtures/health-liveness-notify-system.unit")
            }
            ("liveness", "notify", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-liveness-notify-user.unit")
            }
            ("liveness", "http", UnitMode::System) => {
                include_str!("../tests/fixtures/health-liveness-http-system.unit")
            }
            ("liveness", "http", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-liveness-http-user.unit")
            }
            ("liveness", "tcp", UnitMode::System) => {
                include_str!("../tests/fixtures/health-liveness-tcp-system.unit")
            }
            ("liveness", "tcp", UnitMode::UserFull) => {
                include_str!("../tests/fixtures/health-liveness-tcp-user.unit")
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn restart_policy_is_emitted_only_for_liveness_declarations() {
        for (field, has_restart) in [
            ("", false),
            (r#", "readiness":{"type":"notify","timeout":"10s"}"#, false),
            (r#", "liveness":{"type":"notify","interval":"10s"}"#, true),
        ] {
            let spec = Spec::from_slice(
                format!(r#"{{"cixManifest":0,"start":["bin/app"]{field}}}"#).as_bytes(),
            )
            .unwrap();
            let service = service(&spec);
            let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
            let compiled = compile_unit(
                Path::new("/nix/store/00000000000000000000000000000000-app"),
                "app",
                service,
                &config,
                UnitMode::System,
                &UnitCompileOptions::cix_run("app"),
            )
            .unwrap();
            assert_eq!(
                compiled
                    .properties
                    .iter()
                    .any(|(name, _)| name == "Restart"),
                has_restart
            );
            assert_eq!(
                compiled
                    .properties
                    .iter()
                    .any(|(name, _)| name.starts_with("StartLimit")),
                has_restart
            );
        }
    }

    #[test]
    fn no_declared_network_is_private() {
        let spec = Spec::from_slice(include_bytes!("../tests/fixtures/minimal-spec.json")).unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-worker"),
            "worker",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert_eq!(
            actual,
            include_str!("../tests/fixtures/minimal-system.unit")
        );
        assert!(actual.contains("PrivateNetwork=yes"));
        assert!(actual.contains("PrivatePIDs=yes"));
    }

    #[test]
    fn unsupported_host_drops_private_pids_for_persistent_directories_once() {
        let (spec, config) = fixture();
        let capabilities = HostCapabilities::private_pids_with_persistent_directories_unsupported(
            "synthetic realization failure",
        );
        let compiled = compile_unit_for_host(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service(&spec),
            &config,
            UnitMode::System,
            &UnitCompileOptions::cix_run("web"),
            &capabilities,
        )
        .unwrap();

        assert!(!compiled.text.contains("PrivatePIDs="));
        assert!(compiled.text.contains("StateDirectoryMode=0733"));
        assert_eq!(
            compiled.degradations,
            vec![UnitDegradation {
                property: "PrivatePIDs=yes".into(),
                reason: "synthetic realization failure".into(),
            }]
        );
    }

    #[test]
    fn capable_host_preserves_private_pids_for_persistent_directories() {
        let (spec, config) = fixture();
        let compiled = compile_unit_for_host(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service(&spec),
            &config,
            UnitMode::System,
            &UnitCompileOptions::cix_run("web"),
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        assert!(compiled.text.contains("PrivatePIDs=yes"));
        assert!(compiled.text.contains("StateDirectoryMode=0700"));
        assert!(compiled.degradations.is_empty());
    }

    #[test]
    fn unsupported_user_host_drops_private_devices_once() {
        let (spec, config) = fixture();
        let capabilities = HostCapabilities::user_private_devices_unsupported(
            "synthetic user-manager realization failure",
        );
        let compiled = compile_unit_for_host(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service(&spec),
            &config,
            UnitMode::UserFull,
            &UnitCompileOptions::cix_run("web"),
            &capabilities,
        )
        .unwrap();

        assert!(!compiled.text.contains("PrivateDevices="));
        assert_eq!(
            compiled.degradations,
            vec![UnitDegradation {
                property: "PrivateDevices=yes".into(),
                reason: "synthetic user-manager realization failure".into(),
            }]
        );
    }

    #[test]
    fn runtime_directories_do_not_trigger_persistent_directory_fallback() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/worker"],"dirs":{"run":["/run/worker"]}}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let capabilities = HostCapabilities::private_pids_with_persistent_directories_unsupported(
            "synthetic realization failure",
        );
        let compiled = compile_unit_for_host(
            Path::new("/nix/store/00000000000000000000000000000000-worker"),
            "worker",
            service,
            &config,
            UnitMode::System,
            &UnitCompileOptions::cix_run("worker"),
            &capabilities,
        )
        .unwrap();

        assert!(compiled.text.contains("PrivatePIDs=yes"));
        assert!(compiled.degradations.is_empty());
    }

    #[test]
    fn system_units_project_existing_mounts_without_cix_app() {
        let output = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output.path().join("etc/nginx")).unwrap();
        std::fs::write(output.path().join("etc/nginx/nginx.conf"), "events {}\n").unwrap();
        std::fs::write(output.path().join("cix-probe.conf"), "probe\n").unwrap();
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"mounts":["/etc/nginx","/cix-probe.conf"]}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let definition =
            build_unit(output.path(), "worker", service, &config, UnitMode::System).unwrap();

        assert!(definition.properties.contains(&(
            "BindReadOnlyPaths".into(),
            format!("{}/etc/nginx:/etc/nginx", output.path().display()),
        )));
        assert!(definition.properties.contains(&(
            "BindReadOnlyPaths".into(),
            format!("{}/cix-probe.conf:/cix-probe.conf", output.path().display()),
        )));
        assert!(!definition
            .environment
            .iter()
            .any(|(name, _)| name == "CIX_APP"));

        let user_definition = build_unit(
            output.path(),
            "worker",
            service,
            &config,
            UnitMode::UserFull,
        )
        .unwrap();
        assert!(!user_definition
            .properties
            .iter()
            .any(|(name, _)| name == "BindReadOnlyPaths"));
        assert!(user_definition.environment.contains(&(
            "CIX_APP".into(),
            output.path().to_string_lossy().into_owned(),
        )));

        let mut closed_options = UnitCompileOptions::cix_run("worker");
        closed_options.probe_binary =
            Some("/nix/store/11111111111111111111111111111111-cix/bin/cix".into());
        closed_options.closed_root = Some(
            crate::closed_root::options_for_unit("cix-worker-user.service", false)
                .unwrap()
                .with_identity_override("operator"),
        );
        let closed_user = build_unit_with_options(
            output.path(),
            "worker",
            service,
            &config,
            UnitMode::UserFull,
            &closed_options,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        assert!(closed_user.properties.contains(&(
            "BindReadOnlyPaths".into(),
            format!("{}/etc/nginx:/etc/nginx", output.path().display()),
        )));
        assert!(closed_user.properties.contains(&(
            "BindReadOnlyPaths".into(),
            format!("{}/cix-probe.conf:/cix-probe.conf", output.path().display()),
        )));

        let degraded_definition = build_unit(
            output.path(),
            "worker",
            service,
            &config,
            UnitMode::UserDegraded,
        )
        .unwrap();
        assert!(user_definition
            .properties
            .iter()
            .any(|(name, value)| name == "PrivatePIDs" && value == "yes"));
        assert!(!degraded_definition
            .properties
            .iter()
            .any(|(name, _)| name == "PrivatePIDs"));
    }

    #[test]
    fn refuses_a_declared_mount_missing_from_the_store_item() {
        let output = tempfile::tempdir().unwrap();
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"mounts":["/opt/a/b/c/d"]}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let error = build_unit(output.path(), "worker", service, &config, UnitMode::System)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("declared mount /opt/a/b/c/d is missing"),
            "{error}"
        );
    }

    #[test]
    fn v2_system_unit_matches_golden_file() {
        let spec = Spec::from_slice(include_bytes!("../tests/fixtures/v2-spec.json")).unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web-v2"),
            "web",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert_eq!(actual, include_str!("../tests/fixtures/v2-system.unit"));
        assert!(!actual.contains("TemporaryFileSystem=/run"));
        assert!(!actual.contains("MemoryDenyWriteExecute"));
    }

    #[test]
    fn jit_claim_drops_memory_deny_write_execute() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"claims":["jit"]}"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-worker"),
            "worker",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert!(!actual.contains("MemoryDenyWriteExecute"), "{actual}");
    }

    #[test]
    fn device_claims_replace_private_devices_with_a_closed_allow_list() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"claims":["gpu",{"device":"/dev/null"}],"shm":"128M"}"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-worker"),
            "worker",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        for expected in [
            "DevicePolicy=closed",
            "DeviceAllow=/dev/dri rwm",
            "DeviceAllow=/dev/null rwm",
            "SupplementaryGroups=render root video",
            "TemporaryFileSystem=/dev/shm:size=128M",
        ] {
            assert!(actual.contains(expected), "missing {expected} in {actual}");
        }
        assert!(!actual.contains("PrivateDevices="), "{actual}");
    }

    #[test]
    fn ordinary_units_keep_private_devices() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["/nix/store/00000000000000000000000000000000-worker/bin/worker"]}"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-worker"),
            "worker",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert!(actual.contains("PrivateDevices=yes"), "{actual}");
    }

    #[test]
    fn item_bin_default_is_projected_into_the_run_unit_environment() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"env":{"PATH":{"default":"bin"}}}"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let output = Path::new("/nix/store/00000000000000000000000000000000-app");
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let definition = build_unit(output, "app", service, &config, UnitMode::System).unwrap();

        assert_eq!(
            definition.argv,
            [output.join("bin/app").to_string_lossy().into_owned()]
        );
        assert!(definition.environment.contains(&(
            "PATH".into(),
            output.join("bin").to_string_lossy().into_owned(),
        )));
    }

    #[test]
    fn v3_listener_unit_keeps_network_private_and_denies_binds() {
        let spec =
            Spec::from_slice(include_bytes!("../tests/fixtures/v3-listener-spec.json")).unwrap();
        let service = service(&spec);
        let config =
            ResolvedConfig::resolve(service, &[], &["http=127.0.0.1:8080".into()]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert_eq!(
            actual,
            include_str!("../tests/fixtures/v3-listener-system.unit")
        );
        assert!(actual.contains("PrivateNetwork=yes"));
        assert!(actual.contains("RestrictAddressFamilies=AF_UNIX"));
        assert!(actual.contains("CapabilityBoundingSet=\n"));
        assert!(actual.contains("SocketBindDeny=any"));
        assert!(!actual.contains("SocketBindAllow="));
    }

    #[test]
    fn ports_and_listeners_compile_independent_socket_capabilities() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/web"],
                "ports": {
                    "http": {"value": 8080, "protocol": "tcp"},
                    "dns": {"value": 5353, "protocol": "udp"}
                },
                "listeners": {"admin": {"type": "stream"}}
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        let config =
            ResolvedConfig::resolve(service, &[], &["admin=127.0.0.1:9090".into()]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert!(actual.contains("RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6"));
        assert!(actual.contains("SocketBindAllow=tcp:8080"));
        assert!(actual.contains("SocketBindAllow=udp:5353"));
        assert!(actual.contains("SocketBindDeny=any"));
    }

    #[test]
    fn public_compiler_accepts_foreign_names_and_extra_properties() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/web"],"dirs":{"state":["/var/lib/web"]}}"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let compiled = compile_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service,
            &config,
            UnitMode::System,
            &UnitCompileOptions {
                naming: UnitNaming {
                    unit: "cix-mycomp-web.service".into(),
                    slice: "cix-mycomp.slice".into(),
                    target: "cix-mycomp.target".into(),
                    directory_prefix: "cix-mycomp".into(),
                },
                extra_properties: vec![("SupplementaryGroups".into(), "cix-edge".into())],
                unit_properties: Vec::new(),
                log_fields: vec![
                    ("CIX_COMPOSITE".into(), "mycomp".into()),
                    ("CIX_SERVICE".into(), "web".into()),
                ],
                probe_binary: None,
                closed_root: None,
            },
        )
        .unwrap();
        assert_eq!(compiled.name, "cix-mycomp-web.service");
        assert_eq!(compiled.target, "cix-mycomp.target");
        assert!(compiled.text.contains(
            "LogExtraFields=CIX_COMPOSITE=mycomp CIX_SERVICE=web CIX_ITEM=/nix/store/00000000000000000000000000000000-web"
        ));
        assert!(compiled.text.contains("Slice=cix-mycomp.slice"));
        assert!(compiled
            .text
            .contains("StateDirectory=cix-mycomp-web cix-mycomp-web/var/lib/web"));
        assert!(compiled.text.contains("SupplementaryGroups=cix-edge"));
    }

    #[test]
    fn env_default_and_override_low_ports_claim_bind_capability() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/web"],
                "env": {"PORT": {"default": "80"}},
                "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        for config in [
            ResolvedConfig::resolve(service, &[], &[]).unwrap(),
            ResolvedConfig::resolve(service, &[], &["http=81".into()]).unwrap(),
        ] {
            let actual = generate_unit(
                Path::new("/nix/store/00000000000000000000000000000000-web"),
                "web",
                service,
                &config,
                UnitMode::System,
            )
            .unwrap();
            assert!(actual.contains("AmbientCapabilities=CAP_NET_BIND_SERVICE"));
            assert!(actual.contains("CapabilityBoundingSet=CAP_NET_BIND_SERVICE"));
        }
    }

    #[test]
    fn high_default_overridden_to_low_port_claims_bind_capability() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/web"],
                "env": {"PORT": {"default": "8080"}},
                "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        let config =
            ResolvedConfig::resolve(service, &[], &["http=80".into()]).expect("valid override");
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-web"),
            "web",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert!(actual.contains("AmbientCapabilities=CAP_NET_BIND_SERVICE"));
        assert!(actual.contains("CapabilityBoundingSet=CAP_NET_BIND_SERVICE"));
    }

    #[test]
    fn refuses_an_executable_that_escapes_the_store_output() {
        let spec = Spec::from_slice(br#"{"cixManifest":0,"start":["../bin/x"]}"#).unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        assert!(generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-x"),
            "x",
            service,
            &config,
            UnitMode::System
        )
        .is_err());
    }

    #[test]
    fn system_role_paths_use_full_mirror_binds_and_in_namespace_environment() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/database"],
                "dirs": {
                    "state": ["/var/lib/database"],
                    "cache": ["/var/cache/database"],
                    "logs": ["/var/log/database"]
                }
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-database"),
            "database",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        for expected in [
            "TemporaryFileSystem=/var/lib:ro",
            "StateDirectory=cix-run-database cix-run-database/var/lib/database",
            "BindPaths=/var/lib/cix-run-database/var/lib/database:/var/lib/database",
            "TemporaryFileSystem=/var/cache:ro",
            "CacheDirectory=cix-run-database cix-run-database/var/cache/database",
            "BindPaths=/var/cache/cix-run-database/var/cache/database:/var/cache/database",
            "TemporaryFileSystem=/var/log:ro",
            "LogsDirectory=cix-run-database cix-run-database/var/log/database",
            "BindPaths=/var/log/cix-run-database/var/log/database:/var/log/database",
            "Environment=\"STATE_DIRECTORY=/var/lib/database\"",
            "Environment=\"CACHE_DIRECTORY=/var/cache/database\"",
            "Environment=\"LOGS_DIRECTORY=/var/log/database\"",
        ] {
            assert!(
                actual.contains(expected),
                "missing {expected:?} in:\n{actual}"
            );
        }
    }

    #[test]
    fn arbitrary_and_multiple_role_paths_are_fully_mirrored() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/app"],
                "dirs": {
                    "state": ["/srv/app/state", "/var/lib/app-extra"],
                    "cache": ["/app/cache"],
                    "logs": ["/app/logs", "/var/log/app-extra"],
                    "run": ["/tmp/app/run"]
                }
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let actual = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-app"),
            "app",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap();
        for expected in [
            "StateDirectory=cix-run-app cix-run-app/srv/app/state cix-run-app/var/lib/app-extra",
            "BindPaths=/var/lib/cix-run-app/srv/app/state:/srv/app/state",
            "BindPaths=/var/lib/cix-run-app/var/lib/app-extra:/var/lib/app-extra",
            "CacheDirectory=cix-run-app cix-run-app/app/cache",
            "BindPaths=/var/cache/cix-run-app/app/cache:/app/cache",
            "LogsDirectory=cix-run-app cix-run-app/app/logs cix-run-app/var/log/app-extra",
            "BindPaths=/var/log/cix-run-app/app/logs:/app/logs",
            "RuntimeDirectory=cix-run-app/tmp/app/run",
            "BindPaths=/run/cix-run-app/tmp/app/run:/tmp/app/run",
            "TemporaryFileSystem=/app:ro",
            "TemporaryFileSystem=/srv:ro",
            "TemporaryFileSystem=/tmp:ro",
            "Environment=\"STATE_DIRECTORY=/srv/app/state:/var/lib/app-extra\"",
            "Environment=\"LOGS_DIRECTORY=/app/logs:/var/log/app-extra\"",
            "Environment=\"RUNTIME_DIRECTORY=/tmp/app/run\"",
        ] {
            assert!(
                actual.contains(expected),
                "missing {expected:?} in:\n{actual}"
            );
        }
        assert!(
            !actual.contains(":app-extra"),
            "legacy aliases leaked into:\n{actual}"
        );
        assert!(
            !actual.contains("state-0"),
            "legacy indexes leaked into:\n{actual}"
        );
    }

    #[test]
    fn dir_without_compose_materialization_has_the_teaching_error() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/app"],
                "dirs": {"data": [{"path": "/media", "ro": true}]}
            }"#,
        )
        .unwrap();
        let service = service(&spec);
        let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        let error = generate_unit(
            Path::new("/nix/store/00000000000000000000000000000000-app"),
            "app",
            service,
            &config,
            UnitMode::System,
        )
        .unwrap_err()
        .to_string();
        assert_eq!(
            error,
            "DIR declares operator-supplied data; materialization arrives with compose (docs/cixfile.md#role-dirs); for a cix-managed dir pick a role: STATEDIR/CACHEDIR/LOGDIR/RUNDIR"
        );
    }
}
