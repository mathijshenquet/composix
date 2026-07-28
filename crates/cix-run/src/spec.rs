use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    #[serde(rename = "cixSpec")]
    pub cix_spec: u32,
    pub services: BTreeMap<String, Service>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub exec: Vec<String>,
    /// Pre-start argv run in the service sandbox on every start.
    ///
    /// It follows the same output-relative executable and environment interpolation rules as
    /// `exec` and must be idempotent.
    pub setup: Option<Vec<String>>,
    #[serde(default)]
    pub env: BTreeMap<String, Env>,
    #[serde(default)]
    pub ports: BTreeMap<String, Port>,
    #[serde(default)]
    pub dirs: Dirs,
    pub health: Option<Health>,
    pub network: Option<Network>,
    pub jit: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    #[serde(rename = "type")]
    pub kind: EnvType,
    pub default: Option<Value>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum EnvType {
    String,
    Int,
    Bool,
    Port,
    Path,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub env: Option<String>,
    pub value: Option<u16>,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dirs {
    #[serde(default)]
    pub state: Vec<PathBuf>,
    #[serde(default)]
    pub cache: Vec<PathBuf>,
    #[serde(default)]
    pub logs: Vec<PathBuf>,
    #[serde(default)]
    pub config: Vec<PathBuf>,
    pub run: Option<Vec<PathBuf>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub exec: Vec<String>,
    pub interval: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Host,
}

impl Spec {
    pub fn from_slice(json: &[u8]) -> Result<Self> {
        let spec: Self = serde_json::from_slice(json).context("failed to parse cix-spec.json")?;
        spec.validate()?;
        Ok(spec)
    }

    pub fn load(output: &Path) -> Result<Self> {
        let path = output.join("cix-spec.json");
        let json = fs::read(&path)
            .with_context(|| format!("failed to read spec at {}", path.display()))?;
        Self::from_slice(&json)
    }

    pub fn select_service<'a>(&'a self, requested: Option<&str>) -> Result<(&'a str, &'a Service)> {
        if let Some(name) = requested {
            let service = self.services.get(name).with_context(|| {
                let available = self.services.keys().cloned().collect::<Vec<_>>().join(", ");
                format!("service {name:?} is not declared; available services: {available}")
            })?;
            return Ok((self.services.get_key_value(name).unwrap().0, service));
        }

        if self.services.len() != 1 {
            let available = self.services.keys().cloned().collect::<Vec<_>>().join(", ");
            bail!(
                "the spec declares {} services; select one with #service (available: {available})",
                self.services.len()
            );
        }
        let (name, service) = self.services.first_key_value().unwrap();
        Ok((name, service))
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.cix_spec, 1 | 2) {
            bail!(
                "unsupported cixSpec version {}; this cix supports versions 1 and 2",
                self.cix_spec
            );
        }
        if self.services.is_empty() {
            bail!("cix-spec.json must declare at least one service");
        }

        for (name, service) in &self.services {
            validate_name("service", name)?;
            service
                .validate(self.cix_spec)
                .with_context(|| format!("invalid service {name:?}"))?;
        }
        Ok(())
    }
}

impl Service {
    fn validate(&self, version: u32) -> Result<()> {
        self.validate_version_fields(version)?;
        validate_exec("exec", &self.exec, &self.env)?;
        if let Some(setup) = &self.setup {
            validate_exec("setup", setup, &self.env)?;
        }
        if let Some(health) = &self.health {
            validate_exec("health.exec", &health.exec, &self.env)?;
            if health.interval.is_empty() {
                bail!("health.interval must not be empty");
            }
        }

        for (name, declaration) in &self.env {
            validate_env_name(name)?;
            if let Some(default) = &declaration.default {
                declaration.parse_json(default).with_context(|| {
                    format!("default for environment variable {name:?} has the wrong type")
                })?;
            }
        }

        for (name, port) in &self.ports {
            validate_name("port", name)?;
            match (&port.env, port.value) {
                (Some(env_name), None) => {
                    let env = self.env.get(env_name).with_context(|| {
                        format!(
                            "port {name:?} refers to undeclared environment variable {env_name:?}"
                        )
                    })?;
                    if env.kind != EnvType::Port {
                        bail!(
                            "port {name:?} refers to {env_name:?}, which must have type \"port\""
                        );
                    }
                }
                (None, Some(0)) => bail!("port {name:?} value must be between 1 and 65535"),
                (None, Some(_)) => {}
                (Some(_), Some(_)) => {
                    bail!("port {name:?} must declare exactly one of \"env\" or \"value\"")
                }
                (None, None) => {
                    bail!("port {name:?} must declare exactly one of \"env\" or \"value\"")
                }
            }
        }

        let mut seen: Vec<&Path> = Vec::new();
        for (role, root, paths) in self.dirs.roles() {
            for path in paths {
                validate_app_path(version, role, root, path)?;
                for other in &seen {
                    if path.starts_with(other) || other.starts_with(path) {
                        bail!(
                            "directory paths {} and {} overlap",
                            other.display(),
                            path.display()
                        );
                    }
                }
                seen.push(path);
            }
        }
        Ok(())
    }

    pub fn has_network(&self) -> bool {
        !self.ports.is_empty() || self.network == Some(Network::Host)
    }

    fn validate_version_fields(&self, version: u32) -> Result<()> {
        if version != 1 {
            return Ok(());
        }
        if self.setup.is_some() {
            bail!("field \"setup\" requires cixSpec 2");
        }
        if self.dirs.run.is_some() {
            bail!("field \"dirs.run\" requires cixSpec 2");
        }
        if self.jit.is_some() {
            bail!("field \"jit\" requires cixSpec 2");
        }
        for (name, port) in &self.ports {
            if port.value.is_some() {
                bail!("field \"ports.{name}.value\" requires cixSpec 2");
            }
        }
        Ok(())
    }
}

impl Env {
    pub fn parse_cli(&self, value: &str) -> Result<String> {
        match self.kind {
            EnvType::String => Ok(value.to_owned()),
            EnvType::Int => value
                .parse::<i64>()
                .map(|value| value.to_string())
                .context("expected an integer"),
            EnvType::Bool => match value {
                "true" | "false" => Ok(value.to_owned()),
                _ => bail!("expected true or false"),
            },
            EnvType::Port => parse_port(value).map(|value| value.to_string()),
            EnvType::Path => {
                let path = Path::new(value);
                validate_absolute_clean_path(path, "path environment value")?;
                Ok(value.to_owned())
            }
        }
    }

    fn parse_json(&self, value: &Value) -> Result<String> {
        match self.kind {
            EnvType::String => value
                .as_str()
                .map(ToOwned::to_owned)
                .context("expected a JSON string"),
            EnvType::Int => value
                .as_i64()
                .map(|value| value.to_string())
                .context("expected a JSON integer"),
            EnvType::Bool => value
                .as_bool()
                .map(|value| value.to_string())
                .context("expected a JSON boolean"),
            EnvType::Port => {
                let number = value.as_u64().context("expected a JSON integer")?;
                let number = u16::try_from(number).context("port is larger than 65535")?;
                if number == 0 {
                    bail!("port must be between 1 and 65535");
                }
                Ok(number.to_string())
            }
            EnvType::Path => {
                let value = value.as_str().context("expected a JSON string")?;
                validate_absolute_clean_path(Path::new(value), "path environment default")?;
                Ok(value.to_owned())
            }
        }
    }

    pub fn default_string(&self) -> Result<Option<String>> {
        self.default
            .as_ref()
            .map(|value| self.parse_json(value))
            .transpose()
    }
}

impl Dirs {
    pub fn roles(&self) -> [(&'static str, &'static str, &[PathBuf]); 5] {
        [
            ("state", "/var/lib", &self.state),
            ("cache", "/var/cache", &self.cache),
            ("logs", "/var/log", &self.logs),
            ("config", "/etc", &self.config),
            ("run", "/run", self.run.as_deref().unwrap_or_default()),
        ]
    }
}

pub fn parse_port(value: &str) -> Result<u16> {
    let port = value.parse::<u16>().context("expected a port number")?;
    if port == 0 {
        bail!("port must be between 1 and 65535");
    }
    Ok(port)
}

fn validate_exec(field: &str, exec: &[String], declarations: &BTreeMap<String, Env>) -> Result<()> {
    if exec.is_empty() {
        bail!("{field} must contain at least one argument");
    }
    if exec.iter().any(|arg| arg.contains(['\0', '\n', '\r'])) {
        bail!("{field} arguments must not contain NUL or newlines");
    }

    for arg in exec {
        for variable in referenced_variables(arg)
            .with_context(|| format!("invalid interpolation in {field} argument {arg:?}"))?
        {
            if !declarations.contains_key(&variable) {
                bail!("{field} references undeclared environment variable ${variable}");
            }
        }
    }
    Ok(())
}

pub fn referenced_variables(value: &str) -> Result<BTreeSet<String>> {
    let bytes = value.as_bytes();
    let mut variables = BTreeSet::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'$' {
            index += 1;
            continue;
        }
        index += 1;
        if index >= bytes.len() || !is_env_start(bytes[index]) {
            bail!("a '$' must be followed by an environment variable name");
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_env_continue(bytes[index]) {
            index += 1;
        }
        variables.insert(value[start..index].to_owned());
    }
    Ok(variables)
}

fn validate_name(kind: &str, name: &str) -> Result<()> {
    let mut chars = name.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphanumeric())
        || !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-'))
    {
        bail!(
            "{kind} name {name:?} must start with an ASCII letter or digit and contain only ASCII letters, digits, '.', '-', or '_'"
        );
    }
    Ok(())
}

fn validate_env_name(name: &str) -> Result<()> {
    let bytes = name.as_bytes();
    if bytes.is_empty()
        || !is_env_start(bytes[0])
        || !bytes[1..].iter().copied().all(is_env_continue)
    {
        bail!("environment variable name {name:?} must match [A-Za-z_][A-Za-z0-9_]*");
    }
    Ok(())
}

fn is_env_start(value: u8) -> bool {
    value.is_ascii_alphabetic() || value == b'_'
}

fn is_env_continue(value: u8) -> bool {
    is_env_start(value) || value.is_ascii_digit()
}

fn validate_app_path(version: u32, role: &str, root: &str, path: &Path) -> Result<()> {
    validate_absolute_clean_path(path, &format!("{role} directory"))?;
    if version == 2 {
        let relative = path.strip_prefix(root).ok();
        let is_one_component = relative.is_some_and(|relative| {
            let mut components = relative.components();
            matches!(components.next(), Some(Component::Normal(_))) && components.next().is_none()
        });
        if !is_one_component {
            bail!(
                "{role} directory {} must be exactly one component under {root}, as required by DESIGN.md \"Spec v2\" point 6",
                path.display()
            );
        }
        return Ok(());
    }
    let nix = Path::new("/nix");
    if path.starts_with(nix) || nix.starts_with(path) {
        bail!(
            "{role} directory {} must be outside /nix and must not contain it",
            path.display()
        );
    }
    Ok(())
}

fn validate_absolute_clean_path(path: &Path, label: &str) -> Result<()> {
    if !path.is_absolute() {
        bail!("{label} {} must be absolute", path.display());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        bail!(
            "{label} {} must not contain '.' or '..' components",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> Result<Spec> {
        Spec::from_slice(value.as_bytes())
    }

    #[test]
    fn parses_the_complete_schema() {
        let spec = parse(
            r#"{
                "cixSpec": 1,
                "services": {
                    "app": {
                        "exec": ["bin/app", "--port", "$PORT"],
                        "env": {
                            "PORT": {"type": "port", "default": 8080},
                            "READY": {"type": "bool", "required": true, "secret": false}
                        },
                        "ports": {"http": {"env": "PORT", "protocol": "tcp"}},
                        "dirs": {
                            "state": ["/var/lib/app"],
                            "cache": ["/var/cache/app"],
                            "logs": ["/var/log/app"],
                            "config": ["/etc/app"]
                        },
                        "health": {"exec": ["bin/health", "$READY"], "interval": "30s"},
                        "network": "host"
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(spec.services["app"].env["PORT"].kind, EnvType::Port);
        assert_eq!(spec.services["app"].ports["http"].protocol, Protocol::Tcp);
    }

    #[test]
    fn parses_v2_fields() {
        let spec = parse(
            r#"{
                "cixSpec": 2,
                "services": {
                    "app": {
                        "setup": ["bin/setup", "$PORT"],
                        "exec": ["bin/app", "$PORT"],
                        "env": {"PORT": {"type": "port", "default": 8080}},
                        "ports": {
                            "http": {"value": 8080, "protocol": "tcp"},
                            "admin": {"env": "PORT", "protocol": "tcp"}
                        },
                        "dirs": {"run": ["/run/app"]},
                        "jit": true
                    }
                }
            }"#,
        )
        .unwrap();
        let service = &spec.services["app"];
        assert_eq!(service.setup.as_ref().unwrap()[0], "bin/setup");
        assert_eq!(service.ports["http"].value, Some(8080));
        assert_eq!(
            service.dirs.run.as_deref().unwrap(),
            [PathBuf::from("/run/app")]
        );
        assert_eq!(service.jit, Some(true));
    }

    #[test]
    fn rejects_every_v2_field_under_v1() {
        for (field, json) in [
            (
                "setup",
                r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"setup":["bin/setup"]}}}"#,
            ),
            (
                "dirs.run",
                r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"dirs":{"run":[]}}}}"#,
            ),
            (
                "jit",
                r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"jit":false}}}"#,
            ),
            (
                "ports.http.value",
                r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"ports":{"http":{"value":8080,"protocol":"tcp"}}}}}"#,
            ),
        ] {
            let error = format!("{:#}", parse(json).unwrap_err());
            assert!(error.contains(field), "{error}");
            assert!(error.contains("requires cixSpec 2"), "{error}");
        }
    }

    #[test]
    fn rejects_ports_with_both_or_neither_source() {
        for json in [
            r#"{"cixSpec":2,"services":{"app":{"exec":["bin/app"],"env":{"PORT":{"type":"port"}},"ports":{"http":{"env":"PORT","value":8080,"protocol":"tcp"}}}}}"#,
            r#"{"cixSpec":2,"services":{"app":{"exec":["bin/app"],"ports":{"http":{"protocol":"tcp"}}}}}"#,
        ] {
            let error = format!("{:#}", parse(json).unwrap_err());
            assert!(
                error.contains("exactly one of \"env\" or \"value\""),
                "{error}"
            );
        }
    }

    #[test]
    fn v2_paths_must_be_one_component_under_the_role_root() {
        for json in [
            r#"{"cixSpec":2,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app/data"]}}}}"#,
            r#"{"cixSpec":2,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/cache/app"]}}}}"#,
            r#"{"cixSpec":2,"services":{"app":{"exec":["bin/app"],"dirs":{"run":["/run/app/socket"]}}}}"#,
        ] {
            let error = format!("{:#}", parse(json).unwrap_err());
            assert!(error.contains("exactly one component"), "{error}");
            assert!(error.contains("DESIGN.md \"Spec v2\" point 6"), "{error}");
        }
    }

    #[test]
    fn rejects_unknown_fields_at_every_level() {
        for json in [
            r#"{"cixSpec":1,"services":{},"future":true}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"future":true}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"env":{"X":{"type":"string","future":true}}}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"ports":{"x":{"env":"P","protocol":"tcp","future":true}}}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"dirs":{"future":[]}}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"health":{"exec":["bin/h"],"interval":"1s","future":true}}}}}"#,
        ] {
            assert!(parse(json).is_err(), "{json}");
        }
    }

    #[test]
    fn validates_interpolation_ports_and_directories() {
        let undeclared = r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app","$NOPE"]}}}"#;
        assert!(parse(undeclared)
            .unwrap_err()
            .to_string()
            .contains("invalid service"));

        let port = r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"env":{"P":{"type":"int"}},"ports":{"http":{"env":"P","protocol":"tcp"}}}}}"#;
        assert!(format!("{:#}", parse(port).unwrap_err()).contains("must have type \"port\""));

        for json in [
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/nix/data"]}}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app/../other"]}}}}"#,
            r#"{"cixSpec":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app"],"cache":["/var/lib/app/nested"]}}}}"#,
        ] {
            assert!(parse(json).is_err(), "{json}");
        }
    }
}
