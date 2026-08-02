use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Compose {
    pub compose_version: u32,
    pub name: String,
    pub services: BTreeMap<String, ComposeService>,
    #[serde(default)]
    pub log_namespace: bool,
    #[serde(default)]
    pub edges: BTreeMap<String, Edge>,
    #[serde(default)]
    pub secrets: BTreeMap<String, SecretSource>,
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
    pub service: String,
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
    pub services: BTreeMap<String, LockedService>,
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

    fn validate_shape(&self) -> Result<()> {
        if self.compose_version != 1 {
            bail!(
                "composeVersion: unsupported compose version {}; this cix supports version 1",
                self.compose_version
            );
        }
        validate_name("name", &self.name)?;
        if self.services.is_empty() {
            bail!("services: compose must declare at least one service");
        }
        for (name, service) in &self.services {
            validate_name(&format!("services.{name}"), name)?;
            if service.item.is_empty() {
                bail!("services.{name}.item: item must not be empty");
            }
            if service
                .schedule
                .as_deref()
                .is_some_and(|schedule| schedule.trim().is_empty())
            {
                bail!("services.{name}.schedule: schedule must not be empty");
            }
            if service.schedule.is_none()
                && (service.persistent.is_some() || service.jitter.is_some())
            {
                bail!("services.{name}: persistent and jitter require schedule");
            }
            if let Some(shm) = &service.shm {
                cix_run::spec::validate_systemd_size(shm)
                    .with_context(|| format!("services.{name}.shm"))?;
            }
            if service.identity.as_deref().is_some_and(str::is_empty) {
                bail!("services.{name}.identity: identity must not be empty");
            }
            if let Some(identity) = &service.identity {
                validate_name(&format!("services.{name}.identity"), identity)?;
            }
            for (path, materialization) in &service.dirs {
                if !path.is_absolute() {
                    bail!(
                        "services.{name}.dirs.{}: path must be absolute",
                        path.display()
                    );
                }
                match (&materialization.host, &materialization.shared) {
                    (Some(_), Some(_)) => bail!(
                        "services.{name}.dirs.{}: host and shared are mutually exclusive",
                        path.display()
                    ),
                    (None, None) if materialization.role.is_none() => bail!(
                        "services.{name}.dirs.{}: declare host, shared, or as",
                        path.display()
                    ),
                    _ => {}
                }
                if let Some(host) = &materialization.host {
                    if !host.is_absolute() {
                        bail!(
                            "services.{name}.dirs.{}.host: path must be absolute",
                            path.display()
                        );
                    }
                }
                if materialization.idmap && materialization.host.is_none() {
                    bail!(
                        "services.{name}.dirs.{}.idmap: idmap: true only acknowledges a host bind",
                        path.display()
                    );
                }
                if materialization.write && materialization.host.is_none() {
                    bail!(
                        "services.{name}.dirs.{}.write: write is only valid for an operator host bind",
                        path.display()
                    );
                }
                if materialization.shared.as_deref().is_some_and(str::is_empty) {
                    bail!(
                        "services.{name}.dirs.{}.shared: shared name must not be empty",
                        path.display()
                    );
                }
                if let Some(shared) = &materialization.shared {
                    validate_name(
                        &format!("services.{name}.dirs.{}.shared", path.display()),
                        shared,
                    )?;
                }
            }
        }
        for (name, source) in &self.secrets {
            validate_name(&format!("secrets.{name}"), name)?;
            match (&source.file, &source.encrypted) {
                (Some(_), None) | (None, Some(_)) => {}
                (Some(_), Some(_)) => {
                    bail!("secrets.{name}: file and encrypted are mutually exclusive")
                }
                (None, None) => bail!("secrets.{name}: declare exactly one of file or encrypted"),
            }
            let path = source
                .file
                .as_ref()
                .or(source.encrypted.as_ref())
                .expect("validated source");
            if !path.is_absolute() {
                bail!("secrets.{name}: source path must be absolute");
            }
        }
        for (name, edge) in &self.edges {
            validate_name(&format!("edges.{name}"), name)?;
            if !edge.producer.path.is_absolute() {
                bail!(
                    "edges.{name}.producer.path: path must be absolute, got {}",
                    edge.producer.path.display()
                );
            }
            if edge.consumers.is_empty() {
                bail!("edges.{name}.consumers: edge must declare at least one consumer");
            }
            for (consumer, config) in &edge.consumers {
                validate_name(&format!("edges.{name}.consumers.{consumer}"), consumer)?;
                if config.path.as_ref().is_some_and(|path| !path.is_absolute()) {
                    bail!("edges.{name}.consumers.{consumer}.path: path must be absolute");
                }
            }
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_fields_report_the_json_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"web":{"item":"x:v1","surprise":true}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("services.web.surprise"), "{error}");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn service_selector_is_rejected_by_d41() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"web":{"item":"x:v1","service":"app"}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("services.web.service"), "{error}");
        assert!(error.contains("unknown field"), "{error}");
    }

    #[test]
    fn health_condition_vocabulary_is_rejected_everywhere_on_edges() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        for (fragment, expected_path) in [
            (
                r#""condition":"service_healthy","producer":{"service":"db","path":"/run/db"},"consumers":{"web":{}}"#,
                "edges.database.condition",
            ),
            (
                r#""producer":{"service":"db","path":"/run/db","health":"ready"},"consumers":{"web":{}}"#,
                "edges.database.producer.health",
            ),
            (
                r#""producer":{"service":"db","path":"/run/db"},"consumers":{"web":{"condition":"service_healthy"}}"#,
                "edges.database.consumers.web.condition",
            ),
        ] {
            fs::write(
                &path,
                format!(
                    r#"{{"composeVersion":1,"name":"x","services":{{"db":{{"item":"db:v1"}},"web":{{"item":"web:v1"}}}},"edges":{{"database":{{{fragment}}}}}}}"#
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
            r#"{"services":{"web":{"ref":"x:v1","storePath":"/nix/store/x","narHash":"h","extra":1}}}"#,
        )
        .unwrap();
        let error = Lock::load_optional(&path).unwrap_err().to_string();
        assert!(error.contains("services.web.extra"), "{error}");
    }

    #[test]
    fn rejects_empty_or_orphan_timer_fields() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("compose.json");
        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"job":{"item":"x:v1","schedule":"  "}}}"#,
        )
        .unwrap();
        let error = Compose::load(&path).unwrap_err().to_string();
        assert!(error.contains("services.job.schedule"), "{error}");

        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"job":{"item":"x:v1","persistent":true}}}"#,
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
            r#"{"composeVersion":1,"name":"x","logNamespace":true,"services":{"api":{"item":"x:v1"}}}"#,
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
            r#"{"composeVersion":1,"name":"x","services":{"api":{"item":"x:v1","shm":"128M"}}}"#,
        )
        .unwrap();
        assert_eq!(
            Compose::load(&path).unwrap().services["api"].shm.as_deref(),
            Some("128M")
        );
        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"api":{"item":"x:v1","shm":"bad"}}}"#,
        )
        .unwrap();
        assert!(Compose::load(&path).is_err());
        fs::write(
            &path,
            r#"{"composeVersion":1,"name":"x","services":{"api":{"item":"x:v1","grants":["gpu"]}}}"#,
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
            r#"{"composeVersion":1,"name":"demo","services":{"web":{"item":"${ITEM}","env":{"MESSAGE":"${MESSAGE}"}}}}"#,
        )
        .unwrap();
        let loaded = Compose::load(&compose).unwrap();
        assert_eq!(loaded.services["web"].item, "demo:v1");
        assert_eq!(loaded.services["web"].env["MESSAGE"], "hello");

        fs::write(
            &compose,
            r#"{"composeVersion":1,"name":"demo","services":{"web":{"item":"${AMBIENT}"}}}"#,
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
            r#"{"composeVersion":1,"name":"demo","services":{"web":{"item":"demo:v1","dirs":{"/data":{"host":"/srv/data","shared":"data"}}}}}"#,
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
            r#"{"composeVersion":1,"name":"demo","services":{"web":{"item":"demo:v1"}},"secrets":{"plain":{"file":"/etc/cix/plain"},"sealed":{"encrypted":"/etc/cix/sealed"}}}"#,
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
                    r#"{{"composeVersion":1,"name":"demo","services":{{"web":{{"item":"demo:v1"}}}},"secrets":{{"db":{invalid}}}}}"#
                ),
            )
            .unwrap();
            assert!(Compose::load(&path).is_err(), "{invalid}");
        }
    }
}
