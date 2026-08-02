use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Compose {
    pub cix_compose: u32,
    pub name: String,
    pub children: BTreeMap<String, Child>,
    #[serde(default)]
    pub log_namespace: bool,
    #[serde(default)]
    pub edges: BTreeMap<String, Edge>,
    #[serde(default)]
    pub secrets: BTreeMap<String, SecretSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub publish: BTreeMap<String, Publish>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Child {
    Item(ComposeService),
    Compose(ComposeRef),
    Group(Group),
}

impl<'de> Deserialize<'de> for Child {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        let value = Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("a child must be an object"))?;
        let discriminators = ["item", "compose", "children"]
            .into_iter()
            .filter(|field| object.contains_key(*field))
            .collect::<Vec<_>>();
        match discriminators.as_slice() {
            ["item"] => serde_json::from_value(value)
                .map(Child::Item)
                .map_err(D::Error::custom),
            ["compose"] => serde_json::from_value(value)
                .map(Child::Compose)
                .map_err(D::Error::custom),
            ["children"] => serde_json::from_value(value)
                .map(Child::Group)
                .map_err(D::Error::custom),
            [] => Err(D::Error::custom(
                "a child must declare exactly one of `item`, `compose`, or `children`",
            )),
            _ => Err(D::Error::custom(
                "a child may declare only one of `item`, `compose`, or `children`",
            )),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeRef {
    pub compose: String,
    #[serde(default)]
    pub update: UpdatePolicy,
    #[serde(default)]
    pub bind: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Group {
    pub children: BTreeMap<String, Child>,
    #[serde(default)]
    pub edges: BTreeMap<String, Edge>,
    #[serde(default)]
    pub secrets: BTreeMap<String, SecretSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub publish: BTreeMap<String, Publish>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bind: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Pod,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Publish {
    pub child: String,
    pub port: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SecretSource {
    pub file: Option<PathBuf>,
    pub encrypted: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComposeService {
    pub item: String,
    #[serde(default)]
    pub update: UpdatePolicy,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub bind: BTreeMap<String, String>,
    /// Per-path materialization overrides. Keys are in-service absolute paths.
    #[serde(default)]
    pub dirs: BTreeMap<PathBuf, DirectoryMaterialization>,
    /// A stable host identity used at an operator-owned host-directory seam.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub egress: Option<bool>,
}

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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdatePolicy {
    #[default]
    Pin,
    Track,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub producer: Producer,
    pub consumers: BTreeMap<String, Consumer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub child: String,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub paths: BTreeMap<String, LockedService>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LockedService {
    #[serde(rename = "ref")]
    pub reference: String,
    pub store_path: String,
    pub nar_hash: String,
}

impl Compose {
    pub fn load(path: &Path) -> Result<Self> {
        let contents =
            fs::read(path).with_context(|| format!("reading compose file {}", path.display()))?;
        let contents = interpolate_dotenv(&contents, path)?;
        reject_flat_v0(&contents, path)?;
        let compose: Self = from_slice_with_path(&contents, path)?;
        compose.validate_shape()?;
        Ok(compose)
    }

    pub(crate) fn load_artifact(path: &Path) -> Result<Self> {
        let contents = fs::read(path)
            .with_context(|| format!("reading compose artifact {}", path.display()))?;
        reject_flat_v0(&contents, path)?;
        let compose: Self = from_slice_with_path(&contents, path)?;
        compose.validate_shape()?;
        Ok(compose)
    }

    pub fn lock_path(compose_path: &Path) -> PathBuf {
        compose_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cix.lock")
    }

    pub fn root_add(&mut self, path: &str, reference: String) -> Result<()> {
        validate_ref("ref", &reference)?;
        let parts = validate_tree_path(path)?;
        insert_child(&mut self.children, &parts, reference)?;
        self.validate_shape()
    }

    pub fn root_remove(&mut self, path: &str) -> Result<()> {
        let parts = validate_tree_path(path)?;
        if !remove_child(&mut self.children, &parts)? {
            bail!("path {path:?} is not declared in the root tree");
        }
        self.validate_shape()
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)
            .with_context(|| format!("creating root directory {}", parent.display()))?;
        let temporary = path.with_extension("json.tmp");
        let mut contents = serde_json::to_vec_pretty(self)?;
        contents.push(b'\n');
        fs::write(&temporary, contents)
            .with_context(|| format!("writing root file {}", temporary.display()))?;
        fs::rename(&temporary, path)
            .with_context(|| format!("replacing root file {}", path.display()))
    }

    fn validate_shape(&self) -> Result<()> {
        if self.cix_compose != 1 {
            bail!(
                "cixCompose: unsupported compose version {}; rebuild this alpha compose artifact with the current cix (cixCompose: 1)",
                self.cix_compose
            );
        }
        validate_name("name", &self.name)?;
        validate_group(
            "",
            &self.children,
            &self.edges,
            &self.secrets,
            self.network,
            &self.publish,
        )
    }
}

fn validate_tree_path(path: &str) -> Result<Vec<&str>> {
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        bail!("path {path:?} must be one or more child names separated by '/'");
    }
    for part in &parts {
        validate_name("path", part)?;
    }
    Ok(parts)
}

fn insert_child(
    children: &mut BTreeMap<String, Child>,
    parts: &[&str],
    reference: String,
) -> Result<()> {
    let (head, tail) = parts.split_first().expect("validated nonempty path");
    if tail.is_empty() {
        if children.contains_key(*head) {
            bail!("path {:?} already exists", parts.join("/"));
        }
        children.insert(
            (*head).to_owned(),
            Child::Item(ComposeService {
                item: reference,
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
            }),
        );
        return Ok(());
    }
    let child = children.entry((*head).to_owned()).or_insert_with(|| {
        Child::Group(Group {
            children: BTreeMap::new(),
            edges: BTreeMap::new(),
            secrets: BTreeMap::new(),
            network: None,
            publish: BTreeMap::new(),
            bind: BTreeMap::new(),
        })
    });
    match child {
        Child::Group(group) => insert_child(&mut group.children, tail, reference),
        Child::Item(_) | Child::Compose(_) => {
            bail!("path component {head:?} is a ref, not an inline group")
        }
    }
}

fn remove_child(children: &mut BTreeMap<String, Child>, parts: &[&str]) -> Result<bool> {
    let (head, tail) = parts.split_first().expect("validated nonempty path");
    if tail.is_empty() {
        return Ok(children.remove(*head).is_some());
    }
    let Some(child) = children.get_mut(*head) else {
        return Ok(false);
    };
    let Child::Group(group) = child else {
        bail!("path component {head:?} is a ref, not an inline group");
    };
    let removed = remove_child(&mut group.children, tail)?;
    if removed && group.children.is_empty() {
        children.remove(*head);
    }
    Ok(removed)
}

fn validate_group(
    prefix: &str,
    children: &BTreeMap<String, Child>,
    edges: &BTreeMap<String, Edge>,
    secrets: &BTreeMap<String, SecretSource>,
    _network: Option<Network>,
    publish: &BTreeMap<String, Publish>,
) -> Result<()> {
    let field = |name: &str| {
        if prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{prefix}.{name}")
        }
    };
    if children.is_empty() {
        bail!(
            "{}: group must declare at least one child",
            field("children")
        );
    }
    for (name, child) in children {
        validate_name(&format!("{}.{}", field("children"), name), name)?;
        match child {
            Child::Item(service) => validate_service(&field("children"), name, service)?,
            Child::Compose(reference) => {
                validate_ref(
                    &format!("{}.{}.compose", field("children"), name),
                    &reference.compose,
                )?;
            }
            Child::Group(group) => validate_group(
                &format!("{}.{}", field("children"), name),
                &group.children,
                &group.edges,
                &group.secrets,
                group.network,
                &group.publish,
            )?,
        }
    }
    for (name, published) in publish {
        validate_name(&format!("{}.{}", field("publish"), name), name)?;
        validate_name(
            &format!("{}.{}.child", field("publish"), name),
            &published.child,
        )?;
        validate_name(
            &format!("{}.{}.port", field("publish"), name),
            &published.port,
        )?;
        if !children.contains_key(&published.child) {
            bail!(
                "{}.{}.child: unknown child {:?}",
                field("publish"),
                name,
                published.child
            );
        }
    }
    validate_secrets(&field("secrets"), secrets)?;
    for (name, edge) in edges {
        validate_name(&format!("{}.{}", field("edges"), name), name)?;
        validate_endpoint_path(
            &format!("{}.{}.producer.child", field("edges"), name),
            &edge.producer.child,
        )?;
        if !edge.producer.path.is_absolute() {
            bail!(
                "{}.{}.producer.path: path must be absolute, got {}",
                field("edges"),
                name,
                edge.producer.path.display()
            );
        }
        if edge.consumers.is_empty() {
            bail!(
                "{}.{}.consumers: edge must declare at least one consumer",
                field("edges"),
                name
            );
        }
        for (consumer, config) in &edge.consumers {
            validate_endpoint_path(
                &format!("{}.{}.consumers.{consumer}", field("edges"), name),
                consumer,
            )?;
            if config.path.as_ref().is_some_and(|path| !path.is_absolute()) {
                bail!(
                    "{}.{}.consumers.{consumer}.path: path must be absolute",
                    field("edges"),
                    name
                );
            }
        }
    }
    Ok(())
}

fn validate_service(parent: &str, name: &str, service: &ComposeService) -> Result<()> {
    let service_path = format!("{parent}.{name}");
    if service.item.is_empty() {
        bail!("{service_path}.item: item must not be empty");
    }
    validate_ref(&format!("{service_path}.item"), &service.item)?;
    if service
        .schedule
        .as_deref()
        .is_some_and(|schedule| schedule.trim().is_empty())
    {
        bail!("{service_path}.schedule: schedule must not be empty");
    }
    if service.schedule.is_none() && (service.persistent.is_some() || service.jitter.is_some()) {
        bail!("{service_path}: persistent and jitter require schedule");
    }
    if let Some(shm) = &service.shm {
        cix_run::spec::validate_systemd_size(shm).with_context(|| format!("{service_path}.shm"))?;
    }
    if service.identity.as_deref().is_some_and(str::is_empty) {
        bail!("{service_path}.identity: identity must not be empty");
    }
    if let Some(identity) = &service.identity {
        validate_name(&format!("{service_path}.identity"), identity)?;
    }
    for (dir_path, materialization) in &service.dirs {
        if !dir_path.is_absolute() {
            bail!(
                "{service_path}.dirs.{}: path must be absolute",
                dir_path.display()
            );
        }
        match (&materialization.host, &materialization.shared) {
            (Some(_), Some(_)) => bail!(
                "{service_path}.dirs.{}: host and shared are mutually exclusive",
                dir_path.display()
            ),
            (None, None) if materialization.role.is_none() => bail!(
                "{service_path}.dirs.{}: declare host, shared, or as",
                dir_path.display()
            ),
            _ => {}
        }
        if let Some(host) = &materialization.host {
            if !host.is_absolute() {
                bail!(
                    "{service_path}.dirs.{}.host: path must be absolute",
                    dir_path.display()
                );
            }
        }
        if materialization.idmap && materialization.host.is_none() {
            bail!(
                "{service_path}.dirs.{}.idmap: idmap: true only acknowledges a host bind",
                dir_path.display()
            );
        }
        if materialization.write && materialization.host.is_none() {
            bail!(
                "{service_path}.dirs.{}.write: write is only valid for an operator host bind",
                dir_path.display()
            );
        }
        if materialization.shared.as_deref().is_some_and(str::is_empty) {
            bail!(
                "{service_path}.dirs.{}.shared: shared name must not be empty",
                dir_path.display()
            );
        }
        if let Some(shared) = &materialization.shared {
            validate_name(
                &format!("{service_path}.dirs.{}.shared", dir_path.display()),
                shared,
            )?;
        }
    }
    Ok(())
}

fn validate_secrets(parent: &str, secrets: &BTreeMap<String, SecretSource>) -> Result<()> {
    for (name, source) in secrets {
        validate_name(&format!("{parent}.{name}"), name)?;
        match (&source.file, &source.encrypted) {
            (Some(_), None) | (None, Some(_)) => {}
            (Some(_), Some(_)) => {
                bail!("{parent}.{name}: file and encrypted are mutually exclusive")
            }
            (None, None) => bail!("{parent}.{name}: declare exactly one of file or encrypted"),
        }
        let path = source
            .file
            .as_ref()
            .or(source.encrypted.as_ref())
            .expect("validated source");
        if !path.is_absolute() {
            bail!("{parent}.{name}: source path must be absolute");
        }
    }
    Ok(())
}

fn validate_ref(path: &str, reference: &str) -> Result<()> {
    if reference.contains('$') {
        bail!(
            "{path}: parametric refs are expanded only at publish time; deployment refs must be concrete name:tag values"
        );
    }
    let Some((name, tag)) = reference.rsplit_once(':') else {
        bail!("{path}: refs must be fully qualified as name:tag, got {reference:?}");
    };
    if name.is_empty() || tag.is_empty() || tag.contains('/') {
        bail!("{path}: refs must be fully qualified as name:tag, got {reference:?}");
    }
    Ok(())
}

fn reject_flat_v0(contents: &[u8], path: &Path) -> Result<()> {
    let value: Value = serde_json::from_slice(contents)
        .with_context(|| format!("parsing compose file {}", path.display()))?;
    if value.get("services").is_some() || value.get("composeVersion").is_some() {
        bail!(
            "{}: flat compose v0 (`composeVersion`/`services`) is no longer accepted; migrate to `cixCompose: 1` and rename `services` to the tree-shaped `children` map",
            path.display()
        );
    }
    Ok(())
}

fn interpolate_dotenv(contents: &[u8], compose_path: &Path) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(contents)
        .with_context(|| format!("compose file {} is not UTF-8", compose_path.display()))?;
    let dotenv_path = compose_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(".env");
    let dotenv = match fs::read_to_string(&dotenv_path) {
        Ok(contents) => parse_dotenv(&contents, &dotenv_path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => BTreeMap::new(),
        Err(error) => {
            return Err(error).with_context(|| format!("reading {}", dotenv_path.display()))
        }
    };

    let mut rendered = String::with_capacity(text.len());
    let mut remainder = text;
    while let Some(start) = remainder.find("${") {
        rendered.push_str(&remainder[..start]);
        let after = &remainder[start + 2..];
        let end = after.find('}').with_context(|| {
            format!(
                "{}: unterminated .env interpolation",
                compose_path.display()
            )
        })?;
        let name = &after[..end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        {
            bail!(
                "{}: invalid .env interpolation ${{{name}}}",
                compose_path.display()
            );
        }
        let value = dotenv.get(name).with_context(|| {
            format!(
                "{}: ${{{name}}} is not defined by its own directory .env; ambient environment interpolation is refused",
                compose_path.display()
            )
        })?;
        rendered.push_str(value);
        remainder = &after[end + 1..];
    }
    rendered.push_str(remainder);
    Ok(rendered.into_bytes())
}

fn parse_dotenv(contents: &str, path: &Path) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for (index, line) in contents.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, value) = line
            .split_once('=')
            .with_context(|| format!("{}:{}: expected NAME=VALUE", path.display(), index + 1))?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        {
            bail!(
                "{}:{}: invalid .env name {name:?}",
                path.display(),
                index + 1
            );
        }
        if values.insert(name.to_owned(), value.to_owned()).is_some() {
            bail!(
                "{}:{}: duplicate .env name {name}",
                path.display(),
                index + 1
            );
        }
    }
    Ok(values)
}

impl Lock {
    pub fn load_optional(path: &Path) -> Result<Option<Self>> {
        match fs::read(path) {
            Ok(contents) => Ok(Some(from_slice_with_path(&contents, path)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => {
                Err(error).with_context(|| format!("reading lock file {}", path.display()))
            }
        }
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let temporary = path.with_extension("lock.tmp");
        let mut contents = serde_json::to_vec_pretty(self)?;
        contents.push(b'\n');
        fs::write(&temporary, contents)
            .with_context(|| format!("writing lock file {}", temporary.display()))?;
        fs::rename(&temporary, path)
            .with_context(|| format!("replacing lock file {}", path.display()))
    }
}

fn from_slice_with_path<T: DeserializeOwned>(contents: &[u8], path: &Path) -> Result<T> {
    let mut deserializer = serde_json::Deserializer::from_slice(contents);
    serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|error| anyhow::anyhow!("{}: {}: {}", path.display(), error.path(), error.inner()))
}

fn validate_name(path: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        bail!(
            "{path}: {value:?} must contain only lowercase ASCII letters, digits, '.', '_', or '-'"
        );
    }
    Ok(())
}

fn validate_endpoint_path(path: &str, value: &str) -> Result<()> {
    if value.is_empty() {
        bail!("{path}: endpoint path must not be empty");
    }
    for component in value.split('/') {
        validate_name(path, component)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item<'a>(compose: &'a Compose, name: &str) -> &'a ComposeService {
        match &compose.children[name] {
            Child::Item(service) => service,
            Child::Compose(_) | Child::Group(_) => panic!("expected item child"),
        }
    }

    #[test]
    fn unknown_fields_report_the_json_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"web":{"item":"x:v1","surprise":true}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("children.web"), "{error}");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn service_selector_is_rejected_by_d41() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"web":{"item":"x:v1","service":"app"}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("children.web"), "{error}");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn health_condition_vocabulary_is_rejected_everywhere_on_edges() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        for (fragment, expected_path) in [
            (
                r#""condition":"service_healthy","producer":{"child":"db","path":"/run/db"},"consumers":{"web":{}}"#,
                "edges.database.condition",
            ),
            (
                r#""producer":{"child":"db","path":"/run/db","health":"ready"},"consumers":{"web":{}}"#,
                "edges.database.producer.health",
            ),
            (
                r#""producer":{"child":"db","path":"/run/db"},"consumers":{"web":{"condition":"service_healthy"}}"#,
                "edges.database.consumers.web.condition",
            ),
        ] {
            fs::write(
                &path,
                format!(
                    r#"{{"cixCompose":1,"name":"x","children":{{"db":{{"item":"db:v1"}},"web":{{"item":"web:v1"}}}},"edges":{{"database":{{{fragment}}}}}}}"#
                ),
            )
            .unwrap();
            let error = Compose::load(&path).unwrap_err().to_string();
            assert!(error.contains(expected_path), "{fragment}: {error}");
            assert!(error.contains("unknown field"), "{fragment}: {error}");
        }
    }

    #[test]
    fn rejects_unknown_lock_fields_with_a_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cix.lock");
        fs::write(
            &path,
            r#"{"paths":{"web":{"ref":"x:v1","storePath":"/nix/store/x","narHash":"h","extra":1}}}"#,
        )
        .unwrap();
        let error = Lock::load_optional(&path).unwrap_err().to_string();
        assert!(error.contains("paths.web.extra"), "{error}");
    }

    #[test]
    fn rejects_empty_or_orphan_timer_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"job":{"item":"x:v1","schedule":"  "}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("children.job.schedule"), "{error}");

        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"job":{"item":"x:v1","persistent":true}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(
            error.contains("persistent and jitter require schedule"),
            "{error}"
        );
    }

    #[test]
    fn accepts_a_compose_level_log_namespace() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","logNamespace":true,"children":{"api":{"item":"x:v1"}}}"#,
        )
        .unwrap();
        assert!(Compose::load(&path).unwrap().log_namespace);
    }

    #[test]
    fn shm_is_validated_and_grants_remains_reserved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"api":{"item":"x:v1","shm":"128M"}}}"#,
        )
        .unwrap();
        assert_eq!(
            item(&Compose::load(&path).unwrap(), "api").shm.as_deref(),
            Some("128M")
        );
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"api":{"item":"x:v1","shm":"bad"}}}"#,
        )
        .unwrap();
        assert!(Compose::load(&path).is_err());
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"x","children":{"api":{"item":"x:v1","grants":["gpu"]}}}"#,
        )
        .unwrap();
        assert!(Compose::load(&path)
            .unwrap_err()
            .to_string()
            .contains("grants"));
    }

    #[test]
    fn dotenv_is_contained_to_the_compose_directory() {
        let directory = tempfile::tempdir().unwrap();
        let compose = directory.path().join("compose.json");
        fs::write(
            directory.path().join(".env"),
            "ITEM=demo:v1\nMESSAGE=hello\n",
        )
        .unwrap();
        fs::write(
            &compose,
            r#"{"cixCompose":1,"name":"demo","children":{"web":{"item":"${ITEM}","env":{"MESSAGE":"${MESSAGE}"}}}}"#,
        )
        .unwrap();
        let loaded = Compose::load(&compose).unwrap();
        assert_eq!(item(&loaded, "web").item, "demo:v1");
        assert_eq!(item(&loaded, "web").env["MESSAGE"], "hello");

        fs::write(
            &compose,
            r#"{"cixCompose":1,"name":"demo","children":{"web":{"item":"${AMBIENT}"}}}"#,
        )
        .unwrap();
        let error = Compose::load(&compose).unwrap_err().to_string();
        assert!(error.contains("own directory .env"), "{error}");
    }

    #[test]
    fn directory_materializations_are_strict() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"demo","children":{"web":{"item":"demo:v1","dirs":{"/data":{"host":"/srv/data","shared":"data"}}}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("mutually exclusive"), "{error}");
    }

    #[test]
    fn secret_sources_accept_one_absolute_file_or_encrypted_source() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"demo","children":{"web":{"item":"demo:v1"}},"secrets":{"plain":{"file":"/etc/cix/plain"},"sealed":{"encrypted":"/etc/cix/sealed"}}}"#,
        )
        .unwrap();
        let loaded = Compose::load(&path).unwrap();
        assert_eq!(
            loaded.secrets["plain"].file,
            Some(PathBuf::from("/etc/cix/plain"))
        );
        assert_eq!(
            loaded.secrets["sealed"].encrypted,
            Some(PathBuf::from("/etc/cix/sealed"))
        );

        for invalid in [
            r#"{"file":"relative"}"#,
            r#"{"file":"/etc/cix/plain","encrypted":"/etc/cix/sealed"}"#,
            r#"{}"#,
        ] {
            fs::write(
                &path,
                format!(
                    r#"{{"cixCompose":1,"name":"demo","children":{{"web":{{"item":"demo:v1"}}}},"secrets":{{"db":{invalid}}}}}"#
                ),
            )
            .unwrap();
            assert!(Compose::load(&path).is_err(), "{invalid}");
        }
    }

    #[test]
    fn accepts_inline_referenced_and_pod_groups_but_rejects_host_flag_and_flat_v0() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cix.json");
        fs::write(
            &path,
            r#"{
              "cixCompose": 1,
              "name": "root",
              "children": {
                "inline": {"children": {"api": {"item": "api:v1"}}},
                "sealed": {"compose": "suite:v2"}
              }
            }"#,
        )
        .unwrap();
        let loaded = Compose::load(&path).unwrap();
        assert!(matches!(loaded.children["inline"], Child::Group(_)));
        assert!(matches!(loaded.children["sealed"], Child::Compose(_)));

        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"root","network":"pod","children":{"api":{"item":"api:v1"}}}"#,
        )
        .unwrap();
        assert_eq!(Compose::load(&path).unwrap().network, Some(Network::Pod));

        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"root","network":"host","children":{"api":{"item":"api:v1"}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("network"), "{error}");
        assert!(error.contains("unknown variant"), "{error}");

        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"root","services":{"api":{"item":"api:v1"}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("flat compose v0"), "{error}");
        assert!(error.contains("children"), "{error}");
    }

    #[test]
    fn refs_are_qualified_and_root_edits_round_trip_structurally() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cix.json");
        fs::write(
            &path,
            r#"{"cixCompose":1,"name":"host","children":{"base":{"item":"base:v1"}}}"#,
        )
        .unwrap();
        let mut root = Compose::load(&path).unwrap();
        root.root_add("team/api", "api:v2".into()).unwrap();
        root.write(&path).unwrap();
        let loaded = Compose::load(&path).unwrap();
        let Child::Group(team) = &loaded.children["team"] else {
            panic!("team must be an inline group")
        };
        assert!(matches!(team.children["api"], Child::Item(_)));

        let mut loaded = loaded;
        loaded.root_remove("team/api").unwrap();
        loaded.write(&path).unwrap();
        assert!(!Compose::load(&path).unwrap().children.contains_key("team"));

        let error = root.root_add("bare", "api".into()).unwrap_err().to_string();
        assert!(error.contains("name:tag"), "{error}");

        let error = root
            .root_add("parametric", "api:$tag".into())
            .unwrap_err()
            .to_string();
        assert!(error.contains("publish time"), "{error}");
    }
}
