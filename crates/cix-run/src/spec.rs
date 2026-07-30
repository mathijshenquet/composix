use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Spec {
    pub cix_manifest: u32,
    pub kind: ManifestKind,
    pub services: BTreeMap<String, Service>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ManifestKind {
    #[default]
    Service,
    App,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySpec {
    #[serde(rename = "cixManifest")]
    cix_manifest: u32,
    services: BTreeMap<String, Service>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exec: Vec<String>,
    /// Read-only sparse-rootfs paths projected from the store item in system mode.
    pub mounts: Option<Vec<PathBuf>>,
    /// Pre-start argv run in the service sandbox on every start.
    ///
    /// It follows the same output-relative executable and environment interpolation rules as
    /// `exec` and must be idempotent.
    pub setup: Option<Vec<String>>,
    #[serde(default)]
    pub env: BTreeMap<String, Env>,
    #[serde(default)]
    pub ports: BTreeMap<String, Port>,
    /// Named systemd socket-activation file descriptors accepted by this service.
    #[serde(default)]
    pub listeners: BTreeMap<String, Listener>,
    #[serde(default)]
    pub dirs: Dirs,
    pub health: Option<Health>,
    pub network: Option<Network>,
    pub jit: Option<bool>,
    #[serde(default)]
    pub egress: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    #[serde(rename = "type")]
    /// Deprecated compatibility field. It is accepted and ignored; environment values are strings.
    #[deprecated(
        note = "the manifest's env `type` field is ignored and will be removed in cixManifest 3"
    )]
    pub legacy_type: Option<String>,
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub env: Option<String>,
    pub value: Option<u16>,
    pub protocol: Protocol,
}

/// A named inherited listener.
///
/// Manifest v3 deliberately supports only TCP stream listeners. The listener name is passed to the
/// service through systemd's `LISTEN_FDNAMES` protocol.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Listener {
    #[serde(rename = "type")]
    pub listener_type: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub exec: Vec<String>,
    pub interval: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Host,
}

impl Spec {
    pub fn from_slice(json: &[u8]) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_slice(json).context("failed to parse cix-manifest.json")?;
        let version = value
            .get("cixManifest")
            .and_then(serde_json::Value::as_u64)
            .context("cix-manifest.json field \"cixManifest\" must be an integer")?;
        let version = u32::try_from(version).context("cixManifest version is too large")?;
        reject_outbound_field(&value, version)?;
        let spec = if version == 4 {
            let mut body = value
                .as_object()
                .cloned()
                .context("cix-manifest.json must be a JSON object")?;
            body.remove("cixManifest");
            let kind = body
                .remove("kind")
                .map(serde_json::from_value)
                .transpose()
                .context("cix-manifest.json v4 field \"kind\" must be service or app")?
                .unwrap_or_default();
            let service: Service = serde_json::from_value(serde_json::Value::Object(body))
                .context("failed to parse cix-manifest.json v4 def-node")?;
            Self {
                cix_manifest: 4,
                kind,
                services: BTreeMap::from([("artifact".to_owned(), service)]),
            }
        } else {
            let legacy: LegacySpec =
                serde_json::from_value(value).context("failed to parse cix-manifest.json")?;
            Self {
                cix_manifest: legacy.cix_manifest,
                kind: ManifestKind::Service,
                services: legacy.services,
            }
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn load(output: &Path) -> Result<Self> {
        let path = output.join("cix-manifest.json");
        let json = fs::read(&path)
            .with_context(|| format!("failed to read manifest at {}", path.display()))?;
        let mut spec = Self::from_slice(&json)?;
        if spec.cix_manifest == 4 {
            if let Some(name) = item_name_from_store_path(output) {
                let service = spec
                    .services
                    .remove("artifact")
                    .expect("parsed v4 def-node");
                spec.services.insert(name, service);
            }
        }
        Ok(spec)
    }

    pub fn select_service<'a>(&'a self, requested: Option<&str>) -> Result<(&'a str, &'a Service)> {
        if self.cix_manifest == 4 {
            if requested.is_some() {
                bail!("cixManifest 4 is one bare def-node and has no #service selector (D41)");
            }
            let (name, service) = self.services.first_key_value().expect("validated v4 item");
            return Ok((name, service));
        }
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
                "deprecated multi-service cixManifest {} item declares {} services (available: {available}); D41 requires one item per service",
                self.cix_manifest,
                self.services.len(),
            );
        }
        let (name, service) = self.services.first_key_value().unwrap();
        Ok((name, service))
    }

    fn validate(&self) -> Result<()> {
        if !matches!(self.cix_manifest, 1..=4) {
            bail!(
                "unsupported cixManifest version {}; this cix supports versions 1, 2, 3, and 4",
                self.cix_manifest
            );
        }
        if self.services.is_empty() {
            bail!("cix-manifest.json must declare at least one service");
        }

        for (name, service) in &self.services {
            validate_name("service", name)?;
            service
                .validate(self.cix_manifest, self.kind)
                .with_context(|| format!("invalid service {name:?}"))?;
        }
        Ok(())
    }
}

fn reject_outbound_field(value: &serde_json::Value, version: u32) -> Result<()> {
    let has_outbound = if version == 4 {
        value.get("outbound").is_some()
    } else {
        value
            .get("services")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|services| {
                services.values().any(|service| {
                    service
                        .as_object()
                        .is_some_and(|service| service.contains_key("outbound"))
                })
            })
    };
    if has_outbound {
        bail!(
            "manifest field \"outbound\" was renamed to \"egress\" by D48(b); update cix-manifest.json"
        );
    }
    Ok(())
}

fn item_name_from_store_path(output: &Path) -> Option<String> {
    let name = output.file_name()?.to_str()?;
    let (_, name) = name.split_once('-')?;
    let name = name.strip_prefix("cix-item-").unwrap_or(name);
    validate_name("item", name).ok()?;
    Some(name.to_owned())
}

impl Service {
    fn validate(&self, version: u32, kind: ManifestKind) -> Result<()> {
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

        for name in self.env.keys() {
            validate_env_name(name)?;
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
                    if let Some(default) = &env.default {
                        parse_port(default).with_context(|| {
                            format!(
                                "default for ports-referenced environment variable {env_name:?} must be a port"
                            )
                        })?;
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

        for (name, listener) in &self.listeners {
            validate_name("listener", name)?;
            if self.ports.contains_key(name) {
                bail!("listener {name:?} conflicts with a port of the same name");
            }
            if listener.listener_type != "stream" {
                bail!(
                    "listener {name:?} type {:?} is not yet supported; only \"stream\" is supported",
                    listener.listener_type
                );
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
        validate_mounts(self.mounts.as_deref().unwrap_or_default(), &seen)?;
        self.validate_kind(kind)?;
        Ok(())
    }

    fn validate_kind(&self, kind: ManifestKind) -> Result<()> {
        match kind {
            ManifestKind::Service => Ok(()),
            ManifestKind::App => {
                if self.setup.is_some() {
                    bail!("kind app must not declare setup (D47)");
                }
                if !self.ports.is_empty() {
                    bail!("kind app must not declare ports (D47)");
                }
                if !self.listeners.is_empty() {
                    bail!("kind app must not declare listeners (D47)");
                }
                if self.health.is_some() {
                    bail!("kind app must not declare health (D47)");
                }
                if self.jit.is_some() {
                    bail!("kind app must not declare jit (D47)");
                }
                if !self.dirs.logs.is_empty()
                    || !self.dirs.config.is_empty()
                    || self
                        .dirs
                        .run
                        .as_ref()
                        .is_some_and(|paths| !paths.is_empty())
                {
                    bail!("kind app permits only state and cache directories (D47)");
                }
                Ok(())
            }
        }
    }

    pub fn has_network(&self) -> bool {
        !self.ports.is_empty() || self.network == Some(Network::Host)
    }

    fn validate_version_fields(&self, version: u32) -> Result<()> {
        if version == 1 {
            if self.setup.is_some() {
                bail!("field \"setup\" requires cixManifest 2");
            }
            if self.dirs.run.is_some() {
                bail!("field \"dirs.run\" requires cixManifest 2");
            }
            if self.jit.is_some() {
                bail!("field \"jit\" requires cixManifest 2");
            }
            if self.mounts.is_some() {
                bail!("field \"mounts\" requires cixManifest 2");
            }
            for (name, port) in &self.ports {
                if port.value.is_some() {
                    bail!("field \"ports.{name}.value\" requires cixManifest 2");
                }
            }
        }
        if version < 3 && !self.listeners.is_empty() {
            bail!("field \"listeners\" requires cixManifest 3");
        }
        if version < 4 && self.egress {
            bail!("field \"egress\" requires cixManifest 4");
        }
        if version == 4 && self.network.is_some() {
            bail!("field \"network\" is retired in cixManifest 4; use \"egress\" per D48(b)");
        }
        Ok(())
    }
}

impl Serialize for Spec {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.cix_manifest == 4 {
            let service = self
                .services
                .first_key_value()
                .map(|(_, service)| service)
                .ok_or_else(|| serde::ser::Error::custom("v4 manifest has no def-node"))?;
            let value = serde_json::to_value(service).map_err(serde::ser::Error::custom)?;
            let body = value
                .as_object()
                .ok_or_else(|| serde::ser::Error::custom("v4 def-node is not an object"))?;
            let kind_fields = usize::from(self.kind != ManifestKind::Service);
            let mut map = serializer.serialize_map(Some(body.len() + 1 + kind_fields))?;
            map.serialize_entry("cixManifest", &4)?;
            if self.kind != ManifestKind::Service {
                map.serialize_entry("kind", &self.kind)?;
            }
            for (name, value) in body {
                map.serialize_entry(name, value)?;
            }
            map.end()
        } else {
            let mut map = serializer.serialize_map(Some(2))?;
            map.serialize_entry("cixManifest", &self.cix_manifest)?;
            map.serialize_entry("services", &self.services)?;
            map.end()
        }
    }
}

fn validate_mounts(mounts: &[PathBuf], role_paths: &[&Path]) -> Result<()> {
    for (index, mount) in mounts.iter().enumerate() {
        validate_mount_path(mount)?;
        for other in &mounts[..index] {
            if mount.starts_with(other) || other.starts_with(mount) {
                bail!(
                    "mount paths {} and {} overlap; mounts must not be nested",
                    other.display(),
                    mount.display()
                );
            }
        }
        for role_path in role_paths {
            if mount.starts_with(role_path) || role_path.starts_with(mount) {
                bail!(
                    "mount path {} overlaps declared role directory {}",
                    mount.display(),
                    role_path.display()
                );
            }
        }
    }
    Ok(())
}

fn validate_mount_path(path: &Path) -> Result<()> {
    let value = path.to_str().context("mount path is not valid UTF-8")?;
    if !path.is_absolute() {
        bail!("mount path {value:?} must be absolute");
    }
    if value == "/" {
        bail!("mount path {value:?} is denied by the D22 v3 filesystem-projection rule");
    }
    if value.ends_with('/') || value.contains("//") {
        bail!("mount path {value:?} must be normalized and must not end in '/'");
    }
    if value
        .split('/')
        .any(|component| matches!(component, "." | ".."))
    {
        bail!("mount path {value:?} must be normalized and contain no '.' or '..' components");
    }
    if denied_mount_path(path) {
        bail!("mount path {value:?} is denied by the D22 v3 filesystem-projection rule");
    }
    Ok(())
}

fn denied_mount_path(path: &Path) -> bool {
    [
        "/nix",
        "/proc",
        "/sys",
        "/dev",
        "/run",
        "/var/lib",
        "/var/cache",
        "/var/log",
        "/etc/passwd",
        "/etc/group",
        "/etc/nsswitch.conf",
        "/etc",
        "/usr",
        "/bin",
    ]
    .iter()
    .any(|denied| path == Path::new(denied))
        || path.parent() == Some(Path::new("/"))
            && path
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
}

impl Env {
    pub fn default_string(&self) -> Option<String> {
        self.default.clone()
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
    if version >= 2 {
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
                "cixManifest": 1,
                "services": {
                    "app": {
                        "exec": ["bin/app", "--port", "$PORT"],
                        "env": {
                            "PORT": {"type": "port", "default": "8080"},
                            "READY": {"required": true, "secret": false}
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
        #[allow(deprecated)]
        let legacy_type = spec.services["app"].env["PORT"].legacy_type.as_deref();
        assert_eq!(legacy_type, Some("port"));
        assert_eq!(spec.services["app"].ports["http"].protocol, Protocol::Tcp);
    }

    #[test]
    fn parses_v2_fields() {
        let spec = parse(
            r#"{
                "cixManifest": 2,
                "services": {
                    "app": {
                        "setup": ["bin/setup", "$PORT"],
                        "exec": ["bin/app", "$PORT"],
                        "env": {"PORT": {"default": "8080"}},
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
    fn parses_v3_stream_listeners() {
        let spec = parse(
            r#"{
                "cixManifest": 3,
                "services": {
                    "app": {
                        "exec": ["bin/app"],
                        "listeners": {"http": {"type": "stream"}}
                    }
                }
            }"#,
        )
        .unwrap();
        assert_eq!(
            spec.services["app"].listeners["http"].listener_type,
            "stream"
        );
    }

    #[test]
    fn parses_and_serializes_a_bare_v4_def_node() {
        let spec = parse(
            r#"{
                "cixManifest": 4,
                "exec": ["bin/worker"],
                "env": {"MODE": {"default": "once"}},
                "egress": true
            }"#,
        )
        .unwrap();
        assert_eq!(spec.cix_manifest, 4);
        assert_eq!(spec.services.len(), 1);
        assert!(spec.services["artifact"].egress);
        let (_, service) = spec.select_service(None).unwrap();
        assert_eq!(service.exec, ["bin/worker"]);
        let serialized = serde_json::to_value(&spec).unwrap();
        assert_eq!(serialized["cixManifest"], 4);
        assert_eq!(serialized["exec"][0], "bin/worker");
        assert!(serialized.get("services").is_none());

        let defaulted = parse(r#"{"cixManifest":4,"exec":["bin/app"]}"#).unwrap();
        assert!(!defaulted.services["artifact"].egress);
        let error = defaulted
            .select_service(Some("app"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("D41"), "{error}");
    }

    #[test]
    fn outbound_manifest_field_has_a_d48_migration_error_and_is_not_an_alias() {
        for json in [
            r#"{"cixManifest":4,"exec":["bin/app"],"outbound":true}"#,
            r#"{"cixManifest":3,"services":{"app":{"exec":["bin/app"],"outbound":true}}}"#,
        ] {
            let error = Spec::from_slice(json.as_bytes()).unwrap_err().to_string();
            assert!(error.contains("renamed to \"egress\""), "{error}");
            assert!(error.contains("D48(b)"), "{error}");
        }
    }

    #[test]
    fn legacy_multi_service_items_emit_the_d41_deprecation() {
        let spec = parse(
            r#"{"cixManifest":3,"services":{
                "api":{"exec":["bin/api"]},
                "worker":{"exec":["bin/worker"]}
            }}"#,
        )
        .unwrap();
        let error = spec.select_service(None).unwrap_err().to_string();
        assert!(error.contains("deprecated"), "{error}");
        assert!(error.contains("D41"), "{error}");
        assert_eq!(
            spec.select_service(Some("api")).unwrap().1.exec,
            ["bin/api"]
        );
    }

    #[test]
    fn listener_requires_v3_and_only_stream_is_supported() {
        for version in [1, 2] {
            let error = format!(
                "{:#}",
                parse(&format!(
                    r#"{{"cixManifest":{version},"services":{{"app":{{"exec":["bin/app"],"listeners":{{"http":{{"type":"stream"}}}}}}}}}}"#
                ))
                .unwrap_err()
            );
            assert!(
                error.contains("field \"listeners\" requires cixManifest 3"),
                "{error}"
            );
        }
        let error = format!(
            "{:#}",
            parse(r#"{"cixManifest":3,"services":{"app":{"exec":["bin/app"],"listeners":{"dns":{"type":"datagram"}}}}}"#)
                .unwrap_err()
        );
        assert!(error.contains("not yet supported"), "{error}");
    }

    #[test]
    fn rejects_every_v2_field_under_v1() {
        for (field, json) in [
            (
                "setup",
                r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"setup":["bin/setup"]}}}"#,
            ),
            (
                "dirs.run",
                r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"dirs":{"run":[]}}}}"#,
            ),
            (
                "jit",
                r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"jit":false}}}"#,
            ),
            (
                "ports.http.value",
                r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"ports":{"http":{"value":8080,"protocol":"tcp"}}}}}"#,
            ),
            (
                "mounts",
                r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"mounts":["/etc/app"]}}}"#,
            ),
        ] {
            let error = format!("{:#}", parse(json).unwrap_err());
            assert!(error.contains(field), "{error}");
            assert!(error.contains("requires cixManifest 2"), "{error}");
        }
    }

    #[test]
    fn rejects_ports_with_both_or_neither_source() {
        for json in [
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"env":{"PORT":{}},"ports":{"http":{"env":"PORT","value":8080,"protocol":"tcp"}}}}}"#,
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"ports":{"http":{"protocol":"tcp"}}}}}"#,
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
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app/data"]}}}}"#,
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/cache/app"]}}}}"#,
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"dirs":{"run":["/run/app/socket"]}}}}"#,
        ] {
            let error = format!("{:#}", parse(json).unwrap_err());
            assert!(error.contains("exactly one component"), "{error}");
            assert!(error.contains("DESIGN.md \"Spec v2\" point 6"), "{error}");
        }
    }

    #[test]
    fn rejects_unknown_fields_at_every_level() {
        for json in [
            r#"{"cixManifest":1,"services":{},"future":true}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"future":true}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"env":{"X":{"type":"string","future":true}}}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"ports":{"x":{"env":"P","protocol":"tcp","future":true}}}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"dirs":{"future":[]}}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"health":{"exec":["bin/h"],"interval":"1s","future":true}}}}}"#,
        ] {
            assert!(parse(json).is_err(), "{json}");
        }
    }

    #[test]
    fn validates_interpolation_ports_and_directories() {
        let undeclared = r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app","$NOPE"]}}}"#;
        assert!(parse(undeclared)
            .unwrap_err()
            .to_string()
            .contains("invalid service"));

        let undeclared_port = r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"ports":{"http":{"env":"P","protocol":"tcp"}}}}}"#;
        assert!(format!("{:#}", parse(undeclared_port).unwrap_err())
            .contains("refers to undeclared environment variable \"P\""));

        let invalid_port_default = r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"env":{"P":{"default":"nope"}},"ports":{"http":{"env":"P","protocol":"tcp"}}}}}"#;
        assert!(format!("{:#}", parse(invalid_port_default).unwrap_err())
            .contains("default for ports-referenced environment variable \"P\" must be a port"));

        for json in [
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/nix/data"]}}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app/../other"]}}}}"#,
            r#"{"cixManifest":1,"services":{"app":{"exec":["bin/app"],"dirs":{"state":["/var/lib/app"],"cache":["/var/lib/app/nested"]}}}}"#,
        ] {
            assert!(parse(json).is_err(), "{json}");
        }
    }

    #[test]
    fn validates_mounts_adversarially() {
        let error = format!("{:#}", parse(
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"dirs":{"config":["/etc/app"]},"mounts":["/etc/app/config"]}}}"#,
        )
        .unwrap_err());
        assert!(
            error.contains("overlaps declared role directory"),
            "{error}"
        );

        let reverse_error = format!("{:#}", parse(
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"mounts":["/etc/app/config"],"dirs":{"config":["/etc/app"]}}}}"#,
        )
        .unwrap_err());
        assert!(
            reverse_error.contains("overlaps declared role directory"),
            "{reverse_error}"
        );

        let nested = format!("{:#}", parse(
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"mounts":["/etc/nginx","/etc/nginx/conf.d"]}}}"#,
        )
        .unwrap_err());
        assert!(nested.contains("must not be nested"), "{nested}");

        for denied in [
            "/nix",
            "/proc",
            "/sys",
            "/dev",
            "/run",
            "/var/lib",
            "/var/cache",
            "/var/log",
            "/etc/passwd",
            "/etc/group",
            "/etc/nsswitch.conf",
            "/",
            "/etc",
            "/usr",
            "/bin",
            "/lib",
            "/lib64",
        ] {
            let error = parse(&format!(
                r#"{{"cixManifest":2,"services":{{"app":{{"exec":["bin/app"],"mounts":["{denied}"]}}}}}}"#
            ))
            .unwrap_err()
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>()
            .join(": ");
            assert!(error.contains("D22 v3"), "{denied}: {error}");
        }

        parse(
            r#"{"cixManifest":2,"services":{"app":{"exec":["bin/app"],"mounts":["/cix-probe.conf","/opt/a/b/c/d","/etc/nginx"]}}}"#,
        )
        .unwrap();
    }

    #[test]
    fn rejects_non_normalized_mounts() {
        for mount in [
            "relative",
            "/etc/",
            "/etc/./nginx",
            "/etc/nginx/../ssl",
            "/etc//nginx",
        ] {
            let error = parse(&format!(
                r#"{{"cixManifest":2,"services":{{"app":{{"exec":["bin/app"],"mounts":["{mount}"]}}}}}}"#
            ))
            .unwrap_err()
            .chain()
            .map(|cause| cause.to_string())
            .collect::<Vec<_>>()
            .join(": ");
            assert!(error.contains("mount path"), "{mount}: {error}");
        }
    }
}

#[cfg(test)]
mod d47_kind_tests {
    use super::*;

    #[test]
    fn v4_kind_defaults_to_service_and_round_trips_app() {
        let service =
            Spec::from_slice(br#"{"cixManifest":4,"exec":["/nix/store/x/bin/service"]}"#).unwrap();
        assert_eq!(service.kind, ManifestKind::Service);
        assert!(!serde_json::to_string(&service)
            .unwrap()
            .contains("\"kind\""));

        let app = Spec::from_slice(
            br#"{"cixManifest":4,"kind":"app","exec":["/nix/store/x/bin/job"],"dirs":{"state":["/var/lib/job"],"cache":["/var/cache/job"]},"egress":true}"#,
        )
        .unwrap();
        assert_eq!(app.kind, ManifestKind::App);
        let encoded = serde_json::to_string(&app).unwrap();
        assert!(encoded.contains("\"kind\":\"app\""), "{encoded}");
    }

    #[test]
    fn removed_and_unknown_kinds_are_rejected() {
        let removed =
            Spec::from_slice(br#"{"cixManifest":4,"kind":"item","mounts":["/srv/data"]}"#)
                .unwrap_err()
                .to_string();
        assert!(removed.contains("service or app"), "{removed}");
        let error = Spec::from_slice(br#"{"cixManifest":4,"kind":"timer","exec":["/bin/true"]}"#)
            .unwrap_err()
            .to_string();
        assert!(error.contains("service or app"), "{error}");
    }

    #[test]
    fn kind_specific_fields_are_validated_for_external_manifests() {
        for (json, message) in [
            (
                r#"{"cixManifest":4,"kind":"app","exec":["/bin/true"],"ports":{"http":{"value":8080,"protocol":"tcp"}}}"#,
                "app must not declare ports",
            ),
            (
                r#"{"cixManifest":4,"kind":"app","exec":["/bin/true"],"setup":["/bin/true"]}"#,
                "app must not declare setup",
            ),
        ] {
            let error = format!("{:#}", Spec::from_slice(json.as_bytes()).unwrap_err());
            assert!(error.contains(message), "{error}");
        }
    }
}
