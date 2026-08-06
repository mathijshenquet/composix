use std::collections::BTreeSet;

use cix_cixfile::*;

#[test]
fn parses_blocks_binders_and_both_artifact_kinds() {
    let parsed = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
FETCH ingredient ${pkgs.coreutils}/bin/printf payload
BUILDER build
IMPORT ${pkgs.bash}
COPY Cargo.toml Cargo.toml
FETCH printf fetched > fetched
RUN cp fetched built
SERVICE web
COPY ${build}/built /bin/web
FILE /etc/app.conf <<E
source=${src}
E
	COPY ${pkgs.bash}/bin/bash /bin/sh
	ENV PATH=bin
START web
START_PRE /bin/web
ENV PORT required
PORT http = $PORT
LISTENER admin
STATEDIR /var/lib/web
	CACHEDIR /var/cache/web
LOGDIR /var/log/web
CONFIGDIR /etc/web
RUNDIR /run/web
CLAIM jit
CLAIM egress
APP migrate
COPY ${ingredient} /payload
START /bin/true
ENV MODE=once
STATEDIR /var/lib/migrate
	CACHEDIR /var/cache/migrate
	CLAIM egress
	"#,
    )
    .unwrap();
    assert_eq!(parsed.fetch_order, ["ingredient"]);
    assert_eq!(parsed.builder_order, ["build"]);
    assert_eq!(parsed.artifact_order, ["web", "migrate"]);
    assert_eq!(parsed.builders["build"].steps.len(), 3);
    assert_eq!(parsed.artifacts["web"].kind, ArtifactKind::Service);
    assert_eq!(parsed.artifacts["migrate"].kind, ArtifactKind::App);
}

#[test]
fn env_uses_equals_assignments_and_bare_optionals() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nENV PORT=8080\nENV API_TOKEN required\nENV THEME\nENV MESSAGE=\"hello world\"\nSTART /bin/true\n",
    )
    .unwrap();
    let env = &parsed.artifacts["app"].service.env;
    assert_eq!(
        env["PORT"]
            .default
            .as_ref()
            .unwrap()
            .literal_value()
            .as_deref(),
        Some("8080")
    );
    assert!(env["API_TOKEN"].required);
    assert!(env["THEME"].default.is_none());
    assert!(!env["THEME"].required);
    assert_eq!(
        env["MESSAGE"]
            .default
            .as_ref()
            .unwrap()
            .literal_value()
            .as_deref(),
        Some("hello world")
    );

    for declaration in ["ENV PORT = 8080", "ENV PORT=8080 required"] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n{declaration}\nSTART /bin/true\n"
        ))
        .unwrap_err();
        assert!(error.message.contains("ENV NAME=value"), "{error}");
    }
}

#[test]
fn ports_default_to_tcp_and_accept_systemd_udp_spelling() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nENV PORT=5353\nPORT http = 8080\nPORT dns = udp:$PORT\nSTART /bin/true\n",
    )
    .unwrap();
    assert_eq!(
        parsed.artifacts["app"].service.ports["http"].protocol,
        Protocol::Tcp
    );
    assert_eq!(
        parsed.artifacts["app"].service.ports["dns"].protocol,
        Protocol::Udp
    );
    assert_eq!(
        parsed.artifacts["app"].service.ports["dns"].source,
        PortSource::Env("PORT".into())
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&generate_spec_json(&parsed).unwrap()).unwrap();
    assert_eq!(manifest["ports"]["http"]["protocol"], "tcp");
    assert_eq!(manifest["ports"]["dns"]["protocol"], "udp");
}

#[test]
fn stopsignal_compiles_to_the_manifest_and_rejects_unknown_names() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\nSTOPSIGNAL SIGQUIT\n",
    )
    .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&generate_spec_json(&parsed).unwrap()).unwrap();
    assert_eq!(manifest["stopSignal"], "SIGQUIT");

    let error = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\nSTOPSIGNAL QUIT\n",
    )
    .unwrap_err();
    assert!(error.message.contains("known signal name"), "{error}");
}

#[test]
fn docker_udp_port_spelling_suggests_the_systemd_form() {
    let error = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nPORT http3 = 443/udp\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert_eq!(error.line, 3);
    assert!(error.message.contains("udp:443"), "{error}");
}

#[test]
fn outbound_has_a_d48_migration_error_and_is_not_an_alias() {
    let error =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSTART /bin/true\nOUTBOUND\n").unwrap_err();
    assert_eq!(error.line, 4);
    assert!(error.message.contains("CLAIM egress"), "{error}");
    assert!(error.message.contains("docs/cixfile.md#claims"), "{error}");
}

#[test]
fn role_directory_directives_and_claim_are_hard_migrations() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /var/lib/web\nCACHEDIR /var/cache/web\nLOGDIR /var/log/web\nCONFIGDIR /etc/web\nRUNDIR /run/web\nCLAIM jit\nCLAIM egress\n",
    )
    .unwrap();
    let dirs = &parsed.artifacts["web"].service.dirs;
    assert!(dirs.state.contains("/var/lib/web"));
    assert!(dirs.cache.contains("/var/cache/web"));
    assert!(dirs.logs.contains("/var/log/web"));
    assert!(dirs.config.contains("/etc/web"));
    assert!(dirs.run.contains("/run/web"));
    assert_eq!(
        parsed.artifacts["web"].service.claims,
        BTreeSet::from([Claim::Named("egress".into()), Claim::Named("jit".into()),])
    );
    for (directive, replacement, anchor) in [
        ("STATE /var/lib/web", "STATEDIR", "#role-dirs"),
        ("LOGS /var/log/web", "LOGDIR", "#role-dirs"),
        ("CONFIG /etc/web", "CONFIGDIR", "#role-dirs"),
        ("JIT", "CLAIM jit", "#claims"),
        ("EGRESS", "CLAIM egress", "#claims"),
    ] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{directive}\n"
        ))
        .unwrap_err();
        assert_eq!(error.line, 4);
        assert!(error.message.contains(replacement), "{error}");
        assert!(error.message.contains(anchor), "{error}");
    }
    let unknown =
        parse("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nCLAIM all\n").unwrap_err();
    assert!(unknown.message.contains("jit, egress"), "{unknown}");
}

#[test]
fn device_gpu_and_shm_claims_are_strict() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE detector\nSTART /bin/true\nCLAIM gpu\nCLAIM device /dev/video0\nSHM 256M\n",
    )
    .unwrap();
    let service = &parsed.artifacts["detector"].service;
    assert!(service.claims.contains(&Claim::Named("gpu".into())));
    assert!(service
        .claims
        .contains(&Claim::Device("/dev/video0".into())));
    assert_eq!(service.shm.as_deref(), Some("256M"));
    let manifest: serde_json::Value =
        serde_json::from_str(&generate_spec_json(&parsed).unwrap()).unwrap();
    assert_eq!(
        manifest["claims"],
        serde_json::json!(["gpu", {"device": "/dev/video0"}])
    );
    assert_eq!(manifest["shm"], "256M");

    for directive in [
        "CLAIM device ttyUSB0",
        "CLAIM device /dev/../etc/passwd",
        "CLAIM device /tmp/device",
        "CLAIM gpu extra",
        "SHM -1G",
        "SHM 1Z",
        "SHM 1G extra",
    ] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE detector\nSTART /bin/true\n{directive}\n"
        ))
        .unwrap_err();
        assert_eq!(error.line, 4, "{directive}: {error}");
    }
}

#[test]
fn readiness_and_liveness_parse_all_probe_forms_and_emit_typed_fields() {
    for (readiness, liveness, readiness_type, liveness_type) in [
        (
            "READINESS http://127.0.0.1/healthz IN 90s",
            "LIVENESS tcp://127.0.0.1:8080 EVERY 10s",
            "http",
            "tcp",
        ),
        (
            "READINESS tcp://db.internal:5432 IN 60s",
            "LIVENESS http://127.0.0.1:5432/livez EVERY 5s",
            "tcp",
            "http",
        ),
        (
            "READINESS notify IN 30s",
            "LIVENESS notify EVERY 2s",
            "notify",
            "notify",
        ),
    ] {
        let parsed = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{readiness}\n{liveness}\n"
        ))
        .unwrap();
        let service = &parsed.artifacts["web"].service;
        assert!(service.readiness.is_some());
        assert!(service.liveness.is_some());
        let manifest: serde_json::Value =
            serde_json::from_str(&generate_spec_json(&parsed).unwrap()).unwrap();
        assert_eq!(manifest["readiness"]["type"], readiness_type);
        assert_eq!(manifest["liveness"]["type"], liveness_type);
    }

    let path_only = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nPORT http = 8080\nSTART /bin/true\nREADINESS /healthz IN 90s\n",
    )
    .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&generate_spec_json(&path_only).unwrap()).unwrap();
    assert_eq!(
        manifest["readiness"]["target"],
        "http://127.0.0.1:8080/healthz"
    );

    let query = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nREADINESS http://127.0.0.1/healthz?full=1 IN 90s\n",
    )
    .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_str(&generate_spec_json(&query).unwrap()).unwrap();
    assert_eq!(
        manifest["readiness"]["target"],
        "http://127.0.0.1/healthz?full=1"
    );

    let app = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nAPP migrate\nSTART /bin/true\nREADINESS notify IN 30s\nLIVENESS notify EVERY 2s\n",
    )
    .unwrap();
    assert!(app.artifacts["migrate"].service.readiness.is_some());
}

#[test]
fn health_directives_reject_exec_malformed_targets_wrong_markers_and_duplicates() {
    for directive in [
        "READINESS exec bin/check IN 10s",
        "READINESS tcp://127.0.0.1:5432/health IN 10s",
        "READINESS notify EVERY 10s",
        "LIVENESS notify IN 10s",
        "LIVENESS notify EVERY 0s",
        "LIVENESS grpc://127.0.0.1:5000 EVERY 10s",
    ] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{directive}\n"
        ))
        .unwrap_err();
        assert_eq!(error.line, 4, "{directive}: {error}");
    }
    let legacy = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nREADINESS http 127.0.0.1:8080/healthz IN 10s\n",
    )
    .unwrap_err();
    assert!(
        legacy.message.contains("http://127.0.0.1:8080/healthz"),
        "{legacy}"
    );
    for (ports, expected) in [
        ("", "exactly one PORT"),
        (
            "PORT http = 8080\nPORT admin = 9090\n",
            "http://127.0.0.1:8080/healthz",
        ),
    ] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\n{ports}START /bin/true\nREADINESS /healthz IN 10s\n"
        ))
        .unwrap_err();
        assert!(error.message.contains(expected), "{error}");
    }
    let duplicate = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nREADINESS notify IN 10s\nREADINESS tcp://127.0.0.1:5432 IN 10s\n",
    )
    .unwrap_err();
    assert!(
        duplicate.message.contains("already declared"),
        "{duplicate}"
    );
}

#[test]
fn directories_accept_arbitrary_paths_and_dir_modes() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /srv/web/state\nCACHEDIR /app/cache\nLOGDIR /app/logs\nCONFIGDIR /config/web\nRUNDIR /tmp/web/run\nDIR /media:ro\nDIR /consume:rw\nDIR /scratch\n",
    )
    .unwrap();
    let dirs = &parsed.artifacts["web"].service.dirs;
    assert!(dirs.state.contains("/srv/web/state"));
    assert!(dirs.cache.contains("/app/cache"));
    assert!(dirs.logs.contains("/app/logs"));
    assert!(dirs.config.contains("/config/web"));
    assert!(dirs.run.contains("/tmp/web/run"));
    assert_eq!(dirs.data.get("/media"), Some(&true));
    assert_eq!(dirs.data.get("/consume"), Some(&false));
    assert_eq!(dirs.data.get("/scratch"), Some(&false));

    for declaration in [
        "STATEDIR relative",
        "DIR /data/../escape",
        "DIR /data:other",
    ] {
        let error = parse(&format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\n{declaration}\n"
        ))
        .unwrap_err();
        assert_eq!(error.line, 4, "{declaration}: {error}");
    }

    let duplicate = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE web\nSTART /bin/true\nSTATEDIR /shared\nDIR /shared:ro\n",
    )
    .unwrap_err();
    assert_eq!(duplicate.line, 5);
    assert!(duplicate.message.contains("duplicated"), "{duplicate}");
}

#[test]
fn app_rejects_service_only_surface_at_the_directive_line() {
    for (directive, message) in [
        ("PORT http = 8080", "PORT is not allowed inside APP"),
        ("LISTENER http", "LISTENER is not allowed inside APP"),
        ("JIT", "replace it with CLAIM jit"),
        ("START_PRE /bin/true", "START_PRE is not allowed inside APP"),
        ("LOGDIR /var/log/job", "LOGDIR is not allowed inside APP"),
        ("CONFIGDIR /etc/job", "CONFIGDIR is not allowed inside APP"),
        ("RUNDIR /run/job", "RUNDIR is not allowed inside APP"),
        ("PATH bin", "PATH was removed; use ENV PATH=<value>"),
    ] {
        let input = format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nAPP job\nSTART /bin/true\n{directive}\n");
        let error = parse(&input).unwrap_err();
        assert_eq!(error.line, 4, "{directive}: {error}");
        assert!(error.message.contains(message), "{directive}: {error}");
    }
}

#[test]
fn item_is_pure_assembly_and_runtime_directives_name_the_d68_seam() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\nCOPY payload /payload\nFILE /share/message <<EOF\nhello\nEOF\nCOPY ${pkgs.hello}/bin/hello /bin/hello\n",
    )
    .unwrap();
    assert_eq!(parsed.artifacts["data"].kind, ArtifactKind::Item);
    assert_eq!(parsed.artifacts["data"].copies.len(), 2);
    assert_eq!(parsed.artifacts["data"].assembly.len(), 1);

    for directive in [
        "START /bin/hello",
        "START_PRE /bin/hello",
        "ENV PATH=bin",
        "PORT http = 8080",
        "LISTENER http",
        "STATEDIR /var/lib/data",
        "CACHEDIR /var/cache/data",
        "LOGDIR /var/log/data",
        "CONFIGDIR /etc/data",
        "RUNDIR /run/data",
        "CLAIM egress",
        "HEALTH /bin/hello",
    ] {
        let input =
            format!("FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nITEM data\n{directive}\n");
        let error = parse(&input).unwrap_err();
        assert_eq!(error.line, 3, "{directive}: {error}");
        assert!(
            error.message.contains("ITEM is content-only"),
            "{directive}: {error}"
        );
        assert!(
            error.message.contains("use SERVICE or APP"),
            "{directive}: {error}"
        );
    }
}

#[test]
fn secret_declares_a_credential_name_and_optional_file_environment() {
    let parsed = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSECRET db-password AS DB_PASSWORD_FILE\nSECRET api-key\nSTART /bin/true\n",
    )
    .unwrap();
    assert_eq!(
        parsed.artifacts["app"].service.secrets["db-password"]
            .as_env
            .as_deref(),
        Some("DB_PASSWORD_FILE")
    );
    assert_eq!(
        parsed.artifacts["app"].service.secrets["api-key"].as_env,
        None
    );
    let error = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\nSECRET db AS DB_PASSWORD\nSTART /bin/true\n",
    )
    .unwrap_err();
    assert!(error.message.contains("_FILE"), "{error}");
}
