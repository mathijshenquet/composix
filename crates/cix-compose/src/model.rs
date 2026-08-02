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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persistent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jitter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shm: Option<String>,
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
}
