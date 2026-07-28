use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_NIXPKGS_URL: &str = "github:NixOS/nixpkgs/nixos-unstable";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockFile {
    pub nixpkgs: NixpkgsLock,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NixpkgsLock {
    pub url: String,
    pub rev: String,
    #[serde(rename = "narHash")]
    pub nar_hash: String,
}

#[derive(Deserialize)]
struct FlakeMetadata {
    locked: LockedMetadata,
    url: String,
}

#[derive(Deserialize)]
struct LockedMetadata {
    rev: String,
    #[serde(default)]
    #[serde(rename = "narHash")]
    nar_hash: Option<String>,
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

pub fn ensure_lock(path: &Path, update: bool) -> Result<LockFile> {
    ensure_lock_with(path, update, resolve_default)
}

fn ensure_lock_with<F>(path: &Path, update: bool, resolve: F) -> Result<LockFile>
where
    F: FnOnce(bool) -> Result<LockFile>,
{
    if !update {
        match fs::read(path) {
            Ok(contents) => {
                let lock: LockFile = serde_json::from_slice(&contents)
                    .with_context(|| format!("parsing lock file {}", path.display()))?;
                lock.validate()?;
                return Ok(lock);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| format!("reading lock file {}", path.display()));
            }
        }
    }

    let lock = resolve(update)?;
    lock.validate()?;
    let mut contents = serde_json::to_vec_pretty(&lock)?;
    contents.push(b'\n');
    let temporary = path.with_extension("lock.tmp");
    fs::write(&temporary, contents)
        .with_context(|| format!("writing temporary lock file {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("atomically replacing lock file {}", path.display()))?;
    Ok(lock)
}

fn resolve_default(refresh: bool) -> Result<LockFile> {
    let mut arguments = vec!["flake", "metadata", "--json"];
    if refresh {
        arguments.push("--refresh");
    }
    arguments.push(DEFAULT_NIXPKGS_URL);
    let raw = cix_common::nix(&arguments).context("resolving the default nixpkgs channel")?;
    let metadata: FlakeMetadata =
        serde_json::from_str(&raw).context("parsing nix flake metadata")?;
    let nar_hash = match metadata.locked.nar_hash {
        Some(nar_hash) => nar_hash,
        None => archive_nar_hash(&metadata.url)?,
    };
    Ok(LockFile {
        nixpkgs: NixpkgsLock {
            url: DEFAULT_NIXPKGS_URL.to_owned(),
            rev: metadata.locked.rev,
            nar_hash,
        },
    })
}

fn archive_nar_hash(url: &str) -> Result<String> {
    let archive_raw = cix_common::nix(&["flake", "archive", "--json", url])
        .with_context(|| format!("archiving pinned nixpkgs source {url}"))?;
    let archive: FlakeArchive =
        serde_json::from_str(&archive_raw).context("parsing nix flake archive JSON")?;
    let info_raw = cix_common::nix(&["path-info", "--json", "--json-format", "1", &archive.path])
        .context("reading archived nixpkgs path information")?;
    let infos: BTreeMap<String, PathInfo> =
        serde_json::from_str(&info_raw).context("parsing nix path-info JSON")?;
    infos
        .get(&archive.path)
        .or_else(|| infos.values().next())
        .map(|info| info.nar_hash.clone())
        .context("nix path-info returned no archived nixpkgs path")
}

impl LockFile {
    fn validate(&self) -> Result<()> {
        if !self.nixpkgs.url.starts_with("github:") {
            bail!(
                "lock nixpkgs.url must be a github: URL, got {:?}",
                self.nixpkgs.url
            );
        }
        if self.nixpkgs.rev.is_empty() {
            bail!("lock nixpkgs.rev must not be empty");
        }
        if !self.nixpkgs.nar_hash.starts_with("sha256-") {
            bail!(
                "lock nixpkgs.narHash must be an SRI sha256 hash, got {:?}",
                self.nixpkgs.nar_hash
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    fn lock(revision: &str, hash: &str) -> LockFile {
        LockFile {
            nixpkgs: NixpkgsLock {
                url: DEFAULT_NIXPKGS_URL.into(),
                rev: revision.into(),
                nar_hash: hash.into(),
            },
        }
    }

    #[test]
    fn creates_reuses_and_updates_lock() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cixfile.lock");
        let resolutions = Cell::new(0);

        let created = ensure_lock_with(&path, false, |_| {
            resolutions.set(resolutions.get() + 1);
            Ok(lock("one", "sha256-one"))
        })
        .unwrap();
        assert_eq!(created.nixpkgs.rev, "one");
        assert_eq!(resolutions.get(), 1);

        let reused = ensure_lock_with(&path, false, |_| {
            resolutions.set(resolutions.get() + 1);
            Ok(lock("unexpected", "sha256-unexpected"))
        })
        .unwrap();
        assert_eq!(reused, created);
        assert_eq!(resolutions.get(), 1);

        let updated = ensure_lock_with(&path, true, |refresh| {
            assert!(refresh);
            resolutions.set(resolutions.get() + 1);
            Ok(lock("two", "sha256-two"))
        })
        .unwrap();
        assert_eq!(updated.nixpkgs.rev, "two");
        assert_eq!(resolutions.get(), 2);
        assert_eq!(
            serde_json::from_slice::<LockFile>(&fs::read(path).unwrap()).unwrap(),
            updated
        );
    }

    #[test]
    fn malformed_or_invalid_lock_fails_loudly_without_resolution() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Cixfile.lock");
        fs::write(
            &path,
            r#"{"nixpkgs":{"url":"github:NixOS/nixpkgs","rev":"x","narHash":"wrong"}}"#,
        )
        .unwrap();
        let error = ensure_lock_with(&path, false, |_| panic!("must not resolve"))
            .unwrap_err()
            .to_string();
        assert!(error.contains("SRI sha256 hash"), "{error}");
    }
}
