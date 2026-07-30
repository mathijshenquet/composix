use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;

use anyhow::{bail, Context};
use cix_common::current_system;
use clap::{Args, Parser};
use serde::Serialize;

#[derive(Args)]
struct Inspect {
    /// A local/remote tag, installable, exact unit name, or unique running service name.
    target: String,
    /// Render a compact table instead of stable JSON.
    #[arg(short = 'H', long)]
    human: bool,
    /// Force artifact inspection when a name could also select a running service.
    #[arg(long, conflicts_with = "runtime")]
    artifact: bool,
    /// Force runtime inspection when a name could also select a tag.
    #[arg(long, conflicts_with = "artifact")]
    runtime: bool,
    /// Inspect a user-manager unit (the explicitly degraded runtime mode).
    #[arg(long, requires = "runtime")]
    user: bool,
}

/// composix: a docker-shaped toolkit on nix + systemd.
#[derive(Parser)]
#[command(name = "cix", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Inspect an artifact or a running cix service.
    Inspect(Inspect),
    #[command(flatten)]
    Cixfile(cix_cixfile::cli::Command),
    #[command(flatten)]
    Compose(cix_compose::cli::Command),
    #[command(flatten)]
    Index(cix_index::cli::Command),
    /// Index maintenance commands.
    #[command(name = "index")]
    IndexCommand {
        #[command(subcommand)]
        command: cix_index::cli::Command,
    },
    #[command(flatten)]
    Run(cix_run::cli::Command),
}

fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Inspect(options) => inspect(options),
        Command::Cixfile(cmd) => cmd.run(),
        Command::Compose(cmd) => cmd.run(),
        Command::Index(cmd) => cmd.run(),
        Command::IndexCommand { command } => command.run(),
        Command::Run(cix_run::cli::Command::Ps) => cix_compose::ps(),
        Command::Run(cmd) => cmd.run(),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ArtifactInspection {
    kind: &'static str,
    reference: Option<String>,
    store_path: String,
    nar_hash: String,
    outputs: BTreeMap<String, cix_index::Output>,
    manifest: cix_run::spec::Spec,
    closure_size: Option<u64>,
    trusted_keys: Vec<String>,
    upstream: Option<String>,
    drv_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInspection {
    kind: &'static str,
    unit: String,
    service: String,
    state: RuntimeState,
    main_pid: u32,
    exit_cause: ExitCause,
    properties: BTreeMap<String, String>,
    ports: Vec<String>,
    listeners: BTreeMap<String, String>,
    dirs: BTreeMap<String, Vec<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeState {
    load: String,
    active: String,
    sub: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExitCause {
    result: String,
    code: String,
    status: String,
}

#[derive(Debug, PartialEq, Eq)]
enum InspectionWorld {
    Artifact,
    Runtime,
}

fn inspect(options: Inspect) -> anyhow::Result<()> {
    if options.user && !options.runtime {
        bail!("--user only applies to runtime inspection; add --runtime");
    }

    let artifact_exists = local_artifact_exists(&options.target)?;
    let runtime = if options.artifact {
        None
    } else {
        cix_run::exec::select_running_target(&options.target, options.user).ok()
    };

    match select_world(&options, artifact_exists, runtime.is_some())? {
        InspectionWorld::Runtime => {
            let target = runtime.with_context(|| {
                format!(
                    "no running cix service matches {:?}; use an exact unit from `cix ps`",
                    options.target
                )
            })?;
            let inspection = inspect_runtime(&target, options.user)?;
            render_runtime(&inspection, options.human)
        }
        InspectionWorld::Artifact => {
            let inspection = inspect_artifact(&options.target)?;
            render_artifact(&inspection, options.human)
        }
    }
}

fn select_world(
    options: &Inspect,
    artifact_exists: bool,
    runtime_exists: bool,
) -> anyhow::Result<InspectionWorld> {
    if options.artifact {
        return Ok(InspectionWorld::Artifact);
    }
    if options.runtime {
        return Ok(InspectionWorld::Runtime);
    }
    if artifact_exists && runtime_exists {
        bail!(
            "{target:?} is both a local artifact tag and a running service; use `cix inspect --artifact {target}` or `cix inspect --runtime {target}` to disambiguate",
            target = options.target
        );
    }
    if runtime_exists {
        Ok(InspectionWorld::Runtime)
    } else {
        Ok(InspectionWorld::Artifact)
    }
}

fn local_artifact_exists(target: &str) -> anyhow::Result<bool> {
    let Ok(reference) = cix_common::Ref::parse(target) else {
        return Ok(false);
    };
    if reference.root_url.is_some() {
        return Ok(false);
    }
    Ok(cix_index::Store::open()?.load(&reference)?.is_some())
}

fn inspect_artifact(target: &str) -> anyhow::Result<ArtifactInspection> {
    let artifact = cix_index::inspect_artifact(target)?;
    let manifest = cix_run::spec::Spec::load(&PathBuf::from(&artifact.output.store_path))?;
    let system = current_system()?;
    let metadata = artifact.metadata;
    let outputs = metadata
        .as_ref()
        .map(|metadata| metadata.entry.outputs.clone())
        .unwrap_or_else(|| BTreeMap::from([(system, artifact.output.clone())]));
    Ok(ArtifactInspection {
        kind: "artifact",
        reference: metadata.as_ref().map(|metadata| metadata.reference.clone()),
        store_path: artifact.output.store_path.clone(),
        nar_hash: artifact.output.nar_hash,
        outputs,
        manifest,
        closure_size: cix_index::closure_size(&artifact.output.store_path),
        trusted_keys: metadata
            .as_ref()
            .map(|metadata| metadata.entry.trusted_keys.clone())
            .unwrap_or_default(),
        upstream: metadata.and_then(|metadata| metadata.upstream),
        drv_path: artifact.output.drv_path,
    })
}

const GENERATED_PROPERTIES: &[&str] = &[
    "Type",
    "Slice",
    "DynamicUser",
    "PrivateUsers",
    "ProtectSystem",
    "ProtectHome",
    "PrivateTmp",
    "PrivatePIDs",
    "NoNewPrivileges",
    "RestrictSUIDSGID",
    "ProtectKernelTunables",
    "ProtectKernelModules",
    "ProtectKernelLogs",
    "ProtectControlGroups",
    "LockPersonality",
    "MemoryDenyWriteExecute",
    "SystemCallFilter",
    "AmbientCapabilities",
    "CapabilityBoundingSet",
    "RestrictAddressFamilies",
    "PrivateNetwork",
    "SocketBindAllow",
    "SocketBindDeny",
    "BindReadOnlyPaths",
    "BindPaths",
    "StateDirectory",
    "CacheDirectory",
    "LogsDirectory",
    "ConfigurationDirectory",
    "RuntimeDirectory",
    "Environment",
    "ExecStartPre",
    "ExecStart",
    "Sockets",
];

fn inspect_runtime(
    target: &cix_run::exec::RunningTarget,
    user: bool,
) -> anyhow::Result<RuntimeInspection> {
    let mut names = vec![
        "LoadState",
        "ActiveState",
        "SubState",
        "MainPID",
        "Result",
        "ExecMainCode",
        "ExecMainStatus",
    ];
    names.extend(GENERATED_PROPERTIES);
    let values = systemctl_properties(user, &target.unit, &names)?;
    let property = |name: &str| values.get(name).cloned().unwrap_or_default();
    let main_pid = property("MainPID")
        .parse()
        .with_context(|| format!("systemctl reported an invalid MainPID for {}", target.unit))?;
    let properties = GENERATED_PROPERTIES
        .iter()
        .filter_map(|name| {
            values
                .get(*name)
                .map(|value| ((*name).to_owned(), value.clone()))
        })
        .filter(|(_, value)| !value.is_empty())
        .collect::<BTreeMap<_, _>>();
    let listeners = listener_bindings(user, &property("Sockets"))?;
    Ok(RuntimeInspection {
        kind: "runtime",
        unit: target.unit.clone(),
        service: target.service.clone(),
        state: RuntimeState {
            load: property("LoadState"),
            active: property("ActiveState"),
            sub: property("SubState"),
        },
        main_pid,
        exit_cause: ExitCause {
            result: property("Result"),
            code: property("ExecMainCode"),
            status: property("ExecMainStatus"),
        },
        ports: property("SocketBindAllow")
            .split_whitespace()
            .filter(|binding| *binding != "any")
            .map(str::to_owned)
            .collect(),
        listeners,
        dirs: host_role_directories(user, &properties),
        properties,
    })
}

fn systemctl_properties(
    user: bool,
    unit: &str,
    names: &[&str],
) -> anyhow::Result<BTreeMap<String, String>> {
    let mut command = ProcessCommand::new("systemctl");
    if user {
        command.arg("--user");
    }
    command.arg("show").arg(unit).arg("--no-pager");
    for name in names {
        command.arg(format!("--property={name}"));
    }
    let output = command
        .output()
        .with_context(|| format!("failed to inspect {unit}"))?;
    if !output.status.success() {
        bail!(
            "failed to inspect {unit}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8(output.stdout)?
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect())
}

fn listener_bindings(user: bool, sockets: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let mut listeners = BTreeMap::new();
    for socket in sockets
        .split_whitespace()
        .filter(|socket| socket.ends_with(".socket"))
    {
        let values = systemctl_properties(user, socket, &["Listen"])?;
        let name = socket
            .trim_end_matches(".socket")
            .rsplit_once('-')
            .map(|(_, listener)| listener)
            .unwrap_or(socket);
        listeners.insert(
            name.to_owned(),
            values.get("Listen").cloned().unwrap_or_default(),
        );
    }
    Ok(listeners)
}

fn host_role_directories(
    user: bool,
    properties: &BTreeMap<String, String>,
) -> BTreeMap<String, Vec<String>> {
    [
        ("state", "StateDirectory", "/var/lib", "/var/lib/private"),
        (
            "cache",
            "CacheDirectory",
            "/var/cache",
            "/var/cache/private",
        ),
        ("logs", "LogsDirectory", "/var/log", "/var/log/private"),
        ("config", "ConfigurationDirectory", "/etc", "/etc"),
        ("run", "RuntimeDirectory", "/run", "/run"),
    ]
    .into_iter()
    .filter_map(|(role, property, root, private_root)| {
        let value = properties.get(property)?;
        let root = if user { root } else { private_root };
        let paths = value
            .split_whitespace()
            .map(|name| name.split(':').next().unwrap_or(name))
            .map(|name| format!("{root}/{name}"))
            .collect::<Vec<_>>();
        (!paths.is_empty()).then(|| (role.to_owned(), paths))
    })
    .collect()
}

fn render_artifact(inspection: &ArtifactInspection, human: bool) -> anyhow::Result<()> {
    if !human {
        println!("{}", serde_json::to_string_pretty(inspection)?);
        return Ok(());
    }
    println!(
        "artifact {}",
        inspection.reference.as_deref().unwrap_or("(installable)")
    );
    println!("store path     {}", inspection.store_path);
    println!("nar hash       {}", inspection.nar_hash);
    println!(
        "systems        {}",
        inspection
            .outputs
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "closure size   {}",
        inspection
            .closure_size
            .map(|size| size.to_string())
            .unwrap_or_else(|| "unavailable".into())
    );
    if let Some(upstream) = &inspection.upstream {
        println!("upstream       {upstream}");
    }
    if let Some(drv_path) = &inspection.drv_path {
        println!("drv path       {drv_path}");
    }
    Ok(())
}

fn render_runtime(inspection: &RuntimeInspection, human: bool) -> anyhow::Result<()> {
    if !human {
        println!("{}", serde_json::to_string_pretty(inspection)?);
        return Ok(());
    }
    println!("runtime {} ({})", inspection.service, inspection.unit);
    println!(
        "state          {}/{}/{}",
        inspection.state.load, inspection.state.active, inspection.state.sub
    );
    println!("main pid       {}", inspection.main_pid);
    println!(
        "exit cause     {}/{}/{}",
        inspection.exit_cause.result, inspection.exit_cause.code, inspection.exit_cause.status
    );
    if !inspection.ports.is_empty() {
        println!("ports          {}", inspection.ports.join(","));
    }
    for (role, paths) in &inspection.dirs {
        println!("{role} dirs     {}", paths.join(","));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(target: &str) -> Inspect {
        Inspect {
            target: target.into(),
            human: false,
            artifact: false,
            runtime: false,
            user: false,
        }
    }

    #[test]
    fn dispatches_unambiguous_artifact_and_runtime_targets() {
        assert_eq!(
            select_world(&options("item:v1"), true, false).unwrap(),
            InspectionWorld::Artifact
        );
        assert_eq!(
            select_world(&options("nginx"), false, true).unwrap(),
            InspectionWorld::Runtime
        );
    }

    #[test]
    fn ambiguity_names_both_exact_disambiguators() {
        let error = select_world(&options("nginx"), true, true)
            .unwrap_err()
            .to_string();
        assert!(error.contains("cix inspect --artifact nginx"), "{error}");
        assert!(error.contains("cix inspect --runtime nginx"), "{error}");
    }

    #[test]
    fn json_schema_has_stable_artifact_and_runtime_field_names() {
        let manifest = cix_run::spec::Spec::from_slice(
            br#"{"cixManifest":3,"services":{"web":{"exec":["bin/web"],"listeners":{"http":{"type":"stream"}}}}}"#,
        )
        .unwrap();
        let artifact = ArtifactInspection {
            kind: "artifact",
            reference: Some("web:v1".into()),
            store_path: "/nix/store/example-web".into(),
            nar_hash: "sha256-example".into(),
            outputs: BTreeMap::from([(
                "x86_64-linux".into(),
                cix_index::Output {
                    store_path: "/nix/store/example-web".into(),
                    nar_hash: "sha256-example".into(),
                    drv_path: None,
                },
            )]),
            manifest,
            closure_size: Some(42),
            trusted_keys: vec!["cache.example:abc".into()],
            upstream: None,
            drv_path: None,
        };
        let runtime = RuntimeInspection {
            kind: "runtime",
            unit: "cix-run-web-1.service".into(),
            service: "web".into(),
            state: RuntimeState {
                load: "loaded".into(),
                active: "active".into(),
                sub: "running".into(),
            },
            main_pid: 123,
            exit_cause: ExitCause {
                result: "success".into(),
                code: "exited".into(),
                status: "0".into(),
            },
            properties: BTreeMap::new(),
            ports: vec!["tcp:8080".into()],
            listeners: BTreeMap::new(),
            dirs: BTreeMap::from([("state".into(), vec!["/var/lib/private/web".into()])]),
        };
        assert_eq!(
            serde_json::to_value(&artifact).unwrap()["manifest"]["cixManifest"],
            3
        );
        assert_eq!(
            serde_json::to_value(&artifact).unwrap()["storePath"],
            "/nix/store/example-web"
        );
        assert_eq!(serde_json::to_value(&runtime).unwrap()["mainPid"], 123);
        assert_eq!(
            serde_json::to_value(&runtime).unwrap()["exitCause"]["result"],
            "success"
        );
    }
}
