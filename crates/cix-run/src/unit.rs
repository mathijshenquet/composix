use std::collections::BTreeMap;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};

use crate::config::ResolvedConfig;
use crate::spec::{Dirs, Service};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitMode {
    System,
    UserFull,
    UserDegraded,
}

#[derive(Debug, Clone)]
pub(crate) struct UnitDefinition {
    pub text: String,
    pub properties: Vec<(String, String)>,
    pub environment: Vec<(String, String)>,
    pub argv: Vec<String>,
}

pub fn generate_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
) -> Result<String> {
    Ok(build_unit(output, service_name, service, config, mode)?.text)
}

pub(crate) fn build_unit(
    output: &Path,
    service_name: &str,
    service: &Service,
    config: &ResolvedConfig,
    mode: UnitMode,
) -> Result<UnitDefinition> {
    if !output.is_absolute() {
        bail!("store output path {} is not absolute", output.display());
    }

    let argv = resolved_argv(output, "exec", &service.exec, &config.env)?;
    let mut properties = Vec::new();
    properties.push(("Type".into(), "exec".into()));
    properties.push(("Slice".into(), "cix-run.slice".into()));

    add_directories(
        &mut properties,
        service_name,
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
    if service.jit != Some(true) {
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
    if let Some(setup) = &service.setup {
        let setup = resolved_argv(output, "setup", setup, &config.env)?;
        properties.push(("ExecStartPre".into(), exec_command(&setup)));
    }

    let mut environment = config
        .env
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

    let text = render(service_name, &argv, &environment, &properties);
    Ok(UnitDefinition {
        text,
        properties,
        environment,
        argv,
    })
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
    exec: &[String],
    env: &BTreeMap<String, String>,
) -> Result<Vec<String>> {
    let mut argv = exec
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
    service_name: &str,
    dirs: &Dirs,
    user: bool,
    bind: bool,
) {
    let managed_base = format!("cix-run-{service_name}");
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
        let managed = managed_names(&managed_base, role, paths.len());
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
    use crate::spec::Spec;

    use super::*;

    fn fixture() -> (Spec, ResolvedConfig) {
        let spec = Spec::from_slice(include_bytes!("../tests/fixtures/full-spec.json")).unwrap();
        let service = &spec.services["web"];
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
            &spec.services["web"],
            &config,
            UnitMode::System,
        )
        .unwrap();
        assert_eq!(actual, include_str!("../tests/fixtures/full-system.unit"));
    }

    #[test]
    fn no_declared_network_is_private() {
        let spec = Spec::from_slice(include_bytes!("../tests/fixtures/minimal-spec.json")).unwrap();
        let service = &spec.services["worker"];
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
    }

    #[test]
    fn system_units_project_existing_mounts_without_cix_app() {
        let output = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(output.path().join("etc/nginx")).unwrap();
        std::fs::write(output.path().join("etc/nginx/nginx.conf"), "events {}\n").unwrap();
        std::fs::write(output.path().join("cix-probe.conf"), "probe\n").unwrap();
        let spec = Spec::from_slice(
            br#"{"cixSpec":2,"services":{"worker":{"exec":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"mounts":["/etc/nginx","/cix-probe.conf"]}}}"#,
        )
        .unwrap();
        let service = &spec.services["worker"];
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
    }

    #[test]
    fn refuses_a_declared_mount_missing_from_the_store_item() {
        let output = tempfile::tempdir().unwrap();
        let spec = Spec::from_slice(
            br#"{"cixSpec":2,"services":{"worker":{"exec":["/nix/store/00000000000000000000000000000000-worker/bin/worker"],"mounts":["/opt/a/b/c/d"]}}}"#,
        )
        .unwrap();
        let service = &spec.services["worker"];
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
        let service = &spec.services["web"];
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
    fn env_default_and_override_low_ports_grant_bind_capability() {
        let spec = Spec::from_slice(
            br#"{
                "cixSpec": 2,
                "services": {
                    "web": {
                        "exec": ["bin/web"],
                        "env": {"PORT": {"default": "80"}},
                        "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
                    }
                }
            }"#,
        )
        .unwrap();
        let service = &spec.services["web"];
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
    fn high_default_overridden_to_low_port_grants_bind_capability() {
        let spec = Spec::from_slice(
            br#"{
                "cixSpec": 2,
                "services": {
                    "web": {
                        "exec": ["bin/web"],
                        "env": {"PORT": {"default": "8080"}},
                        "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
                    }
                }
            }"#,
        )
        .unwrap();
        let service = &spec.services["web"];
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
        let spec =
            Spec::from_slice(br#"{"cixSpec":1,"services":{"x":{"exec":["../bin/x"]}}}"#).unwrap();
        let service = &spec.services["x"];
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
                "cixSpec": 1,
                "services": {
                    "database": {
                        "exec": ["bin/database"],
                        "dirs": {
                            "state": ["/var/lib/database"],
                            "cache": ["/var/cache/database"],
                            "logs": ["/var/log/database"]
                        }
                    }
                }
            }"#,
        )
        .unwrap();
        let service = &spec.services["database"];
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
