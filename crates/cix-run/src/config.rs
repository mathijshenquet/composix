use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};

use crate::spec::{parse_port, Service};

#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub env: BTreeMap<String, String>,
    pub ports: BTreeMap<String, u16>,
}

impl ResolvedConfig {
    pub fn resolve(
        service: &Service,
        env_overrides: &[String],
        port_overrides: &[String],
    ) -> Result<Self> {
        let env_overrides = parse_assignments("-e/--env", env_overrides)?;
        let port_overrides = parse_assignments("-p/--port", port_overrides)?;

        for name in env_overrides.keys() {
            if !service.env.contains_key(name) {
                bail!("environment override refers to undeclared variable {name:?}");
            }
        }
        for name in port_overrides.keys() {
            let Some(port) = service.ports.get(name) else {
                bail!("port override refers to undeclared port {name:?}");
            };
            if port.value.is_some() {
                bail!("port {name:?} is fixed at build time");
            }
        }

        let mut env = BTreeMap::new();
        for (name, declaration) in &service.env {
            let value =
                if let Some(value) = env_overrides.get(name) {
                    Some(declaration.parse_cli(value).with_context(|| {
                        format!("invalid value for environment variable {name:?}")
                    })?)
                } else {
                    declaration.default_string()?
                };
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

        Ok(Self { env, ports })
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
                "cixSpec": 1,
                "services": {
                    "app": {
                        "exec": ["bin/app"],
                        "env": {
                            "NAME": {"type": "string", "default": "default"},
                            "COUNT": {"type": "int", "required": true},
                            "PORT": {"type": "port", "default": 8000},
                            "ENABLED": {"type": "bool"},
                            "ROOT": {"type": "path", "default": "/srv/app"}
                        },
                        "ports": {"http": {"env": "PORT", "protocol": "tcp"}}
                    }
                }
            }"#,
        )
        .unwrap();
        spec.services["app"].clone()
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
        assert_eq!(resolved.env["COUNT"], "3");
        assert_eq!(resolved.env["PORT"], "10000");
        assert_eq!(resolved.env["ENABLED"], "false");
    }

    #[test]
    fn rejects_missing_and_invalid_values() {
        assert!(ResolvedConfig::resolve(&service(), &[], &[]).is_err());
        assert!(ResolvedConfig::resolve(&service(), &["COUNT=abc".into()], &[]).is_err());
        assert!(
            ResolvedConfig::resolve(&service(), &["COUNT=1".into(), "NOPE=x".into()], &[]).is_err()
        );
        assert!(
            ResolvedConfig::resolve(&service(), &["COUNT=1".into()], &["http=0".into()]).is_err()
        );
    }

    #[test]
    fn resolves_fixed_ports_and_rejects_overrides() {
        let spec = Spec::from_slice(
            br#"{
                "cixSpec": 2,
                "services": {
                    "app": {
                        "exec": ["bin/app"],
                        "ports": {"http": {"value": 8080, "protocol": "tcp"}}
                    }
                }
            }"#,
        )
        .unwrap();
        let service = &spec.services["app"];
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
}
