use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cix_run::{
    capabilities::HostCapabilities,
    closed_root::options_for_unit,
    unit::{compile_unit_for_host, UnitCompileOptions, UnitMode, UnitNaming},
};
use serde::{Deserialize, Serialize};

use crate::{
    directories::{
        bind_value, collect_shared_directories, manifest_directory, private_service,
        render_shared_directory_unit, shared_group, DirectoryBacking, DirectoryRole,
    },
    network::{
        default_leases, filesystem_segment, namespace_name, netns_unit_name, parent_path,
        pod_address, publish_socket_name, render_netns_unit, render_proxy_unit, render_socket_unit,
        veth_name, PodLease, PublishKind,
    },
    resolve::CheckResult,
    unit_path,
};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Manifest {
    pub name: String,
    pub units: BTreeMap<String, ManifestUnit>,
    pub services: BTreeMap<String, ManifestService>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pods: BTreeMap<String, ManifestPod>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub network_files: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub closed_root: bool,
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
    Netns,
    Proxy,
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
pub struct ManifestPod {
    pub unit: String,
    pub namespace: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub members: BTreeSet<String>,
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
    build_generation_with_closed_root(checked, compose_path, capabilities, false)
}

pub fn build_generation_with_closed_root(
    checked: &CheckResult,
    compose_path: &Path,
    capabilities: &HostCapabilities,
    closed_root: bool,
) -> Result<BuiltGeneration> {
    build_generation_with_leases(
        checked,
        compose_path,
        capabilities,
        closed_root,
        &default_leases(checked),
        Path::new("/etc/resolv.conf"),
    )
}

pub(crate) fn build_generation_with_leases(
    checked: &CheckResult,
    compose_path: &Path,
    capabilities: &HostCapabilities,
    closed_root: bool,
    leases: &BTreeMap<String, PodLease>,
    resolver_source: &Path,
) -> Result<BuiltGeneration> {
    let temporary = tempfile::tempdir().context("creating compose generation workspace")?;
    let generation = temporary
        .path()
        .join(format!("cix-compose-{}-generation", checked.compose.name));
    let manifest = render_generation_with_leases(
        checked,
        compose_path,
        &generation,
        capabilities,
        closed_root,
        leases,
        resolver_source,
    )?;
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
    render_generation_with_closed_root(checked, compose_path, generation, capabilities, false)
}

pub fn render_generation_with_closed_root(
    checked: &CheckResult,
    compose_path: &Path,
    generation: &Path,
    capabilities: &HostCapabilities,
    closed_root: bool,
) -> Result<Manifest> {
    render_generation_with_leases(
        checked,
        compose_path,
        generation,
        capabilities,
        closed_root,
        &default_leases(checked),
        Path::new("/etc/resolv.conf"),
    )
}

fn render_generation_with_leases(
    checked: &CheckResult,
    compose_path: &Path,
    generation: &Path,
    capabilities: &HostCapabilities,
    closed_root: bool,
    leases: &BTreeMap<String, PodLease>,
    resolver_source: &Path,
) -> Result<Manifest> {
    let units_dir = generation.join("units");
    let sysusers_dir = generation.join("sysusers.d");
    let network_dir = generation.join("network");
    fs::create_dir_all(&units_dir)?;
    fs::create_dir_all(&sysusers_dir)?;
    fs::create_dir_all(&network_dir)?;
    let _ = compose_path;
    write_json(&generation.join("compose.json"), &checked.compose)?;
    write_json(&generation.join("cix.lock"), &checked.lock)?;

    let rendered = render_units(checked, capabilities, closed_root, leases, resolver_source)?;
    for (name, text) in &rendered.units {
        fs::write(units_dir.join(name), text)
            .with_context(|| format!("writing generated unit {name}"))?;
    }
    fs::write(
        sysusers_dir.join(format!("cix-{}.conf", checked.compose.name)),
        rendered.sysusers,
    )?;
    for (name, text) in &rendered.network_files {
        fs::write(network_dir.join(name), text)
            .with_context(|| format!("writing generated networkd fragment {name}"))?;
    }
    write_json(&generation.join("manifest.json"), &rendered.manifest)?;
    Ok(rendered.manifest)
}

struct Rendered {
    units: BTreeMap<String, String>,
    network_files: BTreeMap<String, String>,
    sysusers: String,
    manifest: Manifest,
}

fn render_units(
    checked: &CheckResult,
    capabilities: &HostCapabilities,
    closed_root: bool,
    leases: &BTreeMap<String, PodLease>,
    resolver_source: &Path,
) -> Result<Rendered> {
    let composite = &checked.compose.name;
    let target = format!("cix-{composite}.target");
    let prefix = format!("cix-{composite}");
    let mut units = BTreeMap::new();
    let mut manifest_units = BTreeMap::new();
    let mut network_files = BTreeMap::new();
    let mut service_edges: BTreeMap<&str, Vec<EdgeClaim>> = BTreeMap::new();
    let mut sysusers = String::new();
    let mut target_wants = BTreeSet::new();
    let mut target_after = BTreeSet::new();
    let mut degradations = Vec::new();
    let shared = collect_shared_directories(checked);
    let mut manifest_pods = BTreeMap::new();

    if checked.pods.values().any(|pod| pod.egress) {
        let bridge_fragment = format!("80-{prefix}-cix0");
        network_files.insert(
            format!("{bridge_fragment}.netdev"),
            "[NetDev]\nName=cix0\nKind=bridge\n".into(),
        );
        network_files.insert(
            format!("{bridge_fragment}.network"),
            "[Match]\nName=cix0\n\n[Network]\nAddress=10.231.0.1/16\nIPMasquerade=ipv4\nIPv4Forwarding=yes\nLinkLocalAddressing=no\nConfigureWithoutCarrier=yes\n".into(),
        );
    }
    for (pod_path, pod) in &checked.pods {
        let namespace = namespace_name(composite, pod_path);
        let unit = netns_unit_name(composite, pod_path);
        let members = checked
            .services
            .iter()
            .filter(|(_, service)| service.pod.as_deref() == Some(pod_path.as_str()))
            .map(|(service, _)| service_unit_name(composite, service))
            .collect::<Vec<_>>();
        let lease = pod
            .egress
            .then(|| {
                leases
                    .get(pod_path)
                    .copied()
                    .context("missing egress IPAM lease")
            })
            .transpose()?;
        if lease.is_some() {
            let host_link = veth_name(composite, pod_path, 'h');
            network_files.insert(
                format!("80-{host_link}.network"),
                format!(
                    "[Match]\nName={host_link}\n\n[Network]\nBridge=cix0\nLinkLocalAddressing=no\n"
                ),
            );
        }
        units.insert(
            unit.clone(),
            render_netns_unit(pod_path, &namespace, &target, &members, lease, composite),
        );
        manifest_units.insert(
            unit.clone(),
            ManifestUnit {
                kind: UnitKind::Netns,
                service: None,
                scheduled: false,
            },
        );
        target_wants.insert(unit.clone());
        target_after.insert(unit.clone());
        manifest_pods.insert(
            pod_path.clone(),
            ManifestPod {
                unit,
                namespace,
                address: lease.map(|lease| pod_address(lease).to_string()),
                members: members.into_iter().collect(),
            },
        );
    }

    for (name, shared) in &shared {
        let unit = format!("{prefix}-shared-{}.service", unit_path(name));
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

    for (edge_name, edge) in &checked.edges {
        let edge_unit = format!("{prefix}-edge-{}.service", unit_path(edge_name));
        let runtime = format!("{prefix}-edge-{}", filesystem_segment(edge_name));
        let group = edge_group(composite, edge_name);
        sysusers.push_str(&format!("g {group} - -\n"));
        let members = std::iter::once(edge.producer.child.as_str())
            .chain(edge.consumers.keys().map(String::as_str))
            .map(|service| service_unit_name(composite, service))
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
            .entry(&edge.producer.child)
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
                dependency: Some(service_unit_name(composite, &edge.producer.child)),
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
        let service_unit = service_unit_name(composite, service_name);
        let declaration = &checked_service.declaration;
        let service_slice = service_slice_name(composite, service_name);
        let service_segment = unit_path(service_name);
        let claims = service_edges
            .get(service_name.as_str())
            .cloned()
            .unwrap_or_default();
        let mut extra_properties = Vec::new();
        let mut unit_properties = Vec::new();
        let pod_unit = checked_service.pod.as_ref().map(|pod| {
            let namespace = namespace_name(composite, pod);
            extra_properties.push((
                "NetworkNamespacePath".into(),
                format!("/run/netns/{namespace}"),
            ));
            if !checked_service.egress {
                extra_properties.extend([
                    ("IPAddressAllow".into(), "localhost".into()),
                    ("IPAddressDeny".into(), "any".into()),
                ]);
            }
            netns_unit_name(composite, pod)
        });
        if checked_service.pod.is_some() && checked_service.egress && !closed_root {
            extra_properties.push((
                "BindReadOnlyPaths".into(),
                format!("{}:/etc/resolv.conf", resolver_source.display()),
            ));
        }
        if checked.compose.log_namespace {
            extra_properties.push(("LogNamespace".into(), format!("cix-{composite}")));
        }
        for (secret_name, source) in &checked_service.secrets {
            let path = source
                .file
                .as_ref()
                .or(source.encrypted.as_ref())
                .expect("validated source");
            extra_properties.push((
                if source.encrypted.is_some() {
                    "LoadCredentialEncrypted".into()
                } else {
                    "LoadCredential".into()
                },
                format!("{secret_name}:{}", path.display()),
            ));
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
                    let unit = format!("{prefix}-shared-{}.service", unit_path(name));
                    unit_properties.push(("Requires".into(), unit.clone()));
                    unit_properties.push(("After".into(), unit));
                }
            }
        }
        let sockets = checked_service
            .config
            .listeners
            .keys()
            .map(|listener| format!("{prefix}-{service_segment}-{listener}.socket"))
            .chain(
                checked
                    .publishes
                    .iter()
                    .filter(|published| {
                        published.service == *service_name
                            && published.kind == PublishKind::Listener
                    })
                    .map(|published| publish_socket_name(composite, published)),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if !sockets.is_empty() {
            extra_properties.push(("Sockets".into(), sockets.join(" ")));
        }
        let mut compiled_service =
            private_service(&checked_service.spec, &checked_service.directories);
        if checked_service.pod.is_some() {
            compiled_service.network = Some(cix_run::spec::Network::Host);
        }
        compiled_service.claims.retain(
            |claim| !matches!(claim, cix_run::spec::Claim::Named(name) if name == "egress"),
        );
        compiled_service.egress = false;
        if checked_service.egress {
            compiled_service
                .claims
                .push(cix_run::spec::Claim::Named("egress".into()));
        }
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
            &service_segment,
            &compiled_service,
            &checked_service.config,
            UnitMode::System,
            &UnitCompileOptions {
                naming: UnitNaming {
                    unit: service_unit.clone(),
                    slice: service_slice,
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
                closed_root: closed_root
                    .then(|| {
                        let options = options_for_unit(&service_unit, false)?;
                        Ok::<_, anyhow::Error>(
                            if checked_service.pod.is_some() && checked_service.egress {
                                options.with_resolver_source(resolver_source)
                            } else {
                                options
                            },
                        )
                    })
                    .transpose()?,
            },
            capabilities,
        )
        .with_context(|| format!("compiling children.{service_name}"))?;
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
            .chain(pod_unit.iter().cloned())
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
            let timer = format!("{prefix}-{service_segment}.timer");
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
            let socket = format!("{prefix}-{service_segment}-{listener}.socket");
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
        let locked = &checked.lock.paths[service_name];
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

    for published in &checked.publishes {
        let socket = publish_socket_name(composite, published);
        let service = service_unit_name(composite, &published.service);
        match published.kind {
            PublishKind::Listener => {
                units.insert(
                    socket.clone(),
                    render_socket_unit(&published.surface, &service, &target, &published.address),
                );
            }
            PublishKind::Port { target: port } => {
                let proxy = format!(
                    "{prefix}-publish-{}-proxy.service",
                    unit_path(&published.name)
                );
                let netns = netns_unit_name(composite, &published.pod);
                let namespace = namespace_name(composite, &published.pod);
                units.insert(
                    socket.clone(),
                    render_socket_unit(&published.surface, &proxy, &target, &published.address),
                );
                units.insert(
                    proxy.clone(),
                    render_proxy_unit(
                        &published.name,
                        &socket,
                        &netns,
                        &namespace,
                        &target,
                        &group_slice_name(composite, parent_path(&published.service)),
                        port,
                    ),
                );
                manifest_units.insert(
                    proxy,
                    ManifestUnit {
                        kind: UnitKind::Proxy,
                        service: Some(published.service.clone()),
                        scheduled: false,
                    },
                );
            }
        }
        manifest_units.insert(
            socket.clone(),
            ManifestUnit {
                kind: UnitKind::Socket,
                service: Some(published.service.clone()),
                scheduled: false,
            },
        );
        target_wants.insert(socket);
    }

    for group_path in &checked.groups {
        let group_slice = group_slice_name(composite, group_path);
        let description = if group_path.is_empty() {
            composite.as_str()
        } else {
            group_path.as_str()
        };
        units.insert(
            group_slice.clone(),
            format!("[Unit]\nDescription=cix compose slice: {description}\n"),
        );
        manifest_units.insert(
            group_slice,
            ManifestUnit {
                kind: UnitKind::Slice,
                service: None,
                scheduled: false,
            },
        );
    }
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
        network_files: network_files.clone(),
        sysusers,
        manifest: Manifest {
            name: composite.clone(),
            units: manifest_units,
            services: manifest_services,
            pods: manifest_pods,
            network_files: network_files.keys().cloned().collect(),
            closed_root,
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

fn service_unit_name(composite: &str, path: &str) -> String {
    format!("cix-{composite}-{}.service", unit_path(path))
}

fn group_slice_name(composite: &str, path: &str) -> String {
    if path.is_empty() {
        format!("cix-{composite}.slice")
    } else {
        format!("cix-{composite}-{}.slice", unit_path(path))
    }
}

fn service_slice_name(composite: &str, path: &str) -> String {
    let parent = path
        .rsplit_once('/')
        .map(|(parent, _)| parent)
        .unwrap_or("");
    group_slice_name(composite, parent)
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
        directories::{DirectoryClaim, DirectoryRole},
        model::{
            Child, Compose, ComposeService, Edge, Lock, LockedService, Producer, UpdatePolicy,
        },
        resolve::CheckedService,
    };

    fn fixture() -> (tempfile::TempDir, CheckResult, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let compose_path = directory.path().join("compose.json");
        fs::write(&compose_path, b"{\"cixCompose\":1}\n").unwrap();
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
        let web_declaration = ComposeService {
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
            egress: None,
        };
        let worker_declaration = ComposeService {
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
            egress: None,
        };
        let edges = BTreeMap::from([(
            "shared".into(),
            Edge {
                producer: Producer {
                    child: "worker".into(),
                    path: "/run/app".into(),
                },
                consumers: BTreeMap::from([("web".into(), Default::default())]),
            },
        )]);
        let compose = Compose {
            cix_compose: 1,
            name: "stack".into(),
            children: BTreeMap::from([
                ("web".into(), Child::Item(web_declaration.clone())),
                ("worker".into(), Child::Item(worker_declaration.clone())),
            ]),
            log_namespace: false,
            secrets: BTreeMap::new(),
            edges: edges.clone(),
            network: None,
            publish: BTreeMap::new(),
        };
        let lock = Lock {
            paths: BTreeMap::from([
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
            edges,
            groups: BTreeSet::from([String::new()]),
            pods: BTreeMap::new(),
            publishes: Vec::new(),
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
                        secrets: BTreeMap::new(),
                        declaration: web_declaration,
                        pod: None,
                        egress: false,
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
                        secrets: BTreeMap::new(),
                        declaration: worker_declaration,
                        pod: None,
                        egress: false,
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
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();
        assert!(manifest.pods.is_empty());
        assert!(manifest.network_files.is_empty());
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
        assert!(!web.contains("NetworkNamespacePath="));
    }

    #[test]
    fn edge_runtime_paths_do_not_reuse_systemd_unit_escaping() {
        let (directory, mut checked, compose_path) = fixture();
        let edge = checked.edges.remove("shared").unwrap();
        checked.edges.insert("cross-boundary".into(), edge);
        let generation = directory.path().join("hyphenated-edge-generation");
        render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        let owner =
            fs::read_to_string(generation.join(r"units/cix-stack-edge-cross\x2dboundary.service"))
                .unwrap();
        assert!(
            owner.contains("RuntimeDirectory=cix-stack-edge-cross_2dboundary"),
            "{owner}"
        );
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(
            web.contains("BindPaths=/run/cix-stack-edge-cross_2dboundary:/run/app:rbind"),
            "{web}"
        );
    }

    #[test]
    fn namespace_paths_do_not_reuse_systemd_unit_escaping() {
        assert_ne!(
            namespace_name("stack", "a-b"),
            namespace_name("stack", "a/b")
        );
        assert_eq!(
            netns_unit_name("stack", "a-b"),
            r"cix-stack-a\x2db-netns.service"
        );
        assert_eq!(namespace_name("stack", "a-b"), "cix-stack-a_2db-netns");
    }

    #[test]
    fn two_level_paths_name_units_and_nested_slice_snapshot() {
        let (directory, mut checked, compose_path) = fixture();
        let web = checked.services.remove("web").unwrap();
        checked.services.insert("tier/web".into(), web);
        let locked = checked.lock.paths.remove("web").unwrap();
        checked.lock.paths.insert("tier/web".into(), locked);
        checked.groups.insert("tier".into());
        checked.edges.clear();
        let generation = directory.path().join("nested-generation");
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        let unit_name = "cix-stack-tier-web.service";
        let unit = fs::read_to_string(generation.join("units").join(unit_name)).unwrap();
        assert!(unit.contains("Slice=cix-stack-tier.slice"), "{unit}");
        assert!(unit.contains("CIX_SERVICE=tier/web"), "{unit}");
        assert!(manifest.units.contains_key(unit_name));
        assert!(manifest.units.contains_key("cix-stack.slice"));
        assert!(manifest.units.contains_key("cix-stack-tier.slice"));
        let slice = fs::read_to_string(generation.join("units/cix-stack-tier.slice")).unwrap();
        let expected = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cix-stack-tier.slice"),
        )
        .unwrap();
        assert_eq!(slice, expected);
    }

    #[test]
    fn closed_root_generation_marks_manifest_and_seals_each_service() {
        let (directory, checked, compose_path) = fixture();
        let generation = directory.path().join("closed-root-generation");
        let manifest = render_generation_with_closed_root(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
            true,
        )
        .unwrap();
        assert!(manifest.closed_root);
        for service in ["web", "worker"] {
            let unit_name = format!("cix-stack-{service}.service");
            let unit = fs::read_to_string(generation.join("units").join(&unit_name)).unwrap();
            assert!(
                unit.contains(&format!("RootDirectory=/run/cix/closed-roots/{unit_name}")),
                "{unit}"
            );
            assert!(unit.contains("MountAPIVFS=yes"), "{unit}");
            assert!(unit.contains("BindReadOnlyPaths=/nix/store"), "{unit}");
            assert!(unit.contains("/nss/passwd:/etc/passwd"), "{unit}");
            assert!(unit.contains("PrivateUsers=yes"), "{unit}");
        }
    }

    #[test]
    fn scheduled_apps_render_timer_snapshots_and_are_wanted_instead_of_services() {
        let (directory, mut checked, compose_path) = fixture();
        let worker = &mut checked.services.get_mut("worker").unwrap().declaration;
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
            .services
            .get_mut("worker")
            .unwrap()
            .declaration
            .persistent = Some(true);
        checked
            .services
            .get_mut("worker")
            .unwrap()
            .declaration
            .jitter = Some("5m".into());
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
        checked.services.get_mut("web").unwrap().declaration.shm = Some("96M".into());
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
        checked
            .services
            .get_mut("web")
            .unwrap()
            .declaration
            .identity = Some("hostuser".into());
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
        let shared_group = shared_group("stack", "uploads");
        assert!(
            web.lines().any(|line| {
                line.strip_prefix("SupplementaryGroups=")
                    .is_some_and(|groups| {
                        groups.split_whitespace().any(|group| group == shared_group)
                    })
            }),
            "{web}"
        );
        let worker = fs::read_to_string(generation.join("units/cix-stack-worker.service")).unwrap();
        assert!(
            worker.lines().any(|line| {
                line.strip_prefix("SupplementaryGroups=")
                    .is_some_and(|groups| {
                        groups.split_whitespace().any(|group| group == shared_group)
                    })
            }),
            "{worker}"
        );
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
        assert!(!shared.contains(r"\n"), "{shared}");
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

    #[test]
    fn pod_units_attach_members_render_egress_and_choose_publish_tiers() {
        let (directory, mut checked, compose_path) = fixture();
        checked.pods.insert(
            String::new(),
            crate::network::CheckedPod {
                path: String::new(),
                egress: true,
            },
        );
        for service in checked.services.values_mut() {
            service.pod = Some(String::new());
            service.config.listeners.clear();
        }
        checked.services.get_mut("worker").unwrap().egress = true;
        checked.publishes = vec![
            crate::network::CheckedPublish {
                name: "fd".into(),
                service: "web".into(),
                surface: "http".into(),
                address: "127.0.0.1:18080".parse().unwrap(),
                pod: String::new(),
                kind: PublishKind::Listener,
            },
            crate::network::CheckedPublish {
                name: "tcp".into(),
                service: "worker".into(),
                surface: "http".into(),
                address: "127.0.0.1:18081".parse().unwrap(),
                pod: String::new(),
                kind: PublishKind::Port { target: 8080 },
            },
        ];
        let generation = directory.path().join("generation-pod");
        let manifest = render_generation(
            &checked,
            &compose_path,
            &generation,
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        let netns = fs::read_to_string(generation.join("units/cix-stack-netns.service")).unwrap();
        assert!(netns.contains("ip netns add cix-stack-netns"), "{netns}");
        assert!(
            netns.contains("route replace default via 10.231.0.1"),
            "{netns}"
        );
        let web = fs::read_to_string(generation.join("units/cix-stack-web.service")).unwrap();
        assert!(
            web.contains("NetworkNamespacePath=/run/netns/cix-stack-netns"),
            "{web}"
        );
        assert!(web.contains("IPAddressDeny=any"), "{web}");
        let worker = fs::read_to_string(generation.join("units/cix-stack-worker.service")).unwrap();
        assert!(
            worker.contains("NetworkNamespacePath=/run/netns/cix-stack-netns"),
            "{worker}"
        );
        assert!(!worker.contains("IPAddressDeny=any"), "{worker}");
        assert!(
            worker.contains("BindReadOnlyPaths=/etc/resolv.conf:/etc/resolv.conf"),
            "{worker}"
        );
        let fd = fs::read_to_string(generation.join("units/cix-stack-publish-fd.socket")).unwrap();
        assert!(fd.contains("Service=cix-stack-web.service"), "{fd}");
        let proxy =
            fs::read_to_string(generation.join("units/cix-stack-publish-tcp-proxy.service"))
                .unwrap();
        let fixture = |name: &str| {
            fs::read_to_string(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures")
                    .join(name),
            )
            .unwrap()
        };
        assert_eq!(netns, fixture("cix-stack-netns.service"));
        assert_eq!(web, fixture("cix-stack-web-pod.service"));
        assert_eq!(proxy, fixture("cix-stack-publish-tcp-proxy.service"));
        assert_eq!(
            fs::read_to_string(generation.join("network/80-cix-stack-cix0.network")).unwrap(),
            fixture("80-cix-stack-cix0.network")
        );
        assert_eq!(
            fs::read_to_string(generation.join("network/80-cxaed6a92dh.network")).unwrap(),
            fixture("80-cxaed6a92dh.network")
        );
        assert!(
            proxy.contains("systemd-socket-proxyd 127.0.0.1:8080"),
            "{proxy}"
        );
        assert!(
            proxy.contains("JoinsNamespaceOf=cix-stack-netns.service"),
            "{proxy}"
        );
        assert!(manifest.network_files.contains("80-cix-stack-cix0.network"));
    }
}
