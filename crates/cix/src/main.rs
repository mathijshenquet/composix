use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::{fs, io::Read};

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

#[derive(Args)]
struct Logs {
    /// Composite name, optionally followed by /service.
    target: String,
    /// Follow the journal.
    #[arg(short = 'f')]
    follow: bool,
    /// Show entries since this journalctl timestamp.
    #[arg(long)]
    since: Option<String>,
    /// Show this many journal entries.
    #[arg(short = 'n')]
    lines: Option<u32>,
    /// Restrict output to one systemd invocation ID.
    #[arg(long)]
    invocation: Option<String>,
    /// Print the equivalent journalctl command without running it.
    #[arg(long)]
    explain: bool,
}

/// composix: a docker-shaped toolkit on nix + systemd.
#[derive(Parser)]
#[command(name = "cix", version)]
struct Cli {
    /// Directory containing local index state.
    #[arg(long, global = true, env = "CIX_STATE_DIR", default_value_os_t = default_state_directory())]
    state_directory: PathBuf,
    #[command(subcommand)]
    command: Command,
}

fn default_state_directory() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(".local/state"))
        .join("cix")
}

#[derive(clap::Subcommand)]
enum Command {
    /// Inspect an artifact or a running cix service.
    Inspect(Inspect),
    /// Read cix service logs from journald.
    Logs(Logs),
    /// Read one accounting snapshot from systemd; use systemd-cgtop for a live view.
    Stats,
    /// Manage host-local FETCH credential consent.
    Credentials {
        #[command(subcommand)]
        command: CredentialsCommand,
    },
    #[command(flatten)]
    Cixfile(cix_cixfile::cli::Command),
    #[command(flatten)]
    Compose(cix_compose::cli::Command),
    #[command(flatten)]
    Index(cix_index::cli::Command),
    /// Index maintenance commands.
    #[command(name = "index")]
    IndexGroup {
        #[command(subcommand)]
        command: cix_index::cli::Command,
    },
    #[command(flatten)]
    Run(cix_run::cli::Command),
}

#[derive(clap::Subcommand)]
enum CredentialsCommand {
    /// Revoke every host-local consent grant for a named FETCH token.
    Revoke { token: String },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let state_directory = cli.state_directory;
    match cli.command {
        Command::Inspect(options) => inspect(&state_directory, options),
        Command::Logs(options) => cix_compose::logs(cix_compose::LogsOptions {
            target: options.target,
            follow: options.follow,
            since: options.since,
            lines: options.lines,
            invocation: options.invocation,
            explain: options.explain,
        }),
        Command::Stats => cix_compose::stats(),
        Command::Credentials {
            command: CredentialsCommand::Revoke { token },
        } => {
            let removed = cix_cixfile::revoke_fetch_consent(&token)?;
            println!("revoked {removed} FETCH consent grant(s) for {token}");
            Ok(())
        }
        Command::Cixfile(cmd) => cmd.run(&state_directory),
        Command::Compose(cmd) => cmd.run(&cix_index::Store::open(state_directory.clone())?),
        Command::Index(cmd) => cmd.run(&cix_index::Store::open(state_directory.clone())?),
        Command::IndexGroup { command } => {
            command.run(&cix_index::Store::open(state_directory.clone())?)
        }
        Command::Run(cix_run::cli::Command::Ps) => cix_compose::ps(),
        Command::Run(
            command @ cix_run::cli::Command::Run {
                compose: Some(_),
                installable: None,
                ..
            },
        ) => run_compose(&state_directory, command),
        Command::Run(cmd) => cmd.run(&state_directory),
    }
}

fn run_compose(
    state_directory: &std::path::Path,
    command: cix_run::cli::Command,
) -> anyhow::Result<()> {
    let cix_run::cli::Command::Run {
        compose: Some(compose),
        installable: None,
        env,
        port,
        dirs,
        identity,
        detach,
        schedule,
        closed_root,
        user,
    } = command
    else {
        unreachable!("only a compose run command reaches this helper");
    };
    if !env.is_empty()
        || !port.is_empty()
        || !dirs.is_empty()
        || identity.is_some()
        || detach
        || schedule.is_some()
        || user
    {
        bail!("cix run --compose accepts the compose document as the complete operator surface; put service fields in that JSON")
    }
    if compose.as_os_str() != "-" {
        return cix_compose::up(
            &cix_index::Store::open(state_directory.to_owned())?,
            &compose,
            cix_compose::UpdateRequest::None,
            closed_root,
        );
    }
    let mut input = Vec::new();
    std::io::stdin().read_to_end(&mut input)?;
    let file = tempfile::NamedTempFile::new().context("creating anonymous compose input")?;
    fs::write(file.path(), input).context("writing anonymous compose input")?;
    cix_compose::up(
        &cix_index::Store::open(state_directory.to_owned())?,
        file.path(),
        cix_compose::UpdateRequest::None,
        closed_root,
    )
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
    invocation_id: String,
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
    diagnosis: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum InspectionWorld {
    Artifact,
    Runtime,
}

fn inspect(state_directory: &std::path::Path, options: Inspect) -> anyhow::Result<()> {
    if options.user && !options.runtime {
        bail!("--user only applies to runtime inspection; add --runtime");
    }

    let artifact_exists = local_artifact_exists(state_directory, &options.target)?;
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
            let inspection = inspect_artifact(state_directory, &options.target)?;
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

fn local_artifact_exists(state_directory: &std::path::Path, target: &str) -> anyhow::Result<bool> {
    let Ok(reference) = cix_common::Ref::parse(target) else {
        return Ok(false);
    };
    if reference.root_url.is_some() {
        return Ok(false);
    }
    Ok(cix_index::Store::open(state_directory.to_owned())?
        .load(&reference)?
        .is_some())
}

fn inspect_artifact(
    state_directory: &std::path::Path,
    target: &str,
) -> anyhow::Result<ArtifactInspection> {
    let artifact =
        cix_index::inspect_artifact(&cix_index::Store::open(state_directory.to_owned())?, target)?;
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
    "RootDirectory",
    "MountAPIVFS",
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
        "InvocationID",
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
        invocation_id: property("InvocationID"),
        exit_cause: ExitCause {
            result: cix_compose::result_label(&property("Result")).to_owned(),
            code: property("ExecMainCode"),
            status: property("ExecMainStatus"),
            diagnosis: spawn_exit_diagnosis(&property("ExecMainStatus")).map(str::to_owned),
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
    println!("invocation id  {}", inspection.invocation_id);
    println!(
        "exit cause     {}/{}/{}{}",
        inspection.exit_cause.result,
        inspection.exit_cause.code,
        inspection.exit_cause.status,
        inspection
            .exit_cause
            .diagnosis
            .as_deref()
            .map(|diagnosis| format!(" ({diagnosis})"))
            .unwrap_or_default(),
    );
    if !inspection.ports.is_empty() {
        println!("ports          {}", inspection.ports.join(","));
    }
    for (role, paths) in &inspection.dirs {
        println!("{role} dirs     {}", paths.join(","));
    }
    Ok(())
}

fn spawn_exit_diagnosis(status: &str) -> Option<&'static str> {
    let status = status.parse::<u16>().ok()?;
    Some(match status {
        200 => "working-directory setup failed",
        201 => "scheduling priority setup failed",
        202 => "file-descriptor setup failed",
        203 => "exec failed",
        204 => "memory setup failed",
        205 => "resource-limit setup failed",
        206 => "OOM-score setup failed",
        207 => "signal-mask setup failed",
        208 => "standard-input setup failed",
        209 => "standard-output setup failed",
        210 => "chroot setup failed",
        211 => "I/O-priority setup failed",
        212 => "timer-slack setup failed",
        213 => "secure-bits setup failed",
        214 => "scheduler setup failed",
        215 => "CPU-affinity setup failed",
        216 => "group setup failed",
        217 => "user setup failed",
        218 => "capability setup failed",
        219 => "cgroup setup failed",
        220 => "session setup failed",
        221 => "confirmation failed",
        222 => "standard-error setup failed",
        223 => "reserved systemd spawn failure",
        224 => "PAM setup failed",
        225 => "network setup failed",
        226 => "namespace setup failed",
        227 => "no-new-privileges setup failed",
        228 => "seccomp setup failed",
        229 => "SELinux context setup failed",
        230 => "personality setup failed",
        231 => "AppArmor setup failed",
        232 => "address-family setup failed",
        233 => "runtime-directory setup failed",
        234 => "reserved systemd spawn failure",
        235 => "ownership setup failed",
        236 => "SMACK label setup failed",
        237 => "keyring setup failed",
        238 => "state-directory setup failed",
        239 => "cache-directory setup failed",
        240 => "logs-directory setup failed",
        241 => "configuration-directory setup failed",
        242 => "NUMA-policy setup failed",
        243 => "credentials setup failed",
        244 => "BPF setup failed",
        245 => "KSM setup failed",
        _ => return None,
    })
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
            br#"{"cixManifest":0,"start":["bin/web"],"listeners":{"http":{"type":"stream"}}}"#,
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
            invocation_id: "0123456789abcdef0123456789abcdef".into(),
            exit_cause: ExitCause {
                result: "success".into(),
                code: "exited".into(),
                status: "0".into(),
                diagnosis: None,
            },
            properties: BTreeMap::new(),
            ports: vec!["tcp:8080".into()],
            listeners: BTreeMap::new(),
            dirs: BTreeMap::from([("state".into(), vec!["/var/lib/private/web".into()])]),
        };
        assert_eq!(
            serde_json::to_value(&artifact).unwrap()["manifest"]["cixManifest"],
            0
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

    #[test]
    fn maps_systemd_spawn_exit_codes() {
        assert_eq!(spawn_exit_diagnosis("226"), Some("namespace setup failed"));
        assert_eq!(spawn_exit_diagnosis("245"), Some("KSM setup failed"));
        assert_eq!(
            spawn_exit_diagnosis("223"),
            Some("reserved systemd spawn failure")
        );
    }
}
