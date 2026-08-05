use crate::spec::{Service, Spec};

use super::*;

fn service(spec: &Spec) -> &Service {
    spec.select_service(None).unwrap().1
}

fn fixture() -> (Spec, ResolvedConfig) {
    let spec = Spec::from_slice(include_bytes!("../../tests/fixtures/full-spec.json")).unwrap();
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
            UnitMode::System => include_str!("../../tests/fixtures/closed-root-system.unit"),
            UnitMode::UserFull => include_str!("../../tests/fixtures/closed-root-user.unit"),
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
    options.probe_binary = Some("/nix/store/11111111111111111111111111111111-cix/bin/cix".into());
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
    assert_eq!(
        actual,
        include_str!("../../tests/fixtures/full-system.unit")
    );
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
fn stop_signal_projects_to_kill_signal() {
    let spec = Spec::from_slice(br#"{"cixManifest":0,"start":["bin/app"],"stopSignal":"SIGQUIT"}"#)
        .unwrap();
    let service = service(&spec);
    let config = ResolvedConfig::resolve(service, &[], &[]).unwrap();
    let unit = generate_unit(
        Path::new("/nix/store/00000000000000000000000000000000-app"),
        "app",
        service,
        &config,
        UnitMode::System,
    )
    .unwrap();
    assert!(unit.contains("KillSignal=SIGQUIT"), "{unit}");
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
            include_str!("../../tests/fixtures/health-readiness-notify-system.unit")
        }
        ("readiness", "notify", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-readiness-notify-user.unit")
        }
        ("readiness", "http", UnitMode::System) => {
            include_str!("../../tests/fixtures/health-readiness-http-system.unit")
        }
        ("readiness", "http", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-readiness-http-user.unit")
        }
        ("readiness", "tcp", UnitMode::System) => {
            include_str!("../../tests/fixtures/health-readiness-tcp-system.unit")
        }
        ("readiness", "tcp", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-readiness-tcp-user.unit")
        }
        ("liveness", "notify", UnitMode::System) => {
            include_str!("../../tests/fixtures/health-liveness-notify-system.unit")
        }
        ("liveness", "notify", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-liveness-notify-user.unit")
        }
        ("liveness", "http", UnitMode::System) => {
            include_str!("../../tests/fixtures/health-liveness-http-system.unit")
        }
        ("liveness", "http", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-liveness-http-user.unit")
        }
        ("liveness", "tcp", UnitMode::System) => {
            include_str!("../../tests/fixtures/health-liveness-tcp-system.unit")
        }
        ("liveness", "tcp", UnitMode::UserFull) => {
            include_str!("../../tests/fixtures/health-liveness-tcp-user.unit")
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
    let spec = Spec::from_slice(include_bytes!("../../tests/fixtures/minimal-spec.json")).unwrap();
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
        include_str!("../../tests/fixtures/minimal-system.unit")
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
    let spec = Spec::from_slice(include_bytes!("../../tests/fixtures/v2-spec.json")).unwrap();
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
    assert_eq!(actual, include_str!("../../tests/fixtures/v2-system.unit"));
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
        Spec::from_slice(include_bytes!("../../tests/fixtures/v3-listener-spec.json")).unwrap();
    let service = service(&spec);
    let config = ResolvedConfig::resolve(service, &[], &["http=127.0.0.1:8080".into()]).unwrap();
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
        include_str!("../../tests/fixtures/v3-listener-system.unit")
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
    let config = ResolvedConfig::resolve(service, &[], &["admin=127.0.0.1:9090".into()]).unwrap();
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
                    "config": ["/config/app"],
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
        "ConfigurationDirectory=cix-run-app cix-run-app/config/app",
        "BindPaths=/etc/cix-run-app/config/app:/config/app",
        "RuntimeDirectory=cix-run-app/tmp/app/run",
        "BindPaths=/run/cix-run-app/tmp/app/run:/tmp/app/run",
        "TemporaryFileSystem=/app:ro",
        "TemporaryFileSystem=/srv:ro",
        "TemporaryFileSystem=/tmp:ro",
        "Environment=\"STATE_DIRECTORY=/srv/app/state:/var/lib/app-extra\"",
        "Environment=\"CONFIGURATION_DIRECTORY=/config/app\"",
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
