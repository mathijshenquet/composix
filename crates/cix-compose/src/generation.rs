use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cix_run::{
    capabilities::HostCapabilities,
    unit::{compile_unit_for_host, UnitCompileOptions, UnitMode, UnitNaming},
};
use serde::{Deserialize, Serialize};

use crate::{
    model::DirectoryRole,
    resolve::{CheckResult, DirectoryBacking, DirectoryClaim},
};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,
    pub units: BTreeMap<String, ManifestUnit>,
    pub services: BTreeMap<String, ManifestService>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub degradations: Vec<ManifestDegradation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ManifestDegradation {
    pub unit: String,
    pub property: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ManifestUnit {
    pub kind: UnitKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub scheduled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UnitKind {
    Service,
    Timer,
    Edge,
    Socket,
    Slice,
    Target,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestService {
    pub item_service: String,
    pub store_path: String,
    pub nar_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shm: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub directories: Vec<ManifestDirectory>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ManifestDirectory {
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<DirectoryRole>,
    pub backing: DirectoryBackingKind,
    pub host_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryBackingKind {
    Private,
    Host,
    Shared,
}

#[derive(Clone, Debug)]
pub struct BuiltGeneration {
    pub store_path: PathBuf,
    pub manifest: Manifest,
}

pub fn build_generation(
    checked: &CheckResult,
    compose_path: &Path,
    capabilities: &HostCapabilities,
) -> Result<BuiltGeneration> {
    let temporary = tempfile::tempdir().context("creating compose generation workspace")?;
    let generation = temporary
        .path()
        .join(format!("cix-compose-{}-generation", checked.compose.name));
    let manifest = render_generation(checked, compose_path, &generation, capabilities)?;
    let generation_text = generation.to_string_lossy().into_owned();
    let output = cix_common::nix(&["store", "add-path", &generation_text])
        .context("adding compose generation to the Nix store")?;
    let store_path = PathBuf::from(output.trim());
    Ok(BuiltGeneration {
        store_path,
        manifest,
    })
}

pub fn render_generation(
    checked: &CheckResult,
    compose_path: &Path,
    generation: &Path,
    capabilities: &HostCapabilities,
) -> Result<Manifest> {
    let units_dir = generation.join("units");
    let sysusers_dir = generation.join("sysusers.d");
    fs::create_dir_all(&units_dir)?;
    fs::create_dir_all(&sysusers_dir)?;
    let _ = compose_path;
    write_json(&generation.join("compose.json"), &checked.compose)?;
    write_json(&generation.join("cix.lock"), &checked.lock)?;

    let rendered = render_units(checked, capabilities)?;
    for (name, text) in &rendered.units {
        fs::write(units_dir.join(name), text)
            .with_context(|| format!("writing generated unit {name}"))?;
    }
    fs::write(
        sysusers_dir.join(format!("cix-{}.conf", checked.compose.name)),
        rendered.sysusers,
    )?;
    write_json(&generation.join("manifest.json"), &rendered.manifest)?;
    Ok(rendered.manifest)
}

struct Rendered {
    units: BTreeMap<String, String>,
    sysusers: String,
    manifest: Manifest,
}

fn render_units(checked: &CheckResult, capabilities: &HostCapabilities) -> Result<Rendered> {
    let composite = &checked.compose.name;
    let slice = format!("cix-{composite}.slice");
    let target = format!("cix-{composite}.target");
    let prefix = format!("cix-{composite}");
    let mut units = BTreeMap::new();
    let mut manifest_units = BTreeMap::new();
    let mut service_edges: BTreeMap<&str, Vec<EdgeClaim>> = BTreeMap::new();
    let mut sysusers = String::new();
    let mut target_wants = BTreeSet::new();
    let mut target_after = BTreeSet::new();
    let mut degradations = Vec::new();
    let shared = collect_shared_directories(checked);

    for (name, shared) in &shared {
        let unit = format!("{prefix}-shared-{name}.service");
        let group = shared_group(composite, name);
        sysusers.push_str(&format!("g {group} - -\n"));
        units.insert(
            unit.clone(),
            render_shared_directory_unit(name, &target, &shared.host_path, &group, &shared.members),
        );
        manifest_units.insert(
            unit.clone(),
            ManifestUnit {
                kind: UnitKind::Edge,
                service: None,
                scheduled: false,
            },
        );
        target_wants.insert(unit.clone());
        target_after.insert(unit);
    }

    for (edge_name, edge) in &checked.compose.edges {
        let edge_unit = format!("{prefix}-edge-{edge_name}.service");
        let runtime = format!("{prefix}-edge-{edge_name}");
        let group = edge_group(composite, edge_name);
        sysusers.push_str(&format!("g {group} - -\n"));
        let members = std::iter::once(edge.producer.service.as_str())
            .chain(edge.consumers.keys().map(String::as_str))
            .map(|service| format!("{prefix}-{service}.service"))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        units.insert(
            edge_unit.clone(),
            render_edge_unit(edge_name, &target, &runtime, &group, &members),
        );
        manifest_units.insert(
            edge_unit.clone(),
            ManifestUnit {
                kind: UnitKind::Edge,
                service: None,
                scheduled: false,
            },
        );
        target_wants.insert(edge_unit.clone());
        target_after.insert(edge_unit.clone());

        service_edges
            .entry(&edge.producer.service)
            .or_default()
            .push(EdgeClaim {
                unit: edge_unit.clone(),
                dependency: None,
                group: group.clone(),
                source: format!("/run/{runtime}"),
                destination: edge.producer.path.display().to_string(),
            });
        for (consumer, config) in &edge.consumers {
            service_edges.entry(consumer).or_default().push(EdgeClaim {
                unit: edge_unit.clone(),
                dependency: Some(format!("{prefix}-{}.service", edge.producer.service)),
                group: group.clone(),
                source: format!("/run/{runtime}"),
                destination: config
                    .path
                    .as_deref()
                    .unwrap_or(&edge.producer.path)
                    .display()
                    .to_string(),
            });
        }
    }

    let mut manifest_services = BTreeMap::new();
    for (service_name, checked_service) in &checked.services {
        let service_unit = format!("{prefix}-{service_name}.service");
        let declaration = &checked.compose.services[service_name];
        let claims = service_edges
            .get(service_name.as_str())
            .cloned()
            .unwrap_or_default();
        let mut extra_properties = Vec::new();
        let mut unit_properties = Vec::new();
        if checked.compose.log_namespace {
            extra_properties.push(("LogNamespace".into(), format!("cix-{composite}")));
        }
        let directory_groups = checked_service
            .directories
            .iter()
            .filter_map(|claim| match &claim.backing {
                DirectoryBacking::Shared { name } => Some(shared_group(composite, name)),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        if !claims.is_empty() || !directory_groups.is_empty() {
            extra_properties.push((
                "SupplementaryGroups".into(),
                claims
                    .iter()
                    .map(|claim| claim.group.as_str())
                    .chain(directory_groups.iter().map(String::as_str))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
            extra_properties.push((
                "UMask".into(),
                if directory_groups.is_empty() {
                    "0007"
                } else {
                    "0002"
                }
                .into(),
            ));
            for claim in &claims {
                extra_properties.push((
                    "BindPaths".into(),
                    format!("{}:{}:rbind", claim.source, claim.destination),
                ));
            }
        }
        for claim in &checked_service.directories {
            match &claim.backing {
                DirectoryBacking::Private => {}
                DirectoryBacking::Host { path, idmap } => {
                    let value = bind_value(path, &claim.path, *idmap);
                    extra_properties.push((
                        if claim.writable {
                            "BindPaths"
                        } else {
                            "BindReadOnlyPaths"
                        }
                        .into(),
                        value,
                    ));
                    unit_properties.push(("RequiresMountsFor".into(), path.display().to_string()));
                }
                DirectoryBacking::Shared { name } => {
                    let shared = &shared[name];
                    extra_properties.push((
                        "BindPaths".into(),
                        bind_value(&shared.host_path, &claim.path, false),
                    ));
                    let unit = format!("{prefix}-shared-{name}.service");
                    unit_properties.push(("Requires".into(), unit.clone()));
                    unit_properties.push(("After".into(), unit));
                }
            }
        }
        let sockets = checked_service
            .config
            .listeners
            .keys()
            .map(|listener| format!("{prefix}-{service_name}-{listener}.socket"))
            .collect::<Vec<_>>();
        if !sockets.is_empty() {
            extra_properties.push(("Sockets".into(), sockets.join(" ")));
        }
        let mut compiled_service =
            private_service(&checked_service.spec, &checked_service.directories);
        if let Some(shm) = &declaration.shm {
            compiled_service.shm = Some(shm.clone());
        }
        if let Some(identity) = &declaration.identity {
            extra_properties.extend([
                ("DynamicUser".into(), "no".into()),
                ("User".into(), identity.clone()),
                ("Group".into(), identity.clone()),
            ]);
        }
        if let Some(run_paths) = &mut compiled_service.dirs.run {
            run_paths.retain(|path| {
                !claims
                    .iter()
                    .any(|claim| Path::new(&claim.destination) == path)
            });
        }
        let compiled = compile_unit_for_host(
            &checked_service.store_path,
            service_name,
            &compiled_service,
            &checked_service.config,
            UnitMode::System,
            &UnitCompileOptions {
                naming: UnitNaming {
                    unit: service_unit.clone(),
                    slice: slice.clone(),
                    target: target.clone(),
                    directory_prefix: prefix.clone(),
                },
                extra_properties,
                unit_properties: Vec::new(),
                log_fields: vec![
                    ("CIX_COMPOSITE".into(), composite.clone()),
                    ("CIX_SERVICE".into(), service_name.clone()),
                ],
                probe_binary: None,
            },
            capabilities,
        )
        .with_context(|| format!("compiling services.{service_name}"))?;
        degradations.extend(
            compiled
                .degradations
                .iter()
                .map(|degradation| ManifestDegradation {
                    unit: service_unit.clone(),
                    property: degradation.property.clone(),
                    reason: degradation.reason.clone(),
                }),
        );
        let requires = claims
            .iter()
            .map(|claim| claim.unit.clone())
            .chain(sockets.iter().cloned())
            .chain(
                unit_properties
                    .iter()
                    .filter_map(|(name, value)| (name == "Requires").then_some(value.clone())),
            )
            .collect::<BTreeSet<_>>();
        let after = requires
            .iter()
            .cloned()
            .chain(claims.iter().filter_map(|claim| claim.dependency.clone()))
            .chain(
                unit_properties
                    .iter()
                    .filter_map(|(name, value)| (name == "After").then_some(value.clone())),
            )
            .collect::<BTreeSet<_>>();
        let unit_properties = unit_properties
            .into_iter()
            .filter(|(name, _)| name != "Requires" && name != "After")
            .collect::<Vec<_>>();
        let text =
            add_unit_dependencies(&compiled.text, &target, &requires, &after, &unit_properties);
        units.insert(service_unit.clone(), text);
        manifest_units.insert(
            service_unit.clone(),
            ManifestUnit {
                kind: UnitKind::Service,
                service: Some(service_name.clone()),
                scheduled: declaration.schedule.is_some(),
            },
        );
        if let Some(schedule) = declaration.schedule.as_deref() {
            let timer = format!("{prefix}-{service_name}.timer");
            units.insert(
                timer.clone(),
                render_timer_unit(
                    service_name,
                    &service_unit,
                    &target,
                    schedule,
                    declaration.persistent,
                    declaration.jitter.as_deref(),
                ),
            );
            manifest_units.insert(
                timer.clone(),
                ManifestUnit {
                    kind: UnitKind::Timer,
                    service: Some(service_name.clone()),
                    scheduled: false,
                },
            );
            target_wants.insert(timer);
        } else {
            target_wants.insert(service_unit.clone());
        }

        for (listener, address) in &checked_service.config.listeners {
            let socket = format!("{prefix}-{service_name}-{listener}.socket");
            units.insert(
                socket.clone(),
                render_socket_unit(listener, &service_unit, &target, address),
            );
            manifest_units.insert(
                socket.clone(),
                ManifestUnit {
                    kind: UnitKind::Socket,
                    service: Some(service_name.clone()),
                    scheduled: false,
                },
            );
            target_wants.insert(socket);
        }
        let locked = &checked.lock.services[service_name];
        manifest_services.insert(
            service_name.clone(),
            ManifestService {
                item_service: checked_service.item_service.clone(),
                store_path: locked.store_path.clone(),
                nar_hash: locked.nar_hash.clone(),
                shm: compiled_service.shm,
                directories: checked_service
                    .directories
                    .iter()
                    .map(|claim| manifest_directory(composite, service_name, claim))
                    .collect(),
            },
        );
    }

    units.insert(
        slice.clone(),
        format!("[Unit]\nDescription=cix compose slice: {composite}\n"),
    );
    manifest_units.insert(
        slice.clone(),
        ManifestUnit {
            kind: UnitKind::Slice,
            service: None,
            scheduled: false,
        },
    );
    units.insert(
        target.clone(),
        render_target(composite, &target_wants, &target_after),
    );
    manifest_units.insert(
        target,
        ManifestUnit {
            kind: UnitKind::Target,
            service: None,
            scheduled: false,
        },
    );

    Ok(Rendered {
        units,
        sysusers,
        manifest: Manifest {
            name: composite.clone(),
            units: manifest_units,
            services: manifest_services,
            degradations,
        },
    })
}

#[derive(Clone)]
struct EdgeClaim {
    unit: String,
    dependency: Option<String>,
    group: String,
    source: String,
    destination: String,
}

fn edge_group(composite: &str, edge: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in composite
        .bytes()
        .chain(std::iter::once(0))
        .chain(edge.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("cix-e-{hash:016x}")
}

fn render_edge_unit(
    edge: &str,
    target: &str,
    runtime: &str,
    group: &str,
    members: &[String],
) -> String {
    format!(
        "[Unit]\nDescription=cix compose Unix edge: {edge}\nPartOf={target}\nBefore={}\n\n[Service]\nType=oneshot\nSlice={}.slice\nExecStart=/bin/sh -c true\nRemainAfterExit=yes\nGroup={group}\nRuntimeDirectory={runtime}\nRuntimeDirectoryMode=2770\nUMask=0007\n",
        members.join(" "),
        target.trim_end_matches(".target")
    )
}

fn render_socket_unit(
    listener: &str,
    service: &str,
    target: &str,
    address: &std::net::SocketAddr,
) -> String {
    format!(
        "[Unit]\nDescription=cix compose listener: {listener} for {service}\nPartOf={target}\nBefore={service}\n\n[Socket]\nListenStream={address}\nFileDescriptorName={listener}\nService={service}\n"
    )
}

fn render_timer_unit(
    service_name: &str,
    service: &str,
    target: &str,
    schedule: &str,
    persistent: Option<bool>,
    jitter: Option<&str>,
) -> String {
    let mut text = format!(
        "[Unit]\nDescription=cix compose schedule: {service_name}\nPartOf={target}\n\n[Timer]\nOnCalendar={schedule}\nUnit={service}\n"
    );
    if let Some(persistent) = persistent {
        text.push_str(&format!("Persistent={persistent}\n"));
    }
    if let Some(jitter) = jitter {
        text.push_str(&format!("RandomizedDelaySec={jitter}\n"));
    }
    text
}

fn add_unit_dependencies(
    text: &str,
    target: &str,
    requires: &BTreeSet<String>,
    after: &BTreeSet<String>,
    extra: &[(String, String)],
) -> String {
    let mut properties = format!("PartOf={target}\n");
    if !requires.is_empty() {
        let requires = requires.iter().cloned().collect::<Vec<_>>().join(" ");
        properties.push_str(&format!("Requires={requires}\n"));
    }
    if !after.is_empty() {
        let after = after.iter().cloned().collect::<Vec<_>>().join(" ");
        properties.push_str(&format!("After={after}\n"));
    }
    for (name, value) in extra {
        properties.push_str(&format!("{name}={value}\n"));
    }
    text.replacen(
        "\n\n[Service]\n",
        &format!("\n{properties}\n[Service]\n"),
        1,
    )
}

struct SharedDirectory {
    host_path: PathBuf,
    members: Vec<String>,
}

fn collect_shared_directories(checked: &CheckResult) -> BTreeMap<String, SharedDirectory> {
    let mut shared = BTreeMap::new();
    for (service, checked_service) in &checked.services {
        for claim in &checked_service.directories {
            let DirectoryBacking::Shared { name } = &claim.backing else {
                continue;
            };
            let entry = shared
                .entry(name.clone())
                .or_insert_with(|| SharedDirectory {
                    host_path: shared_host_path(&checked.compose.name, name),
                    members: Vec::new(),
                });
            entry
                .members
                .push(format!("cix-{}-{service}.service", checked.compose.name));
        }
    }
    for entry in shared.values_mut() {
        entry.members.sort();
        entry.members.dedup();
    }
    shared
}

fn private_service(
    service: &cix_run::spec::Service,
    claims: &[DirectoryClaim],
) -> cix_run::spec::Service {
    let mut private = service.clone();
    private.dirs.state.clear();
    private.dirs.cache.clear();
    private.dirs.logs.clear();
    private.dirs.config.clear();
    private.dirs.run = None;
    private.dirs.data.clear();
    for claim in claims {
        if claim.backing != DirectoryBacking::Private {
            continue;
        }
        match claim.role {
            Some(DirectoryRole::State) => private.dirs.state.push(claim.path.clone()),
            Some(DirectoryRole::Cache) => private.dirs.cache.push(claim.path.clone()),
            Some(DirectoryRole::Logs) => private.dirs.logs.push(claim.path.clone()),
            Some(DirectoryRole::Config) => private.dirs.config.push(claim.path.clone()),
            Some(DirectoryRole::Run) => private
                .dirs
                .run
                .get_or_insert_with(Vec::new)
                .push(claim.path.clone()),
            None => unreachable!("DIR cannot retain private backing"),
        }
    }
    private
}

fn bind_value(source: &Path, destination: &Path, idmap: bool) -> String {
    let mut value = format!("{}:{}", source.display(), destination.display());
    if idmap {
        value.push_str(":idmap");
    }
    value
}

fn shared_group(composite: &str, name: &str) -> String {
    format!("cix-s-{:016x}", stable_hash(composite, name))
}

fn shared_host_path(composite: &str, name: &str) -> PathBuf {
    Path::new("/var/lib/cix-compose")
        .join(composite)
        .join("shared")
        .join(name)
}

fn stable_hash(composite: &str, name: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in composite
        .bytes()
        .chain(std::iter::once(0))
        .chain(name.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn render_shared_directory_unit(
    name: &str,
    target: &str,
    host_path: &Path,
    group: &str,
    members: &[String],
) -> String {
    let relative = host_path
        .strip_prefix("/var/lib")
        .expect("shared path is below /var/lib")
        .display();
    format!(
        "[Unit]\nDescription=cix compose shared directory: {name}\nPartOf={target}\nBefore={}\n\n[Service]\nType=oneshot\nSlice={}.slice\nExecStart=/bin/sh -c true\nRemainAfterExit=yes\nGroup={group}\nStateDirectory={relative}\nStateDirectoryMode=2770\nUMask=0002\n",
        members.join(" "),
        target.trim_end_matches(".target")
    )
}

fn manifest_directory(composite: &str, service: &str, claim: &DirectoryClaim) -> ManifestDirectory {
    let (backing, host_path) = match &claim.backing {
        DirectoryBacking::Private => (
            DirectoryBackingKind::Private,
            private_host_path(
                composite,
                service,
                claim.role.expect("private role"),
                &claim.path,
            ),
        ),
        DirectoryBacking::Host { path, .. } => (DirectoryBackingKind::Host, path.clone()),
        DirectoryBacking::Shared { name } => (
            DirectoryBackingKind::Shared,
            shared_host_path(composite, name),
        ),
    };
    ManifestDirectory {
        path: claim.path.clone(),
        role: claim.role,
        backing,
        host_path,
    }
}

fn private_host_path(composite: &str, service: &str, role: DirectoryRole, path: &Path) -> PathBuf {
    let root = match role {
        DirectoryRole::State => "/var/lib",
        DirectoryRole::Cache => "/var/cache",
        DirectoryRole::Logs => "/var/log",
        DirectoryRole::Config => "/etc",
        DirectoryRole::Run => "/run",
    };
    Path::new(root)
        .join(format!("cix-{composite}-{service}"))
        .join(path.strip_prefix("/").expect("validated absolute path"))
}

fn render_target(composite: &str, wants: &BTreeSet<String>, after: &BTreeSet<String>) -> String {
    let mut text = format!("[Unit]\nDescription=cix compose target: {composite}\n");
    if !wants.is_empty() {
        text.push_str(&format!(
            "Wants={}\n",
            wants.iter().cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    if !after.is_empty() {
        text.push_str(&format!(
            "After={}\n",
            after.iter().cloned().collect::<Vec<_>>().join(" ")
        ));
    }
    text
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut contents = serde_json::to_vec_pretty(value)?;
    contents.push(b'\n');
    fs::write(path, contents).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use cix_run::{config::ResolvedConfig, spec::Spec};

    use super::*;
    use crate::{
        model::{Compose, ComposeService, Edge, Lock, LockedService, Producer, UpdatePolicy},
        resolve::CheckedService,
    };

    fn fixture() -> (tempfile::TempDir, CheckResult, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let compose_path = directory.path().join("compose.json");
        fs::write(&compose_path, b"{\"composeVersion\":1}\n").unwrap();
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                        "start": ["/nix/store/00000000000000000000000000000000-app/bin/app"],
                        "listeners": {"http": {"type": "stream"}},
                        "dirs": {"run": ["/run/app"]}
            }"#,
        )
        .unwrap();
        let web_service = spec.select_service(None).unwrap().1.clone();
        let mut worker_service = web_service.clone();
        worker_service.dirs.state.push("/var/lib/app".into());
        let web_config =
            ResolvedConfig::resolve(&web_service, &[], &["http=127.0.0.1:8080".into()]).unwrap();
        let worker_config =
            ResolvedConfig::resolve(&worker_service, &[], &["http=127.0.0.1:8081".into()]).unwrap();
        let compose = Compose {
            compose_version: 1,
            name: "stack".into(),
            services: BTreeMap::from([
                (
                    "web".into(),
                    ComposeService {
                        item: "web:v1".into(),
                        update: UpdatePolicy::Pin,
                        env: BTreeMap::new(),
                        bind: BTreeMap::new(),
                        dirs: BTreeMap::new(),
                        identity: None,
                        schedule: None,
                        persistent: None,
                        jitter: None,
                        shm: None,
                    },
                ),
                (
                    "worker".into(),
                    ComposeService {
                        item: "worker:v1".into(),
                        update: UpdatePolicy::Pin,
                        env: BTreeMap::new(),
                        bind: BTreeMap::new(),
                        dirs: BTreeMap::new(),
                        identity: None,
                        schedule: None,
                        persistent: None,
                        jitter: None,
                        shm: None,
                    },
                ),
            ]),
            log_namespace: false,
            edges: BTreeMap::from([(
                "shared".into(),
                Edge {
                    producer: Producer {
                        service: "worker".into(),
                        path: "/run/app".into(),
                    },
                    consumers: BTreeMap::from([("web".into(), Default::default())]),
                },
            )]),
        };
        let lock = Lock {
            services: BTreeMap::from([
                (
                    "web".into(),
                    LockedService {
                        reference: "web:v1".into(),
                        store_path: "/nix/store/00000000000000000000000000000000-web".into(),
                        nar_hash: "sha256-web".into(),
                    },
                ),
                (
                    "worker".into(),
                    LockedService {
                        reference: "worker:v1".into(),
                        store_path: "/nix/store/11111111111111111111111111111111-worker".into(),
                        nar_hash: "sha256-worker".into(),
                    },
                ),
            ]),
        };
        let checked = CheckResult {
            compose,
            lock,
            warnings: Vec::new(),
            services: BTreeMap::from([
                (
                    "web".into(),
                    CheckedService {
                        store_path: "/nix/store/00000000000000000000000000000000-web".into(),
                        item_service: "app".into(),
                        spec: web_service,
                        config: web_config,
                        directories: vec![DirectoryClaim {
                            path: "/run/app".into(),
                            declared_role: Some(DirectoryRole::Run),
                            role: Some(DirectoryRole::Run),
                            writable: true,
                            backing: DirectoryBacking::Private,
                        }],
                    },
                ),
                (
                    "worker".into(),
                    CheckedService {
                        store_path: "/nix/store/11111111111111111111111111111111-worker".into(),
                        item_service: "app".into(),
                        spec: worker_service,
                        config: worker_config,
                        directories: vec![
                            DirectoryClaim {
                                path: "/run/app".into(),
                                declared_role: Some(DirectoryRole::Run),
                                role: Some(DirectoryRole::Run),
                                writable: true,
                                backing: DirectoryBacking::Private,
                            },
                            DirectoryClaim {
                                path: "/var/lib/app".into(),
                                declared_role: Some(DirectoryRole::State),
                                role: Some(DirectoryRole::State),
                                writable: true,
                                backing: DirectoryBacking::Private,
                            },
                        ],
                    },
                ),
            ]),
        };
        (directory, checked, compose_path)
    }

    #[test]
    fn small_composite_units_match_golden_files() {
        let (directory, checked, compose_path) = fixture();
        let generation = directory.path().join("generation");
        render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        for name in [
            "cix-stack-web.service",
            "cix-stack-edge-shared.service",
            "cix-stack-web-http.socket",
            "cix-stack.target",
        ] {
            let actual = fs::read_to_string(generation.join("units").join(name)).unwrap();
            let fixture = format!("tests/fixtures/{name}");
            let expected =
                fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(&fixture)).unwrap();
            assert_eq!(actual, expected, "{name}");
        }
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(!web.contains("RuntimeDirectory="));
    }

    #[test]
    fn scheduled_apps_render_timer_snapshots_and_are_wanted_instead_of_services() {
        let (directory, mut checked, compose_path) = fixture();
        let worker = checked.compose.services.get_mut("worker").unwrap();
        worker.schedule = Some("Mon *-*-* 12:00:00".into());
        let generation = directory.path().join("generation");
        render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        let timer = fs::read_to_string(generation.join("units/cix-stack-worker.timer")).unwrap();
        let expected = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cix-stack-worker.timer"),
        )
        .unwrap();
        assert_eq!(timer, expected);
        let target = fs::read_to_string(generation.join("units/cix-stack.target")).unwrap();
        assert!(target.contains("cix-stack-worker.timer"), "{target}");
        assert!(!target.contains("cix-stack-worker.service"), "{target}");

        checked
            .compose
            .services
            .get_mut("worker")
            .unwrap()
            .persistent = Some(true);
        checked.compose.services.get_mut("worker").unwrap().jitter = Some("5m".into());
        let configured = directory.path().join("configured");
        render_generation(
            &checked,
            &compose_path,
            &configured,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        let timer = fs::read_to_string(configured.join("units/cix-stack-worker.timer")).unwrap();
        let expected = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/cix-stack-worker-persistent.timer"),
        )
        .unwrap();
        assert_eq!(timer, expected);
    }

    #[test]
    fn compose_shm_override_wins_over_the_item() {
        let (directory, mut checked, compose_path) = fixture();
        checked.compose.services.get_mut("web").unwrap().shm = Some("96M".into());
        let generation = directory.path().join("generation");
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        let unit = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(
            unit.contains("TemporaryFileSystem=/dev/shm:size=96M"),
            "{unit}"
        );
        assert_eq!(manifest.services["web"].shm.as_deref(), Some("96M"));
    }

    #[test]
    fn host_shared_and_reclassified_directories_render_their_distinct_mechanisms() {
        let (directory, mut checked, compose_path) = fixture();
        checked.compose.services.get_mut("web").unwrap().identity = Some("hostuser".into());
        checked.services.get_mut("web").unwrap().directories = vec![
            DirectoryClaim {
                path: "/run/app".into(),
                declared_role: Some(DirectoryRole::Run),
                role: Some(DirectoryRole::Run),
                writable: true,
                backing: DirectoryBacking::Host {
                    path: "/tank/web".into(),
                    idmap: true,
                },
            },
            DirectoryClaim {
                path: "/var/lib/web-shared".into(),
                declared_role: Some(DirectoryRole::State),
                role: Some(DirectoryRole::State),
                writable: true,
                backing: DirectoryBacking::Shared {
                    name: "uploads".into(),
                },
            },
        ];
        checked.services.get_mut("worker").unwrap().directories = vec![
            DirectoryClaim {
                path: "/run/app".into(),
                declared_role: Some(DirectoryRole::Run),
                role: Some(DirectoryRole::Run),
                writable: true,
                backing: DirectoryBacking::Private,
            },
            DirectoryClaim {
                path: "/var/lib/app".into(),
                declared_role: Some(DirectoryRole::State),
                role: Some(DirectoryRole::Cache),
                writable: true,
                backing: DirectoryBacking::Private,
            },
            DirectoryClaim {
                path: "/var/lib/worker-shared".into(),
                declared_role: Some(DirectoryRole::State),
                role: Some(DirectoryRole::State),
                writable: true,
                backing: DirectoryBacking::Shared {
                    name: "uploads".into(),
                },
            },
        ];
        let generation = directory.path().join("generation");
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(web.contains("User=hostuser"), "{web}");
        assert!(web.contains("DynamicUser=no"), "{web}");
        assert!(web.contains("BindPaths=/tank/web:/run/app:idmap"), "{web}");
        assert!(web.contains("RequiresMountsFor=/tank/web"), "{web}");
        assert!(web.contains("UMask=0002"), "{web}");
        let worker = fs::read_to_string(generation.join("units/cix-stack-worker.service")).unwrap();
        assert!(
            worker.contains("CacheDirectory=cix-stack-worker cix-stack-worker/var/lib/app"),
            "{worker}"
        );
        assert!(
            !worker.contains("StateDirectory=cix-stack-worker cix-stack-worker/var/lib/app"),
            "{worker}"
        );
        let shared =
            fs::read_to_string(generation.join("units/cix-stack-shared-uploads.service")).unwrap();
        assert!(
            shared.contains("StateDirectory=cix-compose/stack/shared/uploads"),
            "{shared}"
        );
        assert!(shared.contains("StateDirectoryMode=2770"), "{shared}");
        assert!(manifest.services["web"]
            .directories
            .iter()
            .any(|directory| directory.backing == DirectoryBackingKind::Host));
        assert!(manifest.services["worker"]
            .directories
            .iter()
            .any(|directory| directory.backing == DirectoryBackingKind::Shared));
    }

    #[test]
    fn rendering_is_byte_deterministic() {
        let (directory, checked, compose_path) = fixture();
        let left = directory.path().join("left");
        let right = directory.path().join("right");
        render_generation(
            &checked,
            &compose_path,
            &left,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        render_generation(
            &checked,
            &compose_path,
            &right,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        for relative in [
            "compose.json",
            "cix.lock",
            "manifest.json",
            "sysusers.d/cix-stack.conf",
            "units/cix-stack-web.service",
            "units/cix-stack-worker.service",
            "units/cix-stack-edge-shared.service",
            "units/cix-stack-web-http.socket",
            "units/cix-stack-worker-http.socket",
            "units/cix-stack.slice",
            "units/cix-stack.target",
        ] {
            assert_eq!(
                fs::read(left.join(relative)).unwrap(),
                fs::read(right.join(relative)).unwrap(),
                "{relative}"
            );
        }
    }

    #[test]
    fn identical_inputs_have_the_same_generation_store_path() {
        let (_directory, checked, compose_path) = fixture();
        let capabilities = HostCapabilities::all_supported();
        let left = build_generation(&checked, &compose_path, &capabilities).unwrap();
        let right = build_generation(&checked, &compose_path, &capabilities).unwrap();
        assert_eq!(left.store_path, right.store_path);
    }

    #[test]
    fn unsupported_host_records_one_minimal_degradation() {
        let (directory, checked, compose_path) = fixture();
        let generation = directory.path().join("generation");
        let capabilities = HostCapabilities::private_pids_with_persistent_directories_unsupported(
            "synthetic realization failure",
        );
        let manifest =
            render_generation(&checked, &compose_path, &generation, &capabilities).unwrap();

        assert_eq!(
            manifest.degradations,
            vec![ManifestDegradation {
                unit: "cix-stack-worker.service".into(),
                property: "PrivatePIDs=yes".into(),
                reason: "synthetic realization failure".into(),
            }]
        );
        let worker = fs::read_to_string(generation.join("units/cix-stack-worker.service")).unwrap();
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(!worker.contains("PrivatePIDs="));
        assert!(web.contains("PrivatePIDs=yes"));
    }

    #[test]
    fn capable_host_records_no_degradation() {
        let (directory, checked, compose_path) = fixture();
        let generation = directory.path().join("generation");
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        assert!(manifest.degradations.is_empty());
        let worker = fs::read_to_string(generation.join("units/cix-stack-worker.service")).unwrap();
        assert!(worker.contains("PrivatePIDs=yes"));
    }

    #[test]
    fn compose_services_stamp_selectors_and_opt_into_a_log_namespace() {
        let (directory, mut checked, compose_path) = fixture();
        checked.compose.log_namespace = true;
        let generation = directory.path().join("generation");
        render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(web.contains("LogExtraFields=CIX_COMPOSITE=stack CIX_SERVICE=web CIX_ITEM=/nix/store/00000000000000000000000000000000-web"), "{web}");
        assert!(web.contains("LogNamespace=cix-stack"), "{web}");
    }
}
