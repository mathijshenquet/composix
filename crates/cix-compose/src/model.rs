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
}
