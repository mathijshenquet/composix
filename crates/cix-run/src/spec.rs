use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{bail, Context, Result};
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start: Vec<String>,
    /// Read-only sparse-rootfs paths projected from the store item in system mode.
    pub mounts: Option<Vec<PathBuf>>,
    /// Pre-start argv run in the service sandbox on every start.
    ///
    /// It follows the same output-relative executable and environment interpolation rules as
    /// `start` and must be idempotent.
    pub start_pre: Option<Vec<String>>,
    #[serde(default)]
    pub env: BTreeMap<String, Env>,
    #[serde(default)]
    pub secrets: BTreeMap<String, Secret>,
    #[serde(default)]
    pub ports: BTreeMap<String, Port>,
    /// Named systemd socket-activation file descriptors accepted by this service.
    #[serde(default)]
    pub listeners: BTreeMap<String, Listener>,
    #[serde(default)]
    pub dirs: Dirs,
    pub readiness: Option<Readiness>,
    pub liveness: Option<Liveness>,
    pub network: Option<Network>,
    // `grants` is reserved for the future compose-side loosening field.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<Claim>,
    pub shm: Option<String>,
    pub jit: Option<bool>,
    #[serde(default)]
    pub egress: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Claim {
    Named(String),
    Device(DeviceClaim),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceClaim {
    pub device: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Env {
    #[serde(rename = "type")]
    pub legacy_type: Option<String>,
    pub default: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub secret: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Secret {
    #[serde(rename = "as")]
    pub as_env: Option<String>,
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
    #[serde(default)]
    pub data: Vec<DataDir>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataDir {
    pub path: PathBuf,
    #[serde(default)]
    pub ro: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Readiness {
    #[serde(flatten)]
    pub probe: Probe,
    pub timeout: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Liveness {
    #[serde(flatten)]
    pub probe: Probe,
    pub interval: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Probe {
    #[serde(rename = "type")]
    pub probe_type: ProbeType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProbeType {
    Http,
    Tcp,
    Notify,
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
        if version != 0 {
            bail!("unsupported cixManifest version {version}; rebuild with the current cix")
        }
        let mut body = value
            .as_object()
            .cloned()
            .context("cix-manifest.json must be a JSON object")?;
        body.remove("cixManifest");
        if body.contains_key("health") {
            bail!(
                "manifest field \"health\" is obsolete; replace it with typed \"readiness\" and/or \"liveness\" probes (http, tcp, or notify)"
            );
        }
        let kind = body
            .remove("kind")
            .map(serde_json::from_value)
            .transpose()
            .context("cix-manifest.json field \"kind\" must be service or app")?
            .unwrap_or_default();
        let service: Service = serde_json::from_value(serde_json::Value::Object(body))
            .context("failed to parse cix-manifest.json def-node")?;
        let spec = Self {
            cix_manifest: 0,
            kind,
            services: BTreeMap::from([("artifact".to_owned(), service)]),
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn load(output: &Path) -> Result<Self> {
        let path = output.join("cix-manifest.json");
        if !path.exists() {
            bail!(
                "{} has no cix-manifest.json: it is a manifest-less ITEM (D68); items are build products, so use SERVICE/APP to declare a runnable contract",
                output.display()
            );
        }
        let json = fs::read(&path)
            .with_context(|| format!("failed to read manifest at {}", path.display()))?;
        let mut spec = Self::from_slice(&json)?;
        if let Some(name) = item_name_from_store_path(output) {
            let service = spec
                .services
                .remove("artifact")
                .expect("parsed bare def-node");
            spec.services.insert(name, service);
        }
        Ok(spec)
    }

    pub fn select_service<'a>(&'a self, requested: Option<&str>) -> Result<(&'a str, &'a Service)> {
        if requested.is_some() {
            bail!("a version-0 manifest is one def-node and has no #service selector")
        }
        let (name, service) = self.services.first_key_value().unwrap();
        Ok((name, service))
    }

    fn validate(&self) -> Result<()> {
        if self.cix_manifest != 0 {
            bail!(
                "unsupported cixManifest version {}; rebuild with the current cix",
                self.cix_manifest
            );
        }
        if self.services.is_empty() {
            bail!("cix-manifest.json must declare at least one service");
        }

        for (name, service) in &self.services {
            validate_name("service", name)?;
            service
                .validate(self.kind)
                .with_context(|| format!("invalid service {name:?}"))?;
        }
        Ok(())
    }
}

fn item_name_from_store_path(output: &Path) -> Option<String> {
    let name = output.file_name()?.to_str()?;
    let (_, name) = name.split_once('-')?;
    let name = name.strip_prefix("cix-item-").unwrap_or(name);
    validate_name("item", name).ok()?;
    Some(name.to_owned())
}

impl Service {
    fn validate(&self, kind: ManifestKind) -> Result<()> {
        validate_command("start", &self.start, &self.env)?;
        if let Some(start_pre) = &self.start_pre {
            validate_command("start_pre", start_pre, &self.env)?;
        }
        if let Some(readiness) = &self.readiness {
            readiness
                .probe
                .validate()
                .context("readiness probe is invalid")?;
            parse_duration(&readiness.timeout).context("readiness.timeout is invalid")?;
        }
        if let Some(liveness) = &self.liveness {
            liveness
                .probe
                .validate()
                .context("liveness probe is invalid")?;
            parse_duration(&liveness.interval).context("liveness.interval is invalid")?;
        }

        for name in self.env.keys() {
            validate_env_name(name)?;
        }
        for (name, secret) in &self.secrets {
            validate_name("secret", name)?;
            if let Some(as_env) = &secret.as_env {
                validate_env_name(as_env)?;
                if !as_env.ends_with("_FILE") {
                    bail!("secret {name:?} AS variable {as_env:?} must end in _FILE; it carries a credential path, never a secret value");
                }
                if self.env.contains_key(as_env) {
                    bail!("secret {name:?} AS variable {as_env:?} conflicts with declared env");
                }
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

        let mut seen: BTreeSet<&Path> = BTreeSet::new();
        for (role, root, paths) in self.dirs.roles() {
            for path in paths {
                validate_app_path(role, root, path)?;
                if !seen.insert(path) {
                    bail!(
                        "directory path {} is declared more than once",
                        path.display()
                    );
                }
            }
        }
        for data in &self.dirs.data {
            validate_absolute_clean_path(&data.path, "DIR path")?;
            if !seen.insert(&data.path) {
                bail!(
                    "directory path {} is declared more than once",
                    data.path.display()
                );
            }
        }
        validate_mounts(
            self.mounts.as_deref().unwrap_or_default(),
            &seen.into_iter().collect::<Vec<_>>(),
        )?;
        self.validate_capabilities()?;
        self.validate_kind(kind)?;
        Ok(())
    }

    fn validate_kind(&self, kind: ManifestKind) -> Result<()> {
        match kind {
            ManifestKind::Service => Ok(()),
            ManifestKind::App => {
                if self.start_pre.is_some() {
                    bail!("kind app must not declare start_pre (D47)");
                }
                if !self.ports.is_empty() {
                    bail!("kind app must not declare ports (D47)");
                }
                if !self.listeners.is_empty() {
                    bail!("kind app must not declare listeners (D47)");
                }
                if !self.dirs.logs.is_empty()
                    || !self.dirs.config.is_empty()
                    || !self.dirs.data.is_empty()
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

    pub fn has_claim(&self, claim: &str) -> bool {
        self.claims
            .iter()
            .any(|declared| matches!(declared, Claim::Named(name) if name == claim))
            || (claim == "jit" && self.jit == Some(true))
            || (claim == "egress" && self.egress)
    }

    pub fn device_claims(&self) -> impl Iterator<Item = &Path> {
        self.claims.iter().filter_map(|claim| match claim {
            Claim::Device(device) => Some(device.device.as_path()),
            Claim::Named(_) => None,
        })
    }

    pub fn has_device_claim(&self) -> bool {
        self.has_claim("gpu") || self.device_claims().next().is_some()
    }

    fn validate_capabilities(&self) -> Result<()> {
        if self.jit.is_some() || self.egress || self.network.is_some() {
            bail!("legacy capability fields are not supported; use the \"claims\" list")
        }
        let mut seen = BTreeSet::new();
        for claim in &self.claims {
            let key = match claim {
                Claim::Named(name) => {
                    if !matches!(name.as_str(), "jit" | "egress" | "gpu") {
                        bail!("unknown claim {name:?}; supported claims: jit, egress, gpu, device");
                    }
                    name.clone()
                }
                Claim::Device(device) => {
                    validate_device_path(&device.device)?;
                    format!("device:{}", device.device.display())
                }
            };
            if !seen.insert(key) {
                bail!("claim {claim:?} is declared more than once");
            }
        }
        if let Some(shm) = &self.shm {
            validate_systemd_size(shm)?;
        }
        Ok(())
    }
}

impl Probe {
    fn validate(&self) -> Result<()> {
        match (self.probe_type, self.target.as_deref()) {
            (ProbeType::Notify, None) => Ok(()),
            (ProbeType::Notify, Some(_)) => bail!("notify probes must not declare target"),
            (ProbeType::Http, Some(target)) => validate_http_target(target),
            (ProbeType::Tcp, Some(target)) => validate_tcp_target(target),
            (ProbeType::Http | ProbeType::Tcp, None) => {
                bail!("http and tcp probes must declare target")
            }
        }
    }
}

pub fn parse_duration(value: &str) -> Result<std::time::Duration> {
    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
    let amount = value
        .get(..digits)
        .filter(|amount| !amount.is_empty())
        .context("duration must start with a positive integer")?
        .parse::<u64>()
        .context("duration is too large")?;
    if amount == 0 {
        bail!("duration must be greater than zero");
    }
    let unit = value.get(digits..).unwrap_or_default();
    let milliseconds = match unit {
        "ms" => Some(amount),
        "s" => amount.checked_mul(1_000),
        "m" | "min" => amount.checked_mul(60_000),
        "h" => amount.checked_mul(3_600_000),
        "d" => amount.checked_mul(86_400_000),
        _ => bail!("duration must use ms, s, min, m, h, or d, for example 500ms or 10s"),
    }
    .context("duration is too large")?;
    Ok(std::time::Duration::from_millis(milliseconds))
}

pub fn format_duration(value: std::time::Duration) -> String {
    if value.subsec_millis() == 0 {
        format!("{}s", value.as_secs())
    } else {
        format!("{}ms", value.as_millis())
    }
}

fn validate_http_target(target: &str) -> Result<()> {
    let (authority, _) = target
        .split_once('/')
        .with_context(|| "http target must include an absolute path, for example :8080/healthz")?;
    validate_authority(authority)?;
    if target.contains(['\0', '\n', '\r', ' ']) {
        bail!("http target must not contain whitespace, NUL, or newlines");
    }
    Ok(())
}

fn validate_tcp_target(target: &str) -> Result<()> {
    if target.contains('/') {
        bail!("tcp target must be host:port without a path, for example :5432");
    }
    validate_authority(target)
}

fn validate_authority(authority: &str) -> Result<()> {
    if authority.contains(['\0', '\n', '\r', ' ', '/']) {
        bail!("probe target {authority:?} must be host:port");
    }
    let port = if let Some(port) = authority.strip_prefix(':') {
        port
    } else if authority.starts_with('[') {
        authority
            .split_once("]:")
            .filter(|(host, _)| host.len() > 1)
            .map(|(_, port)| port)
            .with_context(|| format!("probe target {authority:?} must be [ipv6]:port"))?
    } else {
        authority
            .rsplit_once(':')
            .filter(|(host, _)| !host.is_empty())
            .map(|(_, port)| port)
            .with_context(|| format!("probe target {authority:?} must be host:port"))?
    };
    parse_port(port).with_context(|| format!("probe target {authority:?} has an invalid port"))?;
    Ok(())
}

fn validate_device_path(path: &Path) -> Result<()> {
    if !path.is_absolute() || path == Path::new("/dev") || !path.starts_with("/dev") {
        bail!(
            "device claim {} must be an absolute path under /dev",
            path.display()
        );
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        bail!(
            "device claim {} must not contain '.' or '..' components",
            path.display()
        );
    }
    Ok(())
}

pub fn validate_systemd_size(size: &str) -> Result<()> {
    let digits = size.bytes().take_while(u8::is_ascii_digit).count();
    let suffix = size.get(digits..).unwrap_or_default().to_ascii_uppercase();
    let valid = digits > 0
        && matches!(
            suffix.as_str(),
            "" | "B"
                | "K"
                | "KB"
                | "KIB"
                | "M"
                | "MB"
                | "MIB"
                | "G"
                | "GB"
                | "GIB"
                | "T"
                | "TB"
                | "TIB"
                | "P"
                | "PB"
                | "PIB"
                | "E"
                | "EB"
                | "EIB"
        );
    if !valid {
        bail!("size {size:?} must use systemd size syntax, for example 64M or 1G");
    }
    Ok(())
}

impl Serialize for Spec {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let service = self
            .services
            .first_key_value()
            .map(|(_, service)| service)
            .ok_or_else(|| serde::ser::Error::custom("bare manifest has no def-node"))?;
        let mut body = serde_json::to_value(service).map_err(serde::ser::Error::custom)?;
        let object = body
            .as_object_mut()
            .ok_or_else(|| serde::ser::Error::custom("bare def-node is not an object"))?;
        object.insert("cixManifest".to_owned(), serde_json::Value::from(0));
        if self.kind != ManifestKind::Service {
            object.insert(
                "kind".to_owned(),
                serde_json::to_value(self.kind).map_err(serde::ser::Error::custom)?,
            );
        }
        body.serialize(serializer)
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

fn validate_command(
    field: &str,
    command: &[String],
    declarations: &BTreeMap<String, Env>,
) -> Result<()> {
    if command.is_empty() {
        bail!("{field} must contain at least one argument");
    }
    if command.iter().any(|arg| arg.contains(['\0', '\n', '\r'])) {
        bail!("{field} arguments must not contain NUL or newlines");
    }

    for arg in command {
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

fn validate_app_path(role: &str, root: &str, path: &Path) -> Result<()> {
    validate_absolute_clean_path(path, &format!("{role} directory"))?;
    if role == "config" && path.strip_prefix(root).is_err() {
        bail!("{role} directory {} must be under {root}", path.display());
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

    #[test]
    fn parses_and_serializes_the_v0_def_node() {
        let spec = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app","$PORT"],"env":{"PORT":{"default":"8080"}},"ports":{"http":{"env":"PORT","protocol":"tcp"}},"listeners":{"admin":{"type":"stream"}},"claims":["jit"]}"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        assert_eq!(service.start, ["bin/app", "$PORT"]);
        assert!(service.has_claim("jit"));
        assert_eq!(serde_json::to_value(&spec).unwrap()["cixManifest"], 0);
    }

    #[test]
    fn rejects_every_nonzero_version_with_the_rebuild_hint() {
        for version in [1, 2, 3, 4, 5, 99] {
            let error = Spec::from_slice(format!(r#"{{"cixManifest":{version}}}"#).as_bytes())
                .unwrap_err()
                .to_string();
            assert_eq!(
                error,
                format!("unsupported cixManifest version {version}; rebuild with the current cix")
            );
        }
    }

    #[test]
    fn validates_current_schema_fields() {
        for json in [
            r#"{"cixManifest":0,"start":["bin/app"],"jit":true}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"grants":["jit"]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"claims":["all"]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"listeners":{"dns":{"type":"datagram"}}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"ports":{"http":{"protocol":"tcp"}}}"#,
            r#"{"cixManifest":0,"services":{}}"#,
        ] {
            assert!(Spec::from_slice(json.as_bytes()).is_err(), "{json}");
        }
    }

    #[test]
    fn health_schema_is_typed_and_refuses_the_v0_exec_shape_with_a_migration() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/app"],
                "readiness": {"type": "http", "target": ":8080/healthz", "timeout": "90s"},
                "liveness": {"type": "tcp", "target": ":8080", "interval": "10s"}
            }"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        assert_eq!(
            service.readiness.as_ref().unwrap().probe.probe_type,
            ProbeType::Http
        );
        assert_eq!(
            service.liveness.as_ref().unwrap().probe.probe_type,
            ProbeType::Tcp
        );

        let error = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"health":{"exec":["bin/check"],"interval":"30s"}}"#,
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("field \"health\" is obsolete"), "{error}");
        assert!(error.contains("readiness"), "{error}");
        assert!(error.contains("liveness"), "{error}");
    }

    #[test]
    fn health_schema_validates_probe_targets_and_durations() {
        for json in [
            r#"{"cixManifest":0,"start":["bin/app"],"readiness":{"type":"notify","target":":8080","timeout":"10s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"readiness":{"type":"http","timeout":"10s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"readiness":{"type":"http","target":":8080","timeout":"10s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"liveness":{"type":"tcp","target":":8080/path","interval":"10s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"liveness":{"type":"tcp","target":":0","interval":"10s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"liveness":{"type":"notify","interval":"0s"}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"liveness":{"type":"exec","interval":"10s"}}"#,
        ] {
            assert!(Spec::from_slice(json.as_bytes()).is_err(), "{json}");
        }
        assert_eq!(format_duration(parse_duration("500ms").unwrap()), "500ms");
        assert_eq!(format_duration(parse_duration("2min").unwrap()), "120s");
    }

    #[test]
    fn validates_device_claim_forms_and_shm_sizes() {
        let accepted = Spec::from_slice(
            br#"{"cixManifest":0,"start":["bin/app"],"claims":["gpu",{"device":"/dev/video0"}],"shm":"256M"}"#,
        )
        .unwrap();
        let service = accepted.select_service(None).unwrap().1;
        assert!(service.has_claim("gpu"));
        assert_eq!(
            service.device_claims().collect::<Vec<_>>(),
            [Path::new("/dev/video0")]
        );

        for json in [
            r#"{"cixManifest":0,"start":["bin/app"],"claims":[{"device":"ttyUSB0"}]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"claims":[{"device":"/dev/null","extra":true}]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"claims":[{"gpu":"/dev/dri"}]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"claims":["gpu","gpu"]}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"shm":"-1G"}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"shm":"1Z"}"#,
        ] {
            assert!(Spec::from_slice(json.as_bytes()).is_err(), "{json}");
        }
    }

    #[test]
    fn app_constraints_and_manifestless_items_remain_explicit() {
        let error = format!("{:#}", Spec::from_slice(
            br#"{"cixManifest":0,"kind":"app","start":["bin/app"],"ports":{"http":{"value":8080,"protocol":"tcp"}}}"#,
        )
        .unwrap_err());
        assert!(error.contains("app must not declare ports"), "{error}");

        let item = tempfile::tempdir().unwrap();
        let error = Spec::load(item.path()).unwrap_err().to_string();
        assert!(error.contains("manifest-less ITEM (D68)"), "{error}");
    }

    #[test]
    fn directories_allow_arbitrary_clean_paths_and_reject_duplicates() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/app"],
                "dirs": {
                    "state": ["/srv/app/state"],
                    "cache": ["/app/cache"],
                    "logs": ["/app/logs"],
                    "run": ["/tmp/app/run"],
                    "data": [{"path": "/media", "ro": true}, {"path": "/consume", "ro": false}]
                }
            }"#,
        )
        .unwrap();
        assert!(spec.select_service(None).unwrap().1.dirs.data[0].ro);

        for json in [
            r#"{"cixManifest":0,"start":["bin/app"],"dirs":{"logs":["relative"]}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"dirs":{"run":["/tmp/../app"]}}"#,
            r#"{"cixManifest":0,"start":["bin/app"],"dirs":{"state":["/same"],"data":[{"path":"/same","ro":false}]}}"#,
        ] {
            assert!(Spec::from_slice(json.as_bytes()).is_err(), "{json}");
        }
    }
}
