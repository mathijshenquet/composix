use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::{Input, InputKind};

pub const DEFAULT_NIXPKGS_URL: &str = "github:NixOS/nixpkgs/nixos-unstable";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockFile {
    pub inputs: BTreeMap<String, InputLock>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub artifacts: BTreeMap<String, ArtifactPin>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fetches: BTreeMap<String, FetchPin>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub memo: BTreeMap<String, MemoEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputLock {
    pub url: String,
    pub rev: String,
    #[serde(rename = "narHash")]
    pub nar_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactPin {
    #[serde(rename = "storePath")]
    pub store_path: String,
    #[serde(rename = "narHash")]
    pub nar_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FetchPin {
    #[serde(rename = "narHash")]
    pub nar_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoEntry {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub paths: BTreeMap<String, ConsumedPath>,
    #[serde(
        rename = "outputNarHash",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub legacy_output_nar_hash: Option<String>,
    #[serde(rename = "storePath", default, skip_serializing_if = "Option::is_none")]
    pub legacy_store_path: Option<String>,
    #[serde(rename = "wallMs", default, skip_serializing_if = "Option::is_none")]
    pub legacy_wall_ms: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumedPath {
    #[serde(rename = "narHash")]
    pub nar_hash: String,
    #[serde(rename = "storePath")]
    pub store_path: String,
}

#[derive(Deserialize)]
struct LegacyLockFile {
    nixpkgs: InputLock,
}

#[derive(Deserialize)]
struct FlakeMetadata {
    locked: LockedMetadata,
    url: String,
}

#[derive(Deserialize)]
struct LockedMetadata {
    #[serde(default)]
    rev: Option<String>,
    #[serde(default)]
    #[serde(rename = "narHash")]
    nar_hash: Option<String>,
    #[serde(default)]
    #[serde(rename = "lastModified")]
    last_modified: Option<u64>,
}

#[derive(Deserialize)]
struct FlakeArchive {
    path: String,
}

#[derive(Deserialize)]
struct PathInfo {
    #[serde(rename = "narHash")]
    nar_hash: String,
}

/// Reads, migrates, or resolves lock entries for the Cixfile's explicit FROM inputs.
/// `update` is None for reuse, Some(name) for one input, and Some("") for all inputs.
pub fn ensure_lock(
    path: &Path,
    inputs: &BTreeMap<String, Input>,
    update: Option<&str>,
) -> Result<LockFile> {
    let lock = ensure_lock_with(path, inputs, update, resolve_input, resolve_artifact)?;
    for input in inputs
        .values()
        .filter(|input| input.kind == InputKind::Artifact)
    {
        verify_artifact_pin(
            &input.url,
            lock.artifacts
                .get(&input.url)
                .expect("artifact lock was validated"),
        )?;
    }
    Ok(lock)
}

fn ensure_lock_with<F, A>(
    path: &Path,
    inputs: &BTreeMap<String, Input>,
    update: Option<&str>,
    mut resolve: F,
    mut resolve_artifact: A,
) -> Result<LockFile>
where
    F: FnMut(&str, bool) -> Result<InputLock>,
    A: FnMut(&str) -> Result<ArtifactPin>,
{
    if let Some(name) = update.filter(|name| !name.is_empty()) {
        let Some(input) = inputs.get(name) else {
            bail!("--update-lock names undeclared FROM namespace {name:?}");
        };
        if input.is_local() {
            bail!("FROM . AS {name} is the local build context and is not lock-pinned");
        }
    }

    let mut migrated = false;
    let existing = match fs::read(path) {
        Ok(contents) => {
            migrated = serde_json::from_slice::<LockFile>(&contents).is_err();
            Some(read_lock(&contents, inputs)?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(error).with_context(|| format!("reading lock file {}", path.display()))
        }
    };
    let was_missing = existing.is_none();
    let mut lock = existing.unwrap_or_else(|| LockFile {
        inputs: BTreeMap::new(),
        artifacts: BTreeMap::new(),
        fetches: BTreeMap::new(),
        memo: BTreeMap::new(),
    });
    let mut changed = false;
    for (name, input) in inputs {
        if input.is_local() {
            continue;
        }
        let refresh = update.is_some_and(|requested| requested.is_empty() || requested == name);
        match input.kind {
            InputKind::Artifact => {
                if refresh || !lock.artifacts.contains_key(&input.url) {
                    lock.artifacts
                        .insert(input.url.clone(), resolve_artifact(&input.url)?);
                    changed = true;
                }
            }
            InputKind::PackageUniverse | InputKind::Source => {
                if refresh || !lock.inputs.contains_key(name) {
                    lock.inputs
                        .insert(name.clone(), resolve(&input.url, refresh)?);
                    changed = true;
                }
            }
        }
    }
    lock.inputs.retain(|name, _| {
        inputs
            .get(name)
            .is_some_and(|input| !input.is_local() && input.kind != InputKind::Artifact)
    });
    let artifact_refs = inputs
        .values()
        .filter(|input| input.kind == InputKind::Artifact)
        .map(|input| input.url.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    lock.artifacts
        .retain(|reference, _| artifact_refs.contains(reference.as_str()));
    lock.validate_for(inputs)?;
    if changed || was_missing || migrated {
        write_lock(path, &lock)?;
    }
    Ok(lock)
}

fn read_lock(contents: &[u8], inputs: &BTreeMap<String, Input>) -> Result<LockFile> {
    if let Ok(lock) = serde_json::from_slice::<LockFile>(contents) {
        return Ok(lock);
    }
    let legacy: LegacyLockFile = serde_json::from_slice(contents).context("parsing lock file")?;
    let remote = inputs
        .iter()
        .filter(|(_, input)| !input.is_local())
        .collect::<Vec<_>>();
    if remote.len() != 1 {
        bail!("legacy single-input Cixfile.lock needs exactly one FROM input to migrate");
    }
    let name = remote[0].0.clone();
    Ok(LockFile {
        inputs: BTreeMap::from([(name, legacy.nixpkgs)]),
        artifacts: BTreeMap::new(),
        fetches: BTreeMap::new(),
        memo: BTreeMap::new(),
    })
}

fn resolve_artifact(reference: &str) -> Result<ArtifactPin> {
    let output = cix_index::resolve(reference).with_context(|| {
        format!("resolving cix-item FROM ref {reference:?}; pull it or tag it first")
    })?;
    Ok(ArtifactPin {
        store_path: output.store_path,
        nar_hash: output.nar_hash,
    })
}

fn verify_artifact_pin(reference: &str, pin: &ArtifactPin) -> Result<()> {
    let raw = cix_common::nix(&["path-info", "--json", "--json-format", "1", &pin.store_path])
        .with_context(|| format!("checking pinned cix-item FROM ref {reference:?}"))?;
    let infos: BTreeMap<String, PathInfo> =
        serde_json::from_str(&raw).context("parsing nix path-info JSON")?;
    let actual = infos
        .get(&pin.store_path)
        .or_else(|| infos.values().next())
        .context("nix path-info returned no pinned cix-item path")?;
    if actual.nar_hash != pin.nar_hash {
        bail!(
            "narHash mismatch for pinned cix-item FROM ref {reference:?}: lock has {}, local store has {}",
            pin.nar_hash,
            actual.nar_hash
        );
    }
    Ok(())
}

fn write_lock(path: &Path, lock: &LockFile) -> Result<()> {
    let mut contents = serde_json::to_vec_pretty(lock)?;
    contents.push(b'\n');
    let temporary = path.with_extension("lock.tmp");
    fs::write(&temporary, contents)
        .with_context(|| format!("writing temporary lock file {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("atomically replacing lock file {}", path.display()))?;
    Ok(())
}

pub fn save_lock(path: &Path, lock: &LockFile) -> Result<()> {
    write_lock(path, lock)
}

fn resolve_input(url: &str, refresh: bool) -> Result<InputLock> {
    let mut arguments = vec!["flake", "metadata", "--json"];
    if refresh {
        arguments.push("--refresh");
    }
    arguments.push(url);
    let raw = cix_common::nix(&arguments).with_context(|| format!("resolving FROM input {url}"))?;
    let metadata: FlakeMetadata =
        serde_json::from_str(&raw).context("parsing nix flake metadata")?;
    let nar_hash = match metadata.locked.nar_hash {
        Some(nar_hash) => nar_hash,
        None => archive_nar_hash(&metadata.url)?,
    };
    Ok(InputLock {
        url: url.to_owned(),
        rev: metadata
            .locked
            .rev
            .or_else(|| metadata.locked.last_modified.map(|value| value.to_string()))
            .context("nix did not report a revision or lastModified pin for FROM input")?,
        nar_hash,
    })
}

fn archive_nar_hash(url: &str) -> Result<String> {
    let archive_raw = cix_common::nix(&["flake", "archive", "--json", url])
        .with_context(|| format!("archiving pinned source {url}"))?;
    let archive: FlakeArchive =
        serde_json::from_str(&archive_raw).context("parsing nix flake archive JSON")?;
    let info_raw = cix_common::nix(&["path-info", "--json", "--json-format", "1", &archive.path])
        .context("reading archived source path information")?;
    let infos: BTreeMap<String, PathInfo> =
        serde_json::from_str(&info_raw).context("parsing nix path-info JSON")?;
    infos
        .get(&archive.path)
        .or_else(|| infos.values().next())
        .map(|info| info.nar_hash.clone())
        .context("nix path-info returned no archived source path")
}

impl LockFile {
    pub fn validate_for(&self, declared: &BTreeMap<String, Input>) -> Result<()> {
        for (name, input) in declared {
            if input.is_local() {
                if self.inputs.contains_key(name) {
                    bail!("lock must not pin local FROM . binder {name:?}");
                }
                continue;
            }
            if input.kind == InputKind::Artifact {
                let pin = self.artifacts.get(&input.url).with_context(|| {
                    format!("lock is missing cix-item FROM ref {:?}", input.url)
                })?;
                if !pin.store_path.starts_with("/nix/store/") {
                    bail!(
                        "lock cix-item ref {:?}.storePath must be a Nix store path, got {:?}",
                        input.url,
                        pin.store_path
                    );
                }
                if !pin.nar_hash.starts_with("sha256-") {
                    bail!(
                        "lock cix-item ref {:?}.narHash must be an SRI sha256 hash, got {:?}",
                        input.url,
                        pin.nar_hash
                    );
                }
                continue;
            }
            let lock = self
                .inputs
                .get(name)
                .with_context(|| format!("lock is missing FROM input {name:?}"))?;
            if lock.url != input.url {
                bail!(
                    "lock input {name:?} URL differs from FROM: {:?} != {:?}",
                    lock.url,
                    input.url
                );
            }
            if lock.rev.is_empty() {
                bail!("lock input {name:?}.rev must not be empty");
            }
            if !lock.nar_hash.starts_with("sha256-") {
                bail!(
                    "lock input {name:?}.narHash must be an SRI sha256 hash, got {:?}",
                    lock.nar_hash
                );
            }
        }
        for (name, pin) in &self.fetches {
            if !pin.nar_hash.starts_with("sha256-") {
                bail!(
                    "lock FETCH pin {name:?}.narHash must be an SRI sha256 hash, got {:?}",
                    pin.nar_hash
                );
            }
        }
        for (key, entry) in &self.memo {
            for (path, consumed) in &entry.paths {
                if !consumed.nar_hash.starts_with("sha256-") {
                    bail!(
                        "lock memo {key:?}.paths[{path:?}].narHash must be an SRI sha256 hash, got {:?}",
                        consumed.nar_hash
                    );
                }
                if !consumed.store_path.starts_with("/nix/store/") {
                    bail!(
                        "lock memo {key:?}.paths[{path:?}].storePath must be a Nix store path, got {:?}",
                        consumed.store_path
                    );
                }
            }
            if let Some(hash) = &entry.legacy_output_nar_hash {
                if !hash.starts_with("sha256-") {
                    bail!(
                        "lock memo {key:?}.outputNarHash must be an SRI sha256 hash, got {hash:?}"
                    );
                }
            }
            if let Some(path) = &entry.legacy_store_path {
                if !path.starts_with("/nix/store/") {
                    bail!("lock memo {key:?}.storePath must be a Nix store path, got {path:?}");
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn inputs() -> BTreeMap<String, Input> {
        BTreeMap::from([
            (
                "pkgs".into(),
                Input {
                    url: DEFAULT_NIXPKGS_URL.into(),
                    kind: crate::InputKind::PackageUniverse,
                    line: 1,
                },
            ),
            (
                "stable".into(),
                Input {
                    url: "github:NixOS/nixpkgs/nixos-25.05".into(),
                    kind: crate::InputKind::PackageUniverse,
                    line: 2,
                },
            ),
        ])
    }
    fn entry(url: &str, revision: &str) -> InputLock {
        InputLock {
            url: url.into(),
            rev: revision.into(),
            nar_hash: "sha256-one".into(),
        }
    }

    #[test]
    fn creates_reuses_and_updates_each_input() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cixfile.lock");
        let resolutions = Cell::new(0);
        let created = ensure_lock_with(
            &path,
            &inputs(),
            None,
            |url, _| {
                resolutions.set(resolutions.get() + 1);
                Ok(entry(url, "one"))
            },
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(created.inputs.len(), 2);
        let reused = ensure_lock_with(
            &path,
            &inputs(),
            None,
            |_, _| panic!("must reuse"),
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(reused, created);
        let updated = ensure_lock_with(
            &path,
            &inputs(),
            Some("pkgs"),
            |url, refresh| {
                assert!(refresh);
                resolutions.set(resolutions.get() + 1);
                Ok(entry(url, "two"))
            },
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(updated.inputs["pkgs"].rev, "two");
        assert_eq!(updated.inputs["stable"].rev, "one");
        assert_eq!(resolutions.get(), 3);
    }

    #[test]
    fn migrates_a_legacy_single_input_lock_and_writes_the_new_shape() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cixfile.lock");
        fs::write(&path, r#"{"nixpkgs":{"url":"github:NixOS/nixpkgs/nixos-unstable","rev":"one","narHash":"sha256-one"}}"#).unwrap();
        let declared = BTreeMap::from([(
            "pkgs".into(),
            Input {
                url: DEFAULT_NIXPKGS_URL.into(),
                kind: crate::InputKind::PackageUniverse,
                line: 1,
            },
        )]);
        let lock = ensure_lock_with(
            &path,
            &declared,
            None,
            |_, _| panic!("must migrate without resolving"),
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(lock.inputs["pkgs"].rev, "one");
        assert!(fs::read_to_string(path).unwrap().contains("\"inputs\""));
    }

    #[test]
    fn local_from_is_never_resolved_or_pinned() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cixfile.lock");
        let mut declared = inputs();
        declared.insert(
            "src".into(),
            Input {
                url: ".".into(),
                kind: crate::InputKind::Source,
                line: 3,
            },
        );
        let resolutions = Cell::new(0);
        let lock = ensure_lock_with(
            &path,
            &declared,
            None,
            |url, _| {
                resolutions.set(resolutions.get() + 1);
                Ok(entry(url, "one"))
            },
            |_| unreachable!(),
        )
        .unwrap();
        assert_eq!(resolutions.get(), 2);
        assert!(!lock.inputs.contains_key("src"));

        let error = ensure_lock_with(
            &path,
            &declared,
            Some("src"),
            |_, _| unreachable!(),
            |_| unreachable!(),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("not lock-pinned"), "{error}");
    }
}
