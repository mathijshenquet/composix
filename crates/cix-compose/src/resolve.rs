use std::{
    collections::{BTreeMap, BTreeSet},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use cix_index::Output;
use cix_run::{config::ResolvedConfig, spec::Service};

use crate::model::{Compose, Lock, LockedService, UpdatePolicy};

#[derive(Clone, Debug, Default)]
pub enum UpdateRequest {
    #[default]
    None,
    All,
    Service(String),
    Services(BTreeSet<String>),
}

#[derive(Clone, Debug)]
pub struct CheckedService {
    pub store_path: PathBuf,
    pub item_service: String,
    pub spec: Service,
    pub config: ResolvedConfig,
}

#[derive(Clone, Debug)]
pub struct CheckResult {
    pub compose: Compose,
    pub lock: Lock,
    pub services: BTreeMap<String, CheckedService>,
}

pub fn load_and_check(compose_path: &Path, update: UpdateRequest) -> Result<CheckResult> {
    let compose = Compose::load(compose_path)?;
    let existing = Lock::load_optional(&Compose::lock_path(compose_path))?.unwrap_or_default();
    check_with(&compose, &existing, &update, &cix_index::resolve)
}

pub(crate) fn check_with(
    compose: &Compose,
    existing: &Lock,
    update: &UpdateRequest,
    resolver: &dyn Fn(&str) -> Result<Output>,
) -> Result<CheckResult> {
    match update {
        UpdateRequest::Service(name) => validate_updated_service(compose, name)?,
        UpdateRequest::Services(names) => {
            for name in names {
                validate_updated_service(compose, name)?;
            }
        }
        UpdateRequest::None | UpdateRequest::All => {}
    }
    validate_edge_references(compose)?;
    let lock = resolve_lock(compose, existing, update, resolver)?;
    let mut services = BTreeMap::new();
    for (name, declaration) in &compose.services {
        let locked = &lock.services[name];
        let store_path = PathBuf::from(&locked.store_path);
        let spec = cix_run::spec::Spec::load(&store_path)
            .with_context(|| format!("services.{name}.item: invalid item {}", declaration.item))?;
        let (item_service, service) = spec.select_service(None).with_context(|| {
            format!("services.{name}.item: D41 requires the resolved item to contain one service")
        })?;
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
            .with_context(|| format!("services.{name}"))?;
        services.insert(
            name.clone(),
            CheckedService {
                store_path,
                item_service: item_service.to_owned(),
                spec: service.clone(),
                config,
            },
        );
    }
    validate_edges(compose, &services)?;
    validate_collisions(&services)?;
    Ok(CheckResult {
        compose: compose.clone(),
        lock,
        services,
    })
}

fn resolve_lock(
    compose: &Compose,
    existing: &Lock,
    update: &UpdateRequest,
    resolver: &dyn Fn(&str) -> Result<Output>,
) -> Result<Lock> {
    let mut services = BTreeMap::new();
    for (name, declaration) in &compose.services {
        let explicitly_updated = matches!(update, UpdateRequest::All)
            || matches!(update, UpdateRequest::Service(selected) if selected == name)
            || matches!(update, UpdateRequest::Services(selected) if selected.contains(name));
        let reusable = declaration.update == UpdatePolicy::Pin
            && !explicitly_updated
            && existing
                .services
                .get(name)
                .is_some_and(|locked| locked.reference == declaration.item);
        let locked = if reusable {
            existing.services[name].clone()
        } else {
            let output = resolver(&declaration.item)
                .with_context(|| format!("services.{name}.item: resolving {}", declaration.item))?;
            LockedService {
                reference: declaration.item.clone(),
                store_path: output.store_path,
                nar_hash: output.nar_hash,
            }
        };
        services.insert(name.clone(), locked);
    }
    Ok(Lock { services })
}

fn validate_updated_service(compose: &Compose, name: &str) -> Result<()> {
    if !compose.services.contains_key(name) {
        bail!("--update: service {name:?} is not declared");
    }
    Ok(())
}

fn validate_edge_references(compose: &Compose) -> Result<()> {
    for (edge_name, edge) in &compose.edges {
        if !compose.services.contains_key(&edge.producer.service) {
            bail!(
                "edges.{edge_name}.producer.service: unknown service {:?}",
                edge.producer.service
            );
        }
        for consumer in edge.consumers.keys() {
            if !compose.services.contains_key(consumer) {
                bail!("edges.{edge_name}.consumers.{consumer}: unknown service");
            }
        }
    }
    Ok(())
}

fn validate_edges(compose: &Compose, services: &BTreeMap<String, CheckedService>) -> Result<()> {
    let mut destinations: BTreeMap<(&str, &Path), &str> = BTreeMap::new();
    for (edge_name, edge) in &compose.edges {
        let producer = &services[&edge.producer.service];
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
                "edges.{edge_name}.producer.path: {} is not declared in services.{}.dirs.run",
                edge.producer.path.display(),
                edge.producer.service
            );
        }
        record_destination(
            &mut destinations,
            &edge.producer.service,
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
            r#"{{"composeVersion":1,"name":"stack","services":{services},"edges":{edges}}}"#
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
            &item_spec(r#""exec":["/nix/store/fake/bin/app"]"#),
        );
        let track = write_item(
            directory.path(),
            "track",
            &item_spec(r#""exec":["/nix/store/fake/bin/app"]"#),
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
            services: BTreeMap::from([
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
        assert_eq!(checked.lock.services["pin"].nar_hash, "pin-old");
        assert_eq!(checked.lock.services["track"].nar_hash, "track-new");
        assert_eq!(&*calls.borrow(), &["track:v1"]);

        calls.borrow_mut().clear();
        let checked = check_with(
            &compose,
            &old,
            &UpdateRequest::Service("pin".into()),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.services["pin"].nar_hash, "pin-new");
        assert_eq!(
            &*calls.borrow(),
            &["pin:v1".to_owned(), "track:v1".to_owned()]
        );

        calls.borrow_mut().clear();
        let checked = check_with(
            &compose,
            &old,
            &UpdateRequest::Services(BTreeSet::from(["pin".into()])),
            &resolver,
        )
        .unwrap();
        assert_eq!(checked.lock.services["pin"].nar_hash, "pin-new");
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
                r#""exec":["/nix/store/fake/bin/app"],"env":{"NEEDED":{"required":true}},"listeners":{"http":{"type":"stream"}},"dirs":{"run":["/run/app"]}"#,
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
            r#"{"db":{"producer":{"service":"missing","path":"/run/app"},"consumers":{"a":{}}}}"#,
        );
        assert!(
            check_with(&bad_edge, &Lock::default(), &UpdateRequest::None, &resolver)
                .unwrap_err()
                .to_string()
                .contains("unknown service")
        );

        let bad_path = compose(
            r#"{
                "a":{"item":"valid:v1","env":{"NEEDED":"x"},"bind":{"http":"127.0.0.1:8080"}},
                "b":{"item":"valid:v1","env":{"NEEDED":"x"},"bind":{"http":"127.0.0.1:8081"}}
            }"#,
            r#"{"db":{"producer":{"service":"a","path":"/run/wrong"},"consumers":{"b":{}}}}"#,
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
    fn detects_port_and_bind_collisions_independent_of_service_order() {
        let directory = tempfile::tempdir().unwrap();
        let port = write_item(
            directory.path(),
            "port",
            &item_spec(
                r#""exec":["/nix/store/fake/bin/app"],"ports":{"http":{"value":8080,"protocol":"tcp"}}"#,
            ),
        );
        let listener = write_item(
            directory.path(),
            "listener",
            &item_spec(
                r#""exec":["/nix/store/fake/bin/app"],"listeners":{"http":{"type":"stream"}}"#,
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
}
