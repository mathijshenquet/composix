use std::collections::BTreeMap;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};

use crate::capabilities::HostCapabilities;
use crate::config::ResolvedConfig;
use crate::spec::{Dirs, Protocol, Service};

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
}

impl UnitCompileOptions {
    /// Return the default cix-run options for `service_name`.
    pub fn cix_run(service_name: &str) -> Self {
        Self {
            naming: UnitNaming::cix_run(service_name),
            extra_properties: Vec::new(),
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

    let item_env = config.item_environment(output)?;
    let argv = resolved_argv(output, "start", &service.start, &item_env)?;
    let mut properties = Vec::new();
    properties.push(("Type".into(), "exec".into()));
    properties.push(("Slice".into(), options.naming.slice.clone()));

    add_directories(
        &mut properties,
        &format!("{}-{service_name}", options.naming.directory_prefix),
        &service.dirs,
        mode != UnitMode::System,
        mode != UnitMode::UserDegraded,
    );

    if mode == UnitMode::System {
        add_mounts(&mut properties, output, service)?;
    }

    if mode == UnitMode::System {
        properties.push(("DynamicUser".into(), "yes".into()));
    } else if mode == UnitMode::UserFull {
        properties.push(("PrivateUsers".into(), "yes".into()));
    }

    if mode != UnitMode::UserDegraded {
        properties.push(("ProtectSystem".into(), "strict".into()));
        properties.push(("ProtectHome".into(), "yes".into()));
        properties.push(("PrivateTmp".into(), "yes".into()));
        properties.push(("PrivatePIDs".into(), "yes".into()));
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
    if let Some(start_pre) = &service.start_pre {
        let start_pre = resolved_argv(output, "start_pre", start_pre, &item_env)?;
        properties.push(("ExecStartPre".into(), exec_command(&start_pre)));
    }

    let mut environment = item_env
        .iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect::<Vec<_>>();
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
    let text = render(service_name, &argv, &environment, &properties);
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

    if mode != UnitMode::System
        || !properties
            .iter()
            .any(|(name, value)| name == "DynamicUser" && value == "yes")
        || !properties
            .iter()
            .any(|(name, value)| name == "PrivatePIDs" && value == "yes")
        || !properties
            .iter()
            .any(|(name, _)| PERSISTENT_DIRECTORIES.contains(&name.as_str()))
    {
        return Vec::new();
    }

    let Some(reason) = capabilities
        .private_pids_with_persistent_directories
        .unsupported_reason()
    else {
        return Vec::new();
    };
    properties.retain(|(name, _)| name != "PrivatePIDs");
    vec![UnitDegradation {
        property: "PrivatePIDs=yes".into(),
        reason: reason.into(),
    }]
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
    let resolved = if executable.is_absolute() {
        clean_executable(executable)?;
        executable.to_owned()
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

fn add_directories(
    properties: &mut Vec<(String, String)>,
    managed_base: &str,
    dirs: &Dirs,
    user: bool,
    bind: bool,
) {
    for (role, paths, directive, mode_directive, system_root, user_root) in [
        (
            "state",
            dirs.state.as_slice(),
            "StateDirectory",
            "StateDirectoryMode",
            "/var/lib",
            "%S",
        ),
        (
            "cache",
            dirs.cache.as_slice(),
            "CacheDirectory",
            "CacheDirectoryMode",
            "/var/cache",
            "%C",
        ),
        (
            "logs",
            dirs.logs.as_slice(),
            "LogsDirectory",
            "LogsDirectoryMode",
            "/var/log",
            "%L",
        ),
        (
            "config",
            dirs.config.as_slice(),
            "ConfigurationDirectory",
            "ConfigurationDirectoryMode",
            "/etc",
            "%E",
        ),
        (
            "run",
            dirs.run.as_deref().unwrap_or_default(),
            "RuntimeDirectory",
            "RuntimeDirectoryMode",
            "/run",
            "%t",
        ),
    ] {
        if paths.is_empty() {
            continue;
        }
        let managed = managed_names(managed_base, role, paths.len());
        let use_directory_aliases = !user && bind && role != "config";
        let mut directory_values = Vec::with_capacity(paths.len());
        let mut bind_values = Vec::new();
        let mut needs_private_role_root = false;
        for (source, destination) in managed.iter().zip(paths) {
            let relative_destination = use_directory_aliases
                .then(|| destination.strip_prefix(system_root).ok())
                .flatten()
                .filter(|path| !path.as_os_str().is_empty());
            if let Some(relative_destination) = relative_destination {
                needs_private_role_root = true;
                directory_values.push(format!(
                    "{source}:{}",
                    relative_destination.to_string_lossy().replace('%', "%%")
                ));
            } else {
                directory_values.push(source.clone());
                if bind {
                    let root = if user { user_root } else { system_root };
                    bind_values.push(format!(
                        "{root}/{source}:{}",
                        destination.to_string_lossy().replace('%', "%%")
                    ));
                }
            }
        }
        if needs_private_role_root && role != "run" {
            properties.push(("TemporaryFileSystem".into(), format!("{system_root}:ro")));
        }
        properties.push((directive.into(), directory_values.join(" ")));
        properties.push((mode_directive.into(), "0700".into()));
        for value in bind_values {
            properties.push(("BindPaths".into(), value));
        }
    }
}

fn managed_names(base: &str, role: &str, count: usize) -> Vec<String> {
    if count == 1 {
        vec![base.to_owned()]
    } else {
        (0..count)
            .map(|index| format!("{base}/{role}-{index}"))
            .collect()
    }
}

fn render(
    service_name: &str,
    argv: &[String],
    environment: &[(String, String)],
    properties: &[(String, String)],
) -> String {
    let mut output = format!(
        "[Unit]\nDescription={}\n\n[Service]\n",
        unit_value(&format!("cix run: {service_name}"))
    );
    for (name, value) in properties {
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

fn exec_command(argv: &[String]) -> String {
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
        assert!(compiled.degradations.is_empty());
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
            },
        )
        .unwrap();
        assert_eq!(compiled.name, "cix-mycomp-web.service");
        assert_eq!(compiled.target, "cix-mycomp.target");
        assert!(compiled.text.contains("Slice=cix-mycomp.slice"));
        assert!(compiled.text.contains("StateDirectory=cix-mycomp-web:web"));
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
    fn system_role_paths_preserve_dynamic_user_id_mapping() {
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
            "StateDirectory=cix-run-database:database",
            "TemporaryFileSystem=/var/cache:ro",
            "CacheDirectory=cix-run-database:database",
            "TemporaryFileSystem=/var/log:ro",
            "LogsDirectory=cix-run-database:database",
        ] {
            assert!(
                actual.contains(expected),
                "missing {expected:?} in:\n{actual}"
            );
        }
        assert!(!actual.contains("BindPaths="));
    }
}
