//! FETCH snapshot, volatility, and pin-refresh state.
//!
//! The build conductor chooses ordered FETCH steps; this owner records their
//! volatile facts, snapshot receipts, and refreshed lock pins.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};

use crate::memo::NeededPath;
use crate::{workspace, Builder, FetchPin, LockFile, ScratchDir, VolatilePath};

pub(crate) struct FetchState<'directory> {
    directory: &'directory Path,
}

pub(crate) struct PinRefreshRequest<'a> {
    pub(crate) previous: Option<&'a FetchPin>,
    pub(crate) expected: bool,
    pub(crate) force: bool,
    pub(crate) actual_paths: BTreeMap<String, String>,
    pub(crate) snapshot_nar_hash: &'a str,
    pub(crate) volatile: BTreeMap<String, VolatilePath>,
    pub(crate) name: &'a str,
}

pub(crate) struct SnapshotProbe {
    temporary: Option<ScratchDir>,
}

impl SnapshotProbe {
    pub(crate) fn path(&self) -> &Path {
        self.temporary
            .as_ref()
            .expect("FETCH probe snapshot is open")
            .path()
    }

    pub(crate) fn close(mut self) -> Result<()> {
        self.temporary
            .take()
            .expect("FETCH probe snapshot is open")
            .close()
    }
}

impl Drop for SnapshotProbe {
    fn drop(&mut self) {
        let Some(temporary) = self.temporary.take() else {
            return;
        };
        if let Err(error) = temporary.close() {
            eprintln!("warning: failed to clean FETCH probe snapshot: {error:#}");
        }
    }
}

impl<'directory> FetchState<'directory> {
    pub(crate) fn new(directory: &'directory Path) -> Self {
        Self { directory }
    }

    pub(crate) fn install_expected(
        lock: &mut LockFile,
        name: &str,
        expected: Option<&str>,
        message: impl FnOnce(&FetchPin) -> String,
    ) -> Result<()> {
        let Some(expected) = expected else {
            return Ok(());
        };
        match lock.fetches.get(name) {
            Some(pin) if pin.nar_hash != expected => bail!("{}", message(pin)),
            Some(_) => {}
            None => {
                lock.fetches
                    .insert(name.to_owned(), FetchPin::expected(expected.to_owned()));
            }
        }
        Ok(())
    }

    pub(crate) fn install_builder_expectations(
        lock: &mut LockFile,
        builder_name: &str,
        builder: &Builder,
        commands: &[String],
    ) -> Result<()> {
        let mut command_index = 0;
        for (index, step) in builder.steps.iter().enumerate() {
            match step {
                crate::BuildStep::Env { .. } => {}
                crate::BuildStep::Fetch {
                    expected,
                    line,
                    source,
                    ..
                } => {
                    let command = &commands[command_index];
                    let id = crate::lock::builder_fetch_id(builder_name, index, command);
                    Self::install_expected(lock, &id, expected.as_deref(), |pin| {
                        format!(
                            "line {line}: BUILDER {builder_name} FETCH EXPECT disagrees with its recorded lock pin\n  | {source:?}\n  declared {}\n  lock records {}",
                            expected.as_deref().expect("EXPECT was supplied"),
                            pin.nar_hash
                        )
                    })?;
                    command_index += 1;
                }
                crate::BuildStep::Run { .. } => command_index += 1,
                crate::BuildStep::Copy(_) => {}
            }
        }
        Ok(())
    }

    pub(crate) fn replay_snapshot(&self, name: &str, pin: &FetchPin) -> Result<String> {
        let receipt = self.snapshot_receipt(name, pin)?;
        let snapshot = fs::read_to_string(&receipt)
            .ok()
            .map(|text| text.trim().to_owned())
            .filter(|path| !path.is_empty())
            .filter(|path| workspace::ensure_store_path(path).unwrap_or(false));
        snapshot.with_context(|| {
            format!(
                "FETCH {name} has no locally cached replay snapshot at {}; run a non-cold build first (--cold never refetches)",
                receipt.display()
            )
        })
    }

    pub(crate) fn cache_snapshot(&self, name: &str, pin: &FetchPin, snapshot: &str) -> Result<()> {
        let receipt = self.snapshot_receipt(name, pin)?;
        let parent = receipt
            .parent()
            .expect("fetch snapshot receipt has a parent");
        fs::create_dir_all(parent)
            .with_context(|| format!("creating FETCH snapshot cache {}", parent.display()))?;
        fs::write(&receipt, format!("{snapshot}\n"))
            .with_context(|| format!("recording FETCH snapshot cache {}", receipt.display()))
    }

    pub(crate) fn snapshot(&self, source: &Path) -> Result<SnapshotProbe> {
        let snapshot =
            ScratchDir::new("cix-fetch-probe-").context("creating FETCH probe snapshot")?;
        workspace::copy_tree(source, snapshot.path())?;
        Ok(SnapshotProbe {
            temporary: Some(snapshot),
        })
    }

    pub(crate) fn volatile_paths(
        &self,
        first: &Path,
        second: &Path,
    ) -> Result<BTreeMap<String, VolatilePath>> {
        let mut first_nodes = BTreeMap::new();
        let mut second_nodes = BTreeMap::new();
        collect_files(first, Path::new(""), &mut first_nodes)?;
        collect_files(second, Path::new(""), &mut second_nodes)?;
        let names = first_nodes
            .keys()
            .chain(second_nodes.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut volatile = BTreeMap::new();
        for name in names {
            let before = first_nodes.get(&name);
            let after = second_nodes.get(&name);
            if before.map(|node| &node.0) != after.map(|node| &node.0) {
                volatile.insert(
                    name,
                    VolatilePath {
                        first_size: before.map_or(0, |node| node.1),
                        second_size: after.map_or(0, |node| node.1),
                    },
                );
            }
        }
        Ok(volatile)
    }

    pub(crate) fn consumed_volatility(
        observed: BTreeMap<String, VolatilePath>,
        needed: &BTreeMap<String, NeededPath>,
    ) -> BTreeMap<String, VolatilePath> {
        observed
            .into_iter()
            .filter(|(path, _)| {
                needed.keys().any(|needed_path| {
                    needed_path == "."
                        || path == needed_path
                        || path
                            .strip_prefix(needed_path)
                            .is_some_and(|suffix| suffix.starts_with('/'))
                })
            })
            .collect()
    }

    pub(crate) fn report_volatility(name: &str, volatile: &BTreeMap<String, VolatilePath>) {
        if volatile.is_empty() {
            eprintln!("FETCH {name} update probe: two outputs were identical");
            return;
        }
        eprintln!("FETCH {name} update probe found volatile files:");
        for (path, sizes) in volatile {
            eprintln!(
                "  {path} ({} B -> {} B)",
                sizes.first_size, sizes.second_size
            );
        }
    }

    pub(crate) fn consumed_path_hashes(
        workspace_path: &Path,
        needed: &BTreeMap<String, NeededPath>,
    ) -> Result<BTreeMap<String, String>> {
        let mut paths = BTreeMap::new();
        for path in needed.keys() {
            let source = if path == "." {
                workspace_path.to_owned()
            } else {
                workspace_path.join(path)
            };
            if !source.exists() && fs::symlink_metadata(&source).is_err() {
                bail!("FETCH-consumed path {path:?} does not exist");
            }
            paths.insert(path.clone(), workspace::nar_hash(&source)?);
        }
        Ok(paths)
    }

    pub(crate) fn refresh_pin(request: PinRefreshRequest<'_>) -> Result<FetchPin> {
        if request.expected {
            let mut pin = request
                .previous
                .cloned()
                .context("declared EXPECT pin was not installed")?;
            pin.snapshot_nar_hash = request.snapshot_nar_hash.to_owned();
            if !request.volatile.is_empty() {
                pin.volatile = request.volatile;
            }
            return Ok(pin);
        }

        let mut pin = request
            .previous
            .cloned()
            .unwrap_or_else(FetchPin::automatic);
        if !request.force && !pin.paths.is_empty() {
            for (path, pinned) in &pin.paths {
                let actual = request
                    .actual_paths
                    .get(path)
                    .with_context(|| format!("FETCH pin's consumed path {path:?} disappeared"))?;
                if actual != pinned {
                    bail!(
                        "FETCH consumed-path mismatch at {path:?}: lock pins {pinned}, fetched {actual}; rerun with --update-lock to accept the new output"
                    );
                }
            }
            for path in request
                .actual_paths
                .keys()
                .filter(|path| !pin.paths.contains_key(*path))
            {
                eprintln!(
                    "FETCH {} consumed a newly observed path {path:?}; recording a fresh pin entry",
                    request.name
                );
            }
        }
        pin.nar_hash.clear();
        pin.snapshot_nar_hash = request.snapshot_nar_hash.to_owned();
        pin.paths = request.actual_paths;
        if request.force {
            pin.volatile = request.volatile;
        }
        Ok(pin)
    }

    pub(crate) fn verify(
        expected: Option<&str>,
        pin: Option<&FetchPin>,
        actual: Option<&str>,
    ) -> Result<()> {
        if let Some(expected) = expected {
            if let Some(actual) = actual {
                if expected != actual {
                    bail!(
                        "FETCH EXPECT hash mismatch: declared {expected}, fetched {actual}. If a refetch of unchanged upstream diverges, the fetched tree is volatile: drop EXPECT and run `cix build --update-lock <fetch-or-builder>` to record TOFU consumed pins, or pin a stable asset URL."
                    );
                }
            } else if pin.is_none_or(|pin| pin.nar_hash != expected) {
                bail!("FETCH EXPECT hash mismatch: declared {expected}, lock has no matching pin");
            }
            return Ok(());
        }
        if let Some(pin) = pin {
            if pin.nar_hash.is_empty() && actual.is_none() {
                return Ok(());
            }
            let actual =
                actual.context("FETCH pin needs fetched bytes for whole-tree verification")?;
            if pin.nar_hash != actual {
                bail!(
                    "FETCH hash mismatch: lock pins {}, fetched {}; rerun with --update-lock to accept the new output",
                    pin.nar_hash,
                    actual
                );
            }
        }
        Ok(())
    }

    pub(crate) fn report_unconsumed_complement(
        name: &str,
        workspace_path: &Path,
        needed: &BTreeMap<String, NeededPath>,
    ) {
        const THRESHOLD_BYTES: u64 = 16 * 1024 * 1024;
        let total = tree_size(workspace_path).unwrap_or(0);
        let consumed = needed
            .keys()
            .map(|path| {
                let source = if path == "." {
                    workspace_path.to_owned()
                } else {
                    workspace_path.join(path)
                };
                tree_size(&source).unwrap_or(0)
            })
            .sum::<u64>();
        let complement = total.saturating_sub(consumed.min(total));
        if complement >= THRESHOLD_BYTES {
            eprintln!(
                "note: FETCH {name} leaves {} MiB unconsumed of {} MiB in its workspace; only COPY-reachable paths enter the pin",
                complement / (1024 * 1024),
                total / (1024 * 1024),
            );
        }
    }

    fn snapshot_receipt(&self, name: &str, pin: &FetchPin) -> Result<PathBuf> {
        let base = if let Some(path) = std::env::var_os("XDG_CACHE_HOME") {
            PathBuf::from(path)
        } else {
            PathBuf::from(
                std::env::var_os("HOME")
                    .context("HOME is unset; set XDG_CACHE_HOME for FETCH replay snapshots")?,
            )
            .join(".cache")
        };
        let directory = self.directory.canonicalize().with_context(|| {
            format!(
                "resolving Cixfile directory for FETCH snapshot cache {}",
                self.directory.display()
            )
        })?;
        let key = hex_hash(format!("{}\0{name}\0{}", directory.display(), pin.key()).as_bytes());
        Ok(base.join("cix/fetch-snapshots").join(key))
    }
}

fn collect_files(
    root: &Path,
    relative: &Path,
    files: &mut BTreeMap<String, (String, u64)>,
) -> Result<()> {
    let directory = root.join(relative);
    for entry in fs::read_dir(&directory)
        .with_context(|| format!("reading FETCH probe tree {}", directory.display()))?
    {
        let entry = entry?;
        let name = relative.join(entry.file_name());
        let path = root.join(&name);
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.is_dir() {
            collect_files(root, &name, files)?;
        } else {
            files.insert(
                name.to_string_lossy().into_owned(),
                (file_fingerprint(&path, &metadata)?, metadata.len()),
            );
        }
    }
    Ok(())
}

fn file_fingerprint(path: &Path, metadata: &fs::Metadata) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(metadata.permissions().mode().to_le_bytes());
    if metadata.file_type().is_symlink() {
        hasher.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else {
        let mut file = fs::File::open(path)?;
        io::copy(&mut file, &mut hasher)?;
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn tree_size(path: &Path) -> Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() {
        return Ok(metadata.len());
    }
    fs::read_dir(path)?.try_fold(0u64, |total, entry| {
        let entry = entry?;
        Ok(total.saturating_add(tree_size(&entry.path())?))
    })
}

fn hex_hash(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
