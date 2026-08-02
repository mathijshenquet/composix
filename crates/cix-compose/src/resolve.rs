use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use cix_index::Output;
use cix_run::{
    config::ResolvedConfig,
    spec::{ManifestKind, Service},
};

use crate::model::{
    Child, Compose, ComposeService, DirectoryMaterialization, DirectoryRole, Edge, Lock,
    LockedService, SecretSource, UpdatePolicy,
};

#[derive(Clone, Debug, Default)]
pub enum UpdateRequest {
    #[default]
    None,
    All,
    Path(String),
    Paths(BTreeSet<String>),
}

#[derive(Clone, Debug)]
pub struct CheckedService {
    pub store_path: PathBuf,
    pub item_service: String,
    pub spec: Service,
    pub config: ResolvedConfig,
    pub directories: Vec<DirectoryClaim>,
    pub secrets: BTreeMap<String, SecretSource>,
    pub declaration: ComposeService,
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

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub compose: Compose,
    pub lock: Lock,
    pub services: BTreeMap<String, CheckedService>,
    pub edges: BTreeMap<String, Edge>,
    /// Relative group paths. The empty path is the deployment root.
    pub groups: BTreeSet<String>,
    pub warnings: Vec<String>,
}

pub fn load_and_check(compose_path: &Path, update: UpdateRequest) -> Result<CheckResult> {
    let compose = Compose::load(compose_path)?;
    let existing = Lock::load_optional(&Compose::lock_path(compose_path))?.unwrap_or_default();
    check_with_calendar(
        &compose,
        &existing,
        &update,
        &cix_index::resolve,
        &validate_calendar,
    )
}

#[cfg(test)]
pub(crate) fn check_with(
    compose: &Compose,
    existing: &Lock,
    update: &UpdateRequest,
    resolver: &dyn Fn(&str) -> Result<Output>,
) -> Result<CheckResult> {
    check_with_calendar(compose, existing, update, resolver, &|_| Ok(()))
}

fn check_with_calendar(
    compose: &Compose,
    existing: &Lock,
    update: &UpdateRequest,
    resolver: &dyn Fn(&str) -> Result<Output>,
    calendar_validator: &dyn Fn(&str) -> Result<()>,
) -> Result<CheckResult> {
    let mut builder = TreeBuilder {
        existing,
        update,
        resolver,
        calendar_validator,
        lock: Lock::default(),
        services: BTreeMap::new(),
        edges: BTreeMap::new(),
        groups: BTreeSet::new(),
        known_paths: BTreeSet::new(),
        warnings: Vec::new(),
    };
    builder.walk_group("", &compose.children, &compose.edges, &compose.secrets, 0)?;
    builder.validate_update_paths()?;
    let TreeBuilder {
        lock,
        services,
        edges,
        groups,
        mut warnings,
        ..
    } = builder;
    let declared_secrets = services
        .values()
        .flat_map(|service| service.spec.secrets.keys().cloned())
        .collect::<BTreeSet<_>>();
    for name in compose
        .secrets
        .keys()
        .filter(|name| !declared_secrets.contains(*name))
    {
        warnings.push(format!(
            "secrets.{name}: supplied but no resolved service declares it; CIP-81 treats this as a LOUD loosening"
        ));
    }
    validate_edges(&edges, &services)?;
    validate_collisions(&services)?;
    validate_shared_directories(&services)?;
    Ok(CheckResult {
        compose: compose.clone(),
        lock,
        services,
        edges,
        groups,
        warnings,
    })
}

struct TreeBuilder<'a> {
    existing: &'a Lock,
    update: &'a UpdateRequest,
    resolver: &'a dyn Fn(&str) -> Result<Output>,
    calendar_validator: &'a dyn Fn(&str) -> Result<()>,
    lock: Lock,
    services: BTreeMap<String, CheckedService>,
    edges: BTreeMap<String, Edge>,
    groups: BTreeSet<String>,
    known_paths: BTreeSet<String>,
    warnings: Vec<String>,
}

impl TreeBuilder<'_> {
    fn walk_group(
        &mut self,
        prefix: &str,
        children: &BTreeMap<String, Child>,
        edges: &BTreeMap<String, Edge>,
        secrets: &BTreeMap<String, SecretSource>,
        depth: usize,
    ) -> Result<()> {
        if depth > 64 {
            bail!("children: compose nesting exceeds the maximum depth of 64");
        }
        self.groups.insert(prefix.to_owned());
        if !prefix.is_empty() {
            self.known_paths.insert(prefix.to_owned());
        }
        for (name, child) in children {
            let path = join_path(prefix, name);
            self.known_paths.insert(path.clone());
            match child {
                Child::Item(declaration) => self.resolve_item(&path, declaration, secrets)?,
                Child::Compose(reference) => {
                    let locked = self.resolve_reference(
                        &path,
                        &reference.compose,
                        reference.update,
                        "compose",
                    )?;
                    let artifact_path = Path::new(&locked.store_path).join("cix.json");
                    let nested = Compose::load_artifact(&artifact_path).with_context(|| {
                        format!(
                            "children.{}.compose: resolved artifact {} must contain cix.json",
                            path, reference.compose
                        )
                    })?;
                    self.walk_group(
                        &path,
                        &nested.children,
                        &nested.edges,
                        &nested.secrets,
                        depth + 1,
                    )?;
                }
                Child::Group(group) => self.walk_group(
                    &path,
                    &group.children,
                    &group.edges,
                    &group.secrets,
                    depth + 1,
                )?,
            }
        }
        self.flatten_edges(prefix, children, edges)
    }

    fn resolve_item(
        &mut self,
        path: &str,
        declaration: &ComposeService,
        secrets: &BTreeMap<String, SecretSource>,
    ) -> Result<()> {
        let locked = self.resolve_reference(path, &declaration.item, declaration.update, "item")?;
        let store_path = PathBuf::from(&locked.store_path);
        let spec = cix_run::spec::Spec::load(&store_path)
            .with_context(|| format!("children.{path}.item: invalid item {}", declaration.item))?;
        validate_schedule(path, declaration, spec.kind, self.calendar_validator)?;
        let (item_service, service) = spec.select_service(None).with_context(|| {
            format!("children.{path}.item: D41 requires the resolved item to contain one service")
        })?;
        reject_secret_env_delivery(path, declaration)?;
        let resolved_secrets = resolve_secrets(path, service, secrets)?;
        let env = declaration
            .env
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>();
        let bindings = declaration
            .bind
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>();
        let config = ResolvedConfig::resolve(service, &env, &bindings)
            .with_context(|| format!("children.{path}"))?;
        let mut directories =
            materialize_directories(path, declaration, service, &mut self.warnings)?;
        let group_path = path.rsplit_once('/').map(|(group, _)| group).unwrap_or("");
        for claim in &mut directories {
            if let DirectoryBacking::Shared { name } = &mut claim.backing {
                *name = join_path(group_path, name);
            }
        }
        self.services.insert(
            path.to_owned(),
            CheckedService {
                store_path,
                item_service: item_service.to_owned(),
                spec: service.clone(),
                config,
                directories,
                secrets: resolved_secrets,
                declaration: declaration.clone(),
            },
        );
        Ok(())
    }

    fn resolve_reference(
        &mut self,
        path: &str,
        reference: &str,
        policy: UpdatePolicy,
        field: &str,
    ) -> Result<LockedService> {
        let explicitly_updated = update_contains(self.update, path);
        let reusable = policy == UpdatePolicy::Pin
            && !explicitly_updated
            && self
                .existing
                .paths
                .get(path)
                .is_some_and(|locked| locked.reference == reference);
        let locked = if reusable {
            self.existing.paths[path].clone()
        } else {
            let output = (self.resolver)(reference)
                .with_context(|| format!("children.{path}.{field}: resolving {reference}"))?;
            LockedService {
                reference: reference.to_owned(),
                store_path: output.store_path,
                nar_hash: output.nar_hash,
            }
        };
        self.lock.paths.insert(path.to_owned(), locked.clone());
        Ok(locked)
    }

    fn flatten_edges(
        &mut self,
        prefix: &str,
        children: &BTreeMap<String, Child>,
        edges: &BTreeMap<String, Edge>,
    ) -> Result<()> {
        for (edge_name, edge) in edges {
            let producer =
                flatten_endpoint(prefix, children, &edge.producer.child).with_context(|| {
                    format!("edges.{}.producer.child", join_path(prefix, edge_name))
                })?;
            let mut consumers = BTreeMap::new();
            for (consumer, config) in &edge.consumers {
                let path = flatten_endpoint(prefix, children, consumer).with_context(|| {
                    format!(
                        "edges.{}.consumers.{consumer}",
                        join_path(prefix, edge_name)
                    )
                })?;
                consumers.insert(path, config.clone());
            }
            self.edges.insert(
                join_path(prefix, edge_name),
                Edge {
                    producer: crate::model::Producer {
                        child: producer,
                        path: edge.producer.path.clone(),
                    },
                    consumers,
                },
            );
        }
        Ok(())
    }

    fn validate_update_paths(&self) -> Result<()> {
        match self.update {
            UpdateRequest::Path(path) => {
                if !self.known_paths.contains(path) {
                    bail!("--update-lock: path {path:?} is not declared in the compose tree");
                }
            }
            UpdateRequest::Paths(paths) => {
                for path in paths {
                    if !self.known_paths.contains(path) {
                        bail!("--update-lock: path {path:?} is not declared in the compose tree");
                    }
                }
            }
            UpdateRequest::None | UpdateRequest::All => {}
        }
        Ok(())
    }
}

fn join_path(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_owned()
    } else {
        format!("{prefix}/{name}")
    }
}

fn update_contains(update: &UpdateRequest, path: &str) -> bool {
    let contains = |selected: &str| path == selected || path.starts_with(&format!("{selected}/"));
    match update {
        UpdateRequest::None => false,
        UpdateRequest::All => true,
        UpdateRequest::Path(selected) => contains(selected),
        UpdateRequest::Paths(selected) => selected.iter().any(|selected| contains(selected)),
    }
}

fn flatten_endpoint(
    prefix: &str,
    children: &BTreeMap<String, Child>,
    name: &str,
) -> Result<String> {
    match children.get(name) {
        Some(Child::Item(_)) => Ok(join_path(prefix, name)),
        Some(Child::Compose(_) | Child::Group(_)) => bail!(
            "child {name:?} is a group; edges crossing a group boundary require publish and are deferred to the netns/publish track"
        ),
        None => bail!("unknown child {name:?}"),
    }
}

fn resolve_secrets(
    service_name: &str,
    service: &Service,
    supplied: &BTreeMap<String, SecretSource>,
) -> Result<BTreeMap<String, SecretSource>> {
    let mut resolved = BTreeMap::new();
    for name in service.secrets.keys() {
        let source = supplied.get(name).with_context(|| {
            format!("services.{service_name}.secrets.{name}: declared by item but not supplied by compose")
        })?;
        resolved.insert(name.clone(), source.clone());
    }
    Ok(resolved)
}

fn reject_secret_env_delivery(
    service_name: &str,
    declaration: &crate::model::ComposeService,
) -> Result<()> {
    for name in declaration.env.keys() {
        let upper = name.to_ascii_uppercase();
        if [
            "SECRET",
            "TOKEN",
            "PASSWORD",
            "CREDENTIAL",
            "PRIVATE_KEY",
            "API_KEY",
        ]
        .iter()
        .any(|needle| upper.contains(needle))
        {
            bail!(
                "services.{service_name}.env.{name}: secret-shaped values must use CIP-81 credentials, not compose env or .env"
            );
        }
    }
    Ok(())
}

fn materialize_directories(
    service_name: &str,
    declaration: &crate::model::ComposeService,
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

fn validate_shared_directories(services: &BTreeMap<String, CheckedService>) -> Result<()> {
    type SharedMembers<'a> = (Option<DirectoryRole>, Vec<(&'a str, &'a Path)>);
    let mut shared: BTreeMap<&str, SharedMembers<'_>> = BTreeMap::new();
    for (service, checked) in services {
        for claim in &checked.directories {
            let DirectoryBacking::Shared { name } = &claim.backing else {
                continue;
            };
            if claim.declared_role.is_none() && claim.role.is_none() {
                // This is the valid undecorated DIR spelling; retain its distinct role.
            }
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

fn role_name(role: DirectoryRole) -> &'static str {
    match role {
        DirectoryRole::State => "STATEDIR",
        DirectoryRole::Cache => "CACHEDIR",
        DirectoryRole::Logs => "LOGDIR",
        DirectoryRole::Config => "CONFIGDIR",
        DirectoryRole::Run => "RUNDIR",
    }
}

fn validate_schedule(
    service_name: &str,
    declaration: &crate::model::ComposeService,
    kind: ManifestKind,
    calendar_validator: &dyn Fn(&str) -> Result<()>,
) -> Result<()> {
    let Some(schedule) = declaration.schedule.as_deref() else {
        return Ok(());
    };
    if kind != ManifestKind::App {
        bail!("services.{service_name}.schedule: schedule is only valid for manifest kind app");
    }
    calendar_validator(schedule).with_context(|| {
        format!("services.{service_name}.schedule: invalid OnCalendar expression {schedule:?}")
    })
}

fn validate_calendar(schedule: &str) -> Result<()> {
    let output = match std::process::Command::new("systemd-analyze")
        .args(["calendar", schedule])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!(
                "note: systemd-analyze is unavailable; skipping OnCalendar validation and leaving it to activation"
            );
            return Ok(());
        }
        Err(error) => return Err(error).context("invoking systemd-analyze calendar"),
    };
    if output.status.success() {
        return Ok(());
    }
    let message = String::from_utf8_lossy(&output.stderr);
    bail!("{}", message.trim());
}

fn validate_edges(
    edges: &BTreeMap<String, Edge>,
    services: &BTreeMap<String, CheckedService>,
) -> Result<()> {
    let mut destinations: BTreeMap<(&str, &Path), &str> = BTreeMap::new();
    for (edge_name, edge) in edges {
        let producer = &services[&edge.producer.child];
        let declared = producer
            .spec
            .dirs
            .run
            .as_deref()
            .unwrap_or_default()
            .iter()
            .any(|path| path == &edge.producer.path);
        if !declared {
            bail!(
                "edges.{edge_name}.producer.path: {} is not declared in children.{}.dirs.run",
                edge.producer.path.display(),
                edge.producer.child
            );
        }
        record_destination(
            &mut destinations,
            &edge.producer.child,
            &edge.producer.path,
            edge_name,
        )?;
        for (consumer, config) in &edge.consumers {
            let path = config.path.as_deref().unwrap_or(&edge.producer.path);
            record_destination(&mut destinations, consumer, path, edge_name)?;
        }
    }
    Ok(())
}

fn record_destination<'a>(
    destinations: &mut BTreeMap<(&'a str, &'a Path), &'a str>,
    service: &'a str,
    path: &'a Path,
    edge: &'a str,
) -> Result<()> {
    if let Some(previous) = destinations.insert((service, path), edge) {
        bail!(
            "edges.{edge}: service {service:?} path {} is already used by edge {previous:?}",
            path.display()
        );
    }
    Ok(())
}

fn validate_collisions(services: &BTreeMap<String, CheckedService>) -> Result<()> {
    let mut ports: BTreeMap<u16, (&str, &str)> = BTreeMap::new();
    for (service_name, checked) in services {
        for (port_name, port) in &checked.config.ports {
            if let Some((other_service, other_port)) =
                ports.insert(*port, (service_name, port_name))
            {
                if other_service != service_name {
                    bail!(
                        "services.{service_name}: host port {port} for {port_name:?} collides with services.{other_service} port {other_port:?}"
                    );
                }
            }
        }
    }

    let mut bindings: Vec<(&str, &str, SocketAddr)> = Vec::new();
    for (service_name, checked) in services {
        for (listener, address) in &checked.config.listeners {
            for (other_service, other_listener, other_address) in &bindings {
                if service_name != other_service && addresses_collide(*address, *other_address) {
                    bail!(
                        "services.{service_name}.bind.{listener}: {address} collides with services.{other_service}.bind.{other_listener} ({other_address})"
                    );
                }
            }
            if let Some((other_service, other_port)) = ports.get(&address.port()) {
                if *other_service != service_name {
                    bail!(
                        "services.{service_name}.bind.{listener}: {address} collides with services.{other_service} port {other_port:?}"
                    );
                }
            }
            bindings.push((service_name, listener, *address));
        }
    }
    Ok(())
}

fn addresses_collide(left: SocketAddr, right: SocketAddr) -> bool {
    if left.port() != right.port() {
        return false;
    }
    match (left.ip(), right.ip()) {
        (IpAddr::V4(left), IpAddr::V4(right)) => {
            left == right || left.is_unspecified() || right.is_unspecified()
        }
        (IpAddr::V6(left), IpAddr::V6(right)) => {
            left == right || left.is_unspecified() || right.is_unspecified()
        }
        (left, right) => left.is_unspecified() || right.is_unspecified(),
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, fs};

    use super::*;

    fn output(path: &Path, hash: &str) -> Output {
        Output {
            store_path: path.display().to_string(),
            nar_hash: hash.into(),
            drv_path: None,
        }
    }

    fn write_item(root: &Path, name: &str, spec: &str) -> PathBuf {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(path.join("cix-manifest.json"), spec).unwrap();
        path
    }

    fn compose(services: &str, edges: &str) -> Compose {
        serde_json::from_str(&format!(
            r#"{{"cixCompose":1,"name":"stack","children":{services},"edges":{edges}}}"#
        ))
        .unwrap()
    }

    fn item_spec(service: &str) -> String {
        format!(r#"{{"cixManifest":0,{service}}}"#)
    }

    #[test]
    fn lock_lifecycle_respects_pin_track_and_update() {
        let directory = tempfile::tempdir().unwrap();
        let pin = write_item(
            directory.path(),
            "pin",
            &item_spec(r#""start":["/nix/store/fake/bin/app"]"#),
        );
        let track = write_item(
            directory.path(),
            "track",
            &item_spec(r#""start":["/nix/store/fake/bin/app"]"#),
        );
        let compose = compose(
            r#"{
                "pin":{"item":"pin:v1"},
                "track":{"item":"track:v1","update":"track"}
            }"#,
            "{}",
        );
        let calls = RefCell::new(Vec::new());
        let resolver = |reference: &str| {
            calls.borrow_mut().push(reference.to_owned());
            Ok(if reference.starts_with("pin") {
                output(&pin, "pin-new")
            } else {
                output(&track, "track-new")
            })
        };
        let old = Lock {
            paths: BTreeMap::from([
                (
                    "pin".into(),
                    LockedService {
                        reference: "pin:v1".into(),
                        store_path: pin.display().to_string(),
                        nar_hash: "pin-old".into(),
                    },
                ),
                (
                    "track".into(),
                    LockedService {
                        reference: "track:v1".into(),
                        store_path: track.display().to_string(),
                        nar_hash: "track-old".into(),
                    },
                ),
            ]),
        };

        let checked = check_with(&compose, &old, &UpdateRequest::None, &resolver).unwrap();
        assert_eq!(checked.lock.paths["pin"].nar_hash, "pin-old");
        assert_eq!(checked.lock.paths["track"].nar_hash, "track-new");
        assert_eq!(&*calls.borrow(), &["track:v1"]);

        calls.borrow_mut().clear();
        let checked = check_with(
            &compose,
            &old,
            &UpdateRequest::Path("pin".into()),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.paths["pin"].nar_hash, "pin-new");
        assert_eq!(
            &*calls.borrow(),
            &["pin:v1".to_owned(), "track:v1".to_owned()]
        );

        calls.borrow_mut().clear();
        let checked = check_with(
            &compose,
            &old,
            &UpdateRequest::Paths(BTreeSet::from(["pin".into()])),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.paths["pin"].nar_hash, "pin-new");
        assert_eq!(
            &*calls.borrow(),
            &["pin:v1".to_owned(), "track:v1".to_owned()]
        );
    }

    #[test]
    fn semantic_checks_cover_edges_env_required_bind_and_invalid_specs() {
        let directory = tempfile::tempdir().unwrap();
        let valid = write_item(
            directory.path(),
            "valid",
            &item_spec(
                r#""start":["/nix/store/fake/bin/app"],"env":{"NEEDED":{"required":true}},"listeners":{"http":{"type":"stream"}},"dirs":{"run":["/run/app"]}"#,
            ),
        );
        let invalid = write_item(directory.path(), "invalid", r#"{"cixManifest":99}"#);
        let resolver = |reference: &str| {
            Ok(if reference == "invalid:v1" {
                output(&invalid, "invalid")
            } else {
                output(&valid, "valid")
            })
        };

        let missing_required = compose(
            r#"{"a":{"item":"valid:v1","bind":{"http":"127.0.0.1:8080"}}}"#,
            "{}",
        );
        assert!(format!(
            "{:#}",
            check_with(
                &missing_required,
                &Lock::default(),
                &UpdateRequest::None,
                &resolver
            )
            .unwrap_err()
        )
        .contains("required environment variable"));

        let undeclared_env = compose(
            r#"{"a":{"item":"valid:v1","env":{"NEEDED":"x","NOPE":"x"},"bind":{"http":"127.0.0.1:8080"}}}"#,
            "{}",
        );
        assert!(format!(
            "{:#}",
            check_with(
                &undeclared_env,
                &Lock::default(),
                &UpdateRequest::None,
                &resolver
            )
            .unwrap_err()
        )
        .contains("undeclared variable"));

        let bad_edge = compose(
            r#"{"a":{"item":"valid:v1","env":{"NEEDED":"x"},"bind":{"http":"127.0.0.1:8080"}}}"#,
            r#"{"db":{"producer":{"child":"missing","path":"/run/app"},"consumers":{"a":{}}}}"#,
        );
        assert!(format!(
            "{:#}",
            check_with(&bad_edge, &Lock::default(), &UpdateRequest::None, &resolver).unwrap_err()
        )
        .contains("unknown child"));

        let bad_path = compose(
            r#"{
                "a":{"item":"valid:v1","env":{"NEEDED":"x"},"bind":{"http":"127.0.0.1:8080"}},
                "b":{"item":"valid:v1","env":{"NEEDED":"x"},"bind":{"http":"127.0.0.1:8081"}}
            }"#,
            r#"{"db":{"producer":{"child":"a","path":"/run/wrong"},"consumers":{"b":{}}}}"#,
        );
        assert!(
            check_with(&bad_path, &Lock::default(), &UpdateRequest::None, &resolver)
                .unwrap_err()
                .to_string()
                .contains("not declared")
        );

        let invalid_item = compose(r#"{"a":{"item":"invalid:v1"}}"#, "{}");
        assert!(format!(
            "{:#}",
            check_with(
                &invalid_item,
                &Lock::default(),
                &UpdateRequest::None,
                &resolver
            )
            .unwrap_err()
        )
        .contains("unsupported cixManifest"));
    }

    #[test]
    fn secrets_require_declaration_and_make_undeclared_supply_loud() {
        let directory = tempfile::tempdir().unwrap();
        let item = write_item(
            directory.path(),
            "item",
            &item_spec(
                r#""start":["/nix/store/fake/bin/app"],"secrets":{"db-password":{"as":"DB_PASSWORD_FILE"}}"#,
            ),
        );
        let resolver = |_reference: &str| Ok(output(&item, "item"));
        let missing = compose(r#"{"app":{"item":"app:v1"}}"#, "{}");
        let error = check_with(&missing, &Lock::default(), &UpdateRequest::None, &resolver)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("declared by item but not supplied"),
            "{error}"
        );

        let supplied: Compose = serde_json::from_str(r#"{
            "cixCompose":1,"name":"stack",
            "children":{"app":{"item":"app:v1"}},
            "secrets":{"db-password":{"file":"/run/keys/db"},"stray":{"encrypted":"/run/keys/stray"}}
        }"#).unwrap();
        let checked =
            check_with(&supplied, &Lock::default(), &UpdateRequest::None, &resolver).unwrap();
        assert_eq!(checked.services["app"].secrets.len(), 1);
        assert!(checked
            .warnings
            .iter()
            .any(|warning| warning.contains("stray") && warning.contains("LOUD")));
    }

    #[test]
    fn schedules_require_apps_and_a_valid_calendar() {
        let directory = tempfile::tempdir().unwrap();
        let service = write_item(
            directory.path(),
            "service",
            &item_spec(r#""start":["/nix/store/fake/bin/service"]"#),
        );
        let app = write_item(
            directory.path(),
            "app",
            &item_spec(r#""kind":"app","start":["/nix/store/fake/bin/app"]"#),
        );
        let resolver = |reference: &str| {
            Ok(if reference == "service:v1" {
                output(&service, "service")
            } else {
                output(&app, "app")
            })
        };

        let service_schedule = compose(
            r#"{"worker":{"item":"service:v1","schedule":"daily"}}"#,
            "{}",
        );
        let error = check_with_calendar(
            &service_schedule,
            &Lock::default(),
            &UpdateRequest::None,
            &resolver,
            &|_| Ok(()),
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("only valid for manifest kind app"),
            "{error}"
        );

        let app_schedule = compose(r#"{"worker":{"item":"app:v1","schedule":"daily"}}"#, "{}");
        let error = format!(
            "{:#}",
            check_with_calendar(
                &app_schedule,
                &Lock::default(),
                &UpdateRequest::None,
                &resolver,
                &|schedule| anyhow::bail!("{schedule} is not a calendar"),
            )
            .unwrap_err()
        );
        assert!(error.contains("services.worker.schedule"), "{error}");
        assert!(error.contains("not a calendar"), "{error}");
    }

    #[test]
    fn detects_port_and_bind_collisions_independent_of_service_order() {
        let directory = tempfile::tempdir().unwrap();
        let port = write_item(
            directory.path(),
            "port",
            &item_spec(
                r#""start":["/nix/store/fake/bin/app"],"ports":{"http":{"value":8080,"protocol":"tcp"}}"#,
            ),
        );
        let listener = write_item(
            directory.path(),
            "listener",
            &item_spec(
                r#""start":["/nix/store/fake/bin/app"],"listeners":{"http":{"type":"stream"}}"#,
            ),
        );
        let resolver = |reference: &str| {
            Ok(if reference.starts_with("port") {
                output(&port, reference)
            } else {
                output(&listener, reference)
            })
        };
        for services in [
            r#"{"a":{"item":"port:v1"},"z":{"item":"port:v2"}}"#,
            r#"{"a":{"item":"port:v2"},"z":{"item":"port:v1"}}"#,
        ] {
            let error = check_with(
                &compose(services, "{}"),
                &Lock::default(),
                &UpdateRequest::None,
                &resolver,
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("host port 8080"), "{error}");
        }
        for services in [
            r#"{"a":{"item":"listener:v1","bind":{"http":"0.0.0.0:9000"}},"z":{"item":"listener:v2","bind":{"http":"127.0.0.1:9000"}}}"#,
            r#"{"a":{"item":"listener:v2","bind":{"http":"127.0.0.1:9000"}},"z":{"item":"listener:v1","bind":{"http":"0.0.0.0:9000"}}}"#,
        ] {
            let error = check_with(
                &compose(services, "{}"),
                &Lock::default(),
                &UpdateRequest::None,
                &resolver,
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("collides"), "{error}");
        }
    }

    #[test]
    fn directory_checks_require_identity_and_make_degradation_loud() {
        let directory = tempfile::tempdir().unwrap();
        let item = write_item(
            directory.path(),
            "item",
            &item_spec(
                r#""start":["/nix/store/fake/bin/app"],"dirs":{"state":["/var/lib/app"],"cache":["/var/cache/app"],"data":[{"path":"/media","ro":false}]}"#,
            ),
        );
        let resolver = |_reference: &str| Ok(output(&item, "item"));
        let host_without_identity = compose(
            r#"{"web":{"item":"item:v1","dirs":{"/var/lib/app":{"host":"/srv/app"},"/media":{"host":"/srv/media"}}}}"#,
            "{}",
        );
        let error = check_with(
            &host_without_identity,
            &Lock::default(),
            &UpdateRequest::None,
            &resolver,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("static identity"), "{error}");

        let degraded = compose(
            r#"{"web":{"item":"item:v1","identity":"operator","dirs":{"/var/lib/app":{"as":"cache"},"/media":{"shared":"uploads"}}}}"#,
            "{}",
        );
        let checked =
            check_with(&degraded, &Lock::default(), &UpdateRequest::None, &resolver).unwrap();
        assert!(checked
            .warnings
            .iter()
            .any(|warning| warning.contains("LOUD durability degradation")));

        let bad_shared = compose(
            r#"{"web":{"item":"item:v1","dirs":{"/var/lib/app":{"shared":"uploads"},"/media":{"shared":"uploads"}}}}"#,
            "{}",
        );
        let error = check_with(
            &bad_shared,
            &Lock::default(),
            &UpdateRequest::None,
            &resolver,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("members disagree on role"), "{error}");
    }

    #[test]
    fn locks_every_ref_by_path_and_updates_only_the_selected_subtree() {
        let directory = tempfile::tempdir().unwrap();
        let item = write_item(
            directory.path(),
            "shared-item",
            &item_spec(r#""start":["/nix/store/fake/bin/app"]"#),
        );
        let artifact = directory.path().join("suite");
        fs::create_dir_all(&artifact).unwrap();
        fs::write(
            artifact.join("cix.json"),
            r#"{"cixCompose":1,"name":"advisory-name","children":{"leaf":{"item":"same:v1"}}}"#,
        )
        .unwrap();
        fs::write(artifact.join("cix.lock"), b"this advisory lock is ignored").unwrap();
        let root: Compose = serde_json::from_str(
            r#"{
              "cixCompose": 1,
              "name": "root",
              "children": {
                "inline": {"children": {
                  "a": {"item": "same:v1"},
                  "b": {"item": "same:v1"}
                }},
                "sealed": {"compose": "suite:v1"}
              }
            }"#,
        )
        .unwrap();
        let old = Lock {
            paths: BTreeMap::from([
                (
                    "inline/a".into(),
                    LockedService {
                        reference: "same:v1".into(),
                        store_path: item.display().to_string(),
                        nar_hash: "a-old".into(),
                    },
                ),
                (
                    "inline/b".into(),
                    LockedService {
                        reference: "same:v1".into(),
                        store_path: item.display().to_string(),
                        nar_hash: "b-old".into(),
                    },
                ),
                (
                    "sealed".into(),
                    LockedService {
                        reference: "suite:v1".into(),
                        store_path: artifact.display().to_string(),
                        nar_hash: "suite-old".into(),
                    },
                ),
                (
                    "sealed/leaf".into(),
                    LockedService {
                        reference: "same:v1".into(),
                        store_path: item.display().to_string(),
                        nar_hash: "leaf-old".into(),
                    },
                ),
            ]),
        };
        let calls = RefCell::new(Vec::new());
        let resolver = |reference: &str| {
            calls.borrow_mut().push(reference.to_owned());
            Ok(if reference == "suite:v1" {
                output(&artifact, "suite-new")
            } else {
                output(&item, "item-new")
            })
        };

        let checked = check_with(&root, &old, &UpdateRequest::None, &resolver).unwrap();
        assert!(calls.borrow().is_empty());
        assert_eq!(
            checked.services.keys().cloned().collect::<Vec<_>>(),
            ["inline/a", "inline/b", "sealed/leaf"]
        );
        assert_eq!(checked.lock.paths.len(), 4);

        let checked = check_with(
            &root,
            &old,
            &UpdateRequest::Path("inline/a".into()),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.paths["inline/a"].nar_hash, "item-new");
        assert_eq!(checked.lock.paths["inline/b"].nar_hash, "b-old");
        assert_eq!(&*calls.borrow(), &["same:v1"]);

        calls.borrow_mut().clear();
        let checked = check_with(
            &root,
            &old,
            &UpdateRequest::Path("sealed".into()),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.paths["sealed"].nar_hash, "suite-new");
        assert_eq!(checked.lock.paths["sealed/leaf"].nar_hash, "item-new");
        assert_eq!(&*calls.borrow(), &["suite:v1", "same:v1"]);
    }
}
