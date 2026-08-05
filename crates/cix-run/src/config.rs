use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Component, Path};

use anyhow::{bail, Context, Result};

use crate::spec::{parse_port, Service};

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub env: BTreeMap<String, String>,
    pub ports: BTreeMap<String, u16>,
    /// Operator bindings for named inherited listeners.
    pub listeners: BTreeMap<String, SocketAddr>,
}

impl ResolvedConfig {
    pub fn resolve(
        service: &Service,
        env_overrides: &[String],
        port_overrides: &[String],
    ) -> Result<Self> {
        Self::resolve_inner(service, env_overrides, port_overrides, true)
    }

    /// Resolve compose input while allowing a listener to receive its host binding from a
    /// containing group's published surface.
    pub fn resolve_compose(
        service: &Service,
        env_overrides: &[String],
        port_overrides: &[String],
    ) -> Result<Self> {
        Self::resolve_inner(service, env_overrides, port_overrides, false)
    }

    pub(crate) fn resolve_debug(service: &Service, env_overrides: &[String]) -> Result<Self> {
        Self::resolve_inner(service, env_overrides, &[], false)
    }

    pub(crate) fn item_environment(&self, output: &Path) -> Result<BTreeMap<String, String>> {
        let mut env = self.env.clone();
        let Some(path) = env.get("PATH") else {
            return Ok(env);
        };
        let paths = std::env::split_paths(path)
            .map(|directory| {
                if directory.is_absolute() {
                    return Ok(directory);
                }
                if directory
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
                {
                    bail!("relative service PATH entry must be a normalized item path");
                }
                Ok(output.join(directory))
            })
            .collect::<Result<Vec<_>>>()?;
        env.insert(
            "PATH".into(),
            std::env::join_paths(paths)
                .context("service PATH contains an invalid path")?
                .into_string()
                .map_err(|_| anyhow::anyhow!("service PATH is not valid UTF-8"))?,
        );
        Ok(env)
    }

    fn resolve_inner(
        service: &Service,
        env_overrides: &[String],
        port_overrides: &[String],
        require_listeners: bool,
    ) -> Result<Self> {
        let env_overrides = parse_assignments("-e/--env", env_overrides)?;
        let overrides = parse_assignments("-p/--port", port_overrides)?;
        let mut port_overrides = BTreeMap::new();
        let mut listeners = BTreeMap::new();

        for name in env_overrides.keys() {
            if !service.env.contains_key(name) {
                bail!("environment override refers to undeclared variable {name:?}");
            }
        }
        for env_name in service.ports.values().filter_map(|port| port.env.as_ref()) {
            if let Some(value) = env_overrides.get(env_name) {
                parse_port(value).with_context(|| {
                    format!(
                        "invalid override for ports-referenced environment variable {env_name:?}"
                    )
                })?;
            }
        }
        for (name, value) in overrides {
            if let Some(port) = service.ports.get(&name) {
                if port.value.is_some() {
                    bail!("port {name:?} is fixed at build time");
                }
                port_overrides.insert(name, value);
            } else if service.listeners.contains_key(&name) {
                let address = value.parse::<SocketAddr>().with_context(|| {
                    format!(
                        "listener binding for {name:?} must be an IP address and port such as 127.0.0.1:8080"
                    )
                })?;
                listeners.insert(name, address);
            } else {
                bail!("-p/--port target {name:?} is neither a declared port nor listener");
            }
        }
        if require_listeners {
            for name in service.listeners.keys() {
                if !listeners.contains_key(name) {
                    bail!(
                        "listener {name:?} has no binding; pass -p {name}=ADDR:PORT when running this service"
                    );
                }
            }
        }

        let mut env = BTreeMap::new();
        for (name, declaration) in &service.env {
            let value = env_overrides
                .get(name)
                .cloned()
                .or_else(|| declaration.default_string());
            if let Some(value) = value {
                env.insert(name.clone(), value);
            } else if declaration.required {
                bail!("required environment variable {name:?} has no value; pass -e {name}=VALUE");
            }
        }

        let mut port_env_values = BTreeMap::new();
        for (port_name, value) in port_overrides {
            let value = parse_port(&value)
                .with_context(|| format!("invalid override for port {port_name:?}"))?
                .to_string();
            let env_name = service.ports[&port_name]
                .env
                .as_ref()
                .expect("validated env-backed port");
            if let Some(previous) = port_env_values.insert(env_name.clone(), value.clone()) {
                if previous != value {
                    bail!(
                        "port override {port_name:?} conflicts with another value for environment variable {env_name:?}"
                    );
                }
            }
        }
        env.extend(port_env_values);

        let mut ports = BTreeMap::new();
        for (name, port) in &service.ports {
            let value = if let Some(value) = port.value {
                Some(value)
            } else {
                port.env
                    .as_ref()
                    .and_then(|env_name| env.get(env_name))
                    .map(|value| parse_port(value))
                    .transpose()?
            };
            if let Some(value) = value {
                ports.insert(name.clone(), value);
            }
        }

        Ok(Self {
            env,
            ports,
            listeners,
        })
    }
}

fn parse_assignments(flag: &str, values: &[String]) -> Result<BTreeMap<String, String>> {
    let mut parsed = BTreeMap::new();
    for assignment in values {
        let (name, value) = assignment.split_once('=').with_context(|| {
            format!("{flag} value {assignment:?} must have the form NAME=VALUE")
        })?;
        if name.is_empty() {
            bail!("{flag} value {assignment:?} has an empty name");
        }
        if parsed.insert(name.to_owned(), value.to_owned()).is_some() {
            bail!("{flag} specifies {name:?} more than once");
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use crate::spec::Spec;

    use super::*;

    fn service() -> Service {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                        "start": ["bin/app"],
                        "env": {
                            "NAME": {"default": "default"},
                            "COUNT": {"required": true},
                            "PORT": {"default": "8000"},
                            "ENABLED": {},
                            "ROOT": {"default": "/srv/app"}
                        },
                        "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
            }"#,
        )
        .unwrap();
        spec.select_service(None).unwrap().1.clone()
    }

    #[test]
    fn resolves_defaults_env_and_port_precedence() {
        let resolved = ResolvedConfig::resolve(
            &service(),
            &[
                "COUNT=03".into(),
                "PORT=9000".into(),
                "ENABLED=false".into(),
            ],
            &["http=10000".into()],
        )
        .unwrap();
        assert_eq!(resolved.env["NAME"], "default");
        assert_eq!(resolved.env["COUNT"], "03");
        assert_eq!(resolved.env["PORT"], "10000");
        assert_eq!(resolved.env["ENABLED"], "false");
    }

    #[test]
    fn rejects_missing_and_invalid_values() {
        assert!(ResolvedConfig::resolve(&service(), &[], &[]).is_err());
        assert!(ResolvedConfig::resolve(&service(), &["COUNT=abc".into()], &[]).is_ok());
        assert!(
            ResolvedConfig::resolve(&service(), &["COUNT=1".into(), "NOPE=x".into()], &[]).is_err()
        );
        assert!(
            ResolvedConfig::resolve(&service(), &["COUNT=1".into()], &["http=0".into()]).is_err()
        );
    }

    #[test]
    fn leaves_bare_optional_environment_unset_until_overridden() {
        let resolved = ResolvedConfig::resolve(&service(), &["COUNT=1".into()], &[]).unwrap();
        assert!(!resolved.env.contains_key("ENABLED"));

        let resolved =
            ResolvedConfig::resolve(&service(), &["COUNT=1".into(), "ENABLED=false".into()], &[])
                .unwrap();
        assert_eq!(resolved.env["ENABLED"], "false");
    }

    #[test]
    fn rejects_non_port_env_overrides_for_ports_referenced_variables() {
        let error =
            ResolvedConfig::resolve(&service(), &["COUNT=1".into(), "PORT=nope".into()], &[])
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid override for ports-referenced environment variable \"PORT\""),
            "{error:#}"
        );
    }

    #[test]
    fn resolves_fixed_ports_and_rejects_overrides() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                        "start": ["bin/app"],
                        "ports": {"http": {"value": 8080, "protocol": "tcp"}}
            }"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let resolved = ResolvedConfig::resolve(service, &[], &[]).unwrap();
        assert_eq!(resolved.ports["http"], 8080);

        let error = ResolvedConfig::resolve(service, &[], &["http=9090".into()]).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("port \"http\" is fixed at build time"),
            "{error:#}"
        );
    }

    #[test]
    fn resolves_listener_bindings_and_requires_all_listeners() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                        "start": ["bin/app"],
                        "listeners": {"http": {"type": "stream"}},
                        "ports": {"metrics": {"value": 9090, "protocol": "tcp"}}
            }"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let resolved =
            ResolvedConfig::resolve(service, &[], &["http=127.0.0.1:8080".into()]).unwrap();
        assert_eq!(resolved.listeners["http"].to_string(), "127.0.0.1:8080");
        assert_eq!(resolved.ports["metrics"], 9090);

        let missing = ResolvedConfig::resolve(service, &[], &[]).unwrap_err();
        assert!(missing
            .to_string()
            .contains("listener \"http\" has no binding"));
        let bad =
            ResolvedConfig::resolve(service, &[], &["other=127.0.0.1:8080".into()]).unwrap_err();
        assert!(bad
            .to_string()
            .contains("neither a declared port nor listener"));
    }

    #[test]
    fn resolves_relative_path_entries_against_the_item() {
        let config = ResolvedConfig {
            env: BTreeMap::from([("PATH".into(), "bin:/nix/store/tools/bin".into())]),
            ports: BTreeMap::new(),
            listeners: BTreeMap::new(),
        };
        let env = config
            .item_environment(Path::new("/nix/store/hash-cix-item-api"))
            .unwrap();
        assert_eq!(
            env["PATH"],
            "/nix/store/hash-cix-item-api/bin:/nix/store/tools/bin"
        );
    }
}
