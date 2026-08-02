//! Compose directory declarations, claim materialization, and unit projections.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{bail, Result};
use cix_run::spec::Service;
use serde::{Deserialize, Serialize};

use crate::{
    generation::{DirectoryBackingKind, ManifestDirectory},
    model::ComposeService,
    resolve::{CheckResult, CheckedService},
    unit_path,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DirectoryMaterialization {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<String>,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub role: Option<DirectoryRole>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub idmap: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub write: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum DirectoryRole {
    State,
    Cache,
    Logs,
    Config,
    Run,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryClaim {
    pub path: PathBuf,
    /// `None` denotes the undecorated manifest `DIR` claim.
    pub declared_role: Option<DirectoryRole>,
    pub role: Option<DirectoryRole>,
    pub writable: bool,
    pub backing: DirectoryBacking,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DirectoryBacking {
    Private,
    Host { path: PathBuf, idmap: bool },
    Shared { name: String },
}

pub(crate) fn materialize_directories(
    service_name: &str,
    declaration: &ComposeService,
    service: &Service,
    warnings: &mut Vec<String>,
) -> Result<Vec<DirectoryClaim>> {
    let mut declared = BTreeMap::new();
    for (role, paths) in [
        (DirectoryRole::State, service.dirs.state.as_slice()),
        (DirectoryRole::Cache, service.dirs.cache.as_slice()),
        (DirectoryRole::Logs, service.dirs.logs.as_slice()),
        (DirectoryRole::Config, service.dirs.config.as_slice()),
        (
            DirectoryRole::Run,
            service.dirs.run.as_deref().unwrap_or_default(),
        ),
    ] {
        for path in paths {
            declared.insert(path.clone(), (Some(role), true));
        }
    }
    for data in &service.dirs.data {
        declared.insert(data.path.clone(), (None, !data.ro));
    }

    let mut claims = Vec::new();
    for (path, (declared_role, writable)) in &declared {
        let materialization = declaration.dirs.get(path);
        if declared_role.is_none() && materialization.is_none() {
            bail!(
                "services.{service_name}.dirs.{}: DIR declares operator-supplied data; provide host or shared backing, or for a cix-managed dir pick STATEDIR/CACHEDIR/LOGDIR/RUNDIR",
                path.display()
            );
        }
        if materialization.is_some_and(|materialization| materialization.write) {
            bail!(
                "services.{service_name}.dirs.{}.write: write is only for an undeclared extra operator bind",
                path.display()
            );
        }
        claims.push(directory_claim(
            service_name,
            path,
            *declared_role,
            *writable,
            materialization,
            declaration.identity.as_deref(),
            warnings,
        )?);
    }

    for (path, materialization) in &declaration.dirs {
        if declared.contains_key(path) {
            continue;
        }
        if materialization.shared.is_some()
            || materialization.role.is_some()
            || materialization.idmap
        {
            bail!(
                "services.{service_name}.dirs.{}: undeclared extra binds may only use host and optional write: true",
                path.display()
            );
        }
        let host = materialization
            .host
            .as_ref()
            .expect("model requires a backing");
        if declaration.identity.is_none() {
            bail!(
                "services.{service_name}.dirs.{}.host: host backing requires a declared static identity (D48d)",
                path.display()
            );
        }
        warnings.push(format!(
            "services.{service_name}.dirs.{}: undeclared operator host bind is {} (CIP-82)",
            path.display(),
            if materialization.write {
                "read-write"
            } else {
                "read-only"
            }
        ));
        claims.push(DirectoryClaim {
            path: path.clone(),
            declared_role: None,
            role: None,
            writable: materialization.write,
            backing: DirectoryBacking::Host {
                path: host.clone(),
                idmap: false,
            },
        });
    }
    Ok(claims)
}

fn directory_claim(
    service_name: &str,
    path: &Path,
    declared_role: Option<DirectoryRole>,
    writable: bool,
    materialization: Option<&DirectoryMaterialization>,
    identity: Option<&str>,
    warnings: &mut Vec<String>,
) -> Result<DirectoryClaim> {
    let role = materialization
        .and_then(|materialization| materialization.role)
        .or(declared_role);
    if let (Some(from), Some(to)) = (declared_role, role) {
        if from != to && role_rank(to) < role_rank(from) {
            warnings.push(format!(
                "services.{service_name}.dirs.{}: LOUD durability degradation: {} is treated as {} (CIP-82/D49a)",
                path.display(),
                role_name(from),
                role_name(to)
            ));
        }
    }
    let backing = match materialization {
        None => DirectoryBacking::Private,
        Some(materialization) if materialization.host.is_some() => {
            if identity.is_none() {
                bail!(
                    "services.{service_name}.dirs.{}.host: host backing requires a declared static identity (D48d)",
                    path.display()
                );
            }
            DirectoryBacking::Host {
                path: materialization.host.clone().expect("checked above"),
                idmap: materialization.idmap,
            }
        }
        Some(materialization) if materialization.shared.is_some() => {
            if !writable {
                bail!(
                    "services.{service_name}.dirs.{}: a read-only DIR cannot be a shared writable surface",
                    path.display()
                );
            }
            if !matches!(role, Some(DirectoryRole::State) | None) {
                bail!(
                    "services.{service_name}.dirs.{}.shared: shared is only valid on STATEDIR or DIR in v0",
                    path.display()
                );
            }
            DirectoryBacking::Shared {
                name: materialization.shared.clone().expect("checked above"),
            }
        }
        Some(_) => DirectoryBacking::Private,
    };
    Ok(DirectoryClaim {
        path: path.to_owned(),
        declared_role,
        role,
        writable,
        backing,
    })
}

pub(crate) fn validate_shared_directories(
    services: &BTreeMap<String, CheckedService>,
) -> Result<()> {
    type SharedMembers<'a> = (Option<DirectoryRole>, Vec<(&'a str, &'a Path)>);
    let mut shared: BTreeMap<&str, SharedMembers<'_>> = BTreeMap::new();
    for (service, checked) in services {
        for claim in &checked.directories {
            let DirectoryBacking::Shared { name } = &claim.backing else {
                continue;
            };
            let entry = shared
                .entry(name)
                .or_insert_with(|| (claim.role, Vec::new()));
            if entry.0 != claim.role {
                bail!(
                    "shared {name:?}: members disagree on role ({} at services.{service}.dirs.{})",
                    entry.0.map(role_name).unwrap_or("DIR"),
                    claim.path.display()
                );
            }
            entry.1.push((service, &claim.path));
        }
    }
    Ok(())
}

fn role_rank(role: DirectoryRole) -> u8 {
    match role {
        DirectoryRole::State => 4,
        DirectoryRole::Logs => 3,
        DirectoryRole::Config => 3,
        DirectoryRole::Cache => 2,
        DirectoryRole::Run => 1,
    }
}

pub(crate) fn role_name(role: DirectoryRole) -> &'static str {
    match role {
        DirectoryRole::State => "STATEDIR",
        DirectoryRole::Cache => "CACHEDIR",
        DirectoryRole::Logs => "LOGDIR",
        DirectoryRole::Config => "CONFIGDIR",
        DirectoryRole::Run => "RUNDIR",
    }
}

pub(crate) struct SharedDirectory {
    pub host_path: PathBuf,
    pub members: Vec<String>,
}

pub(crate) fn collect_shared_directories(
    checked: &CheckResult,
) -> BTreeMap<String, SharedDirectory> {
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
                .push(service_unit_name(&checked.compose.name, service));
        }
    }
    for entry in shared.values_mut() {
        entry.members.sort();
        entry.members.dedup();
    }
    shared
}

pub(crate) fn private_service(service: &Service, claims: &[DirectoryClaim]) -> Service {
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

pub(crate) fn bind_value(source: &Path, destination: &Path, idmap: bool) -> String {
    let mut value = format!("{}:{}", source.display(), destination.display());
    if idmap {
        value.push_str(":idmap");
    }
    value
}

pub(crate) fn shared_group(composite: &str, name: &str) -> String {
    format!("cix-s-{:016x}", stable_hash(composite, name))
}

pub(crate) fn shared_host_path(composite: &str, name: &str) -> PathBuf {
    Path::new("/var/lib/cix-compose")
        .join(composite)
        .join("shared")
        .join(name)
}

pub(crate) fn stable_hash(composite: &str, name: &str) -> u64 {
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

pub(crate) fn render_shared_directory_unit(
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
        "[Unit]\\nDescription=cix compose shared directory: {name}\\nPartOf={target}\\nBefore={}\\n\\n[Service]\\nType=oneshot\\nSlice={}.slice\\nExecStart=/bin/sh -c true\\nRemainAfterExit=yes\\nGroup={group}\\nStateDirectory={relative}\\nStateDirectoryMode=2770\\nUMask=0002\\n",
        members.join(" "),
        target.trim_end_matches(".target")
    )
}

pub(crate) fn manifest_directory(
    composite: &str,
    service: &str,
    claim: &DirectoryClaim,
) -> ManifestDirectory {
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
        .join(format!("cix-{composite}-{}", unit_path(service)))
        .join(path.strip_prefix("/").expect("validated absolute path"))
}

fn service_unit_name(composite: &str, path: &str) -> String {
    format!("cix-{composite}-{}.service", unit_path(path))
}
