//! Persistent and disposable builder workspaces.
//!
//! This module owns the mutable filesystem state of a build: its persisted
//! step state, staged COPY inputs, snapshots, reconciliation, fingerprints,
//! and conversion of workspace trees to store objects.  The build conductor
//! supplies ordered steps and lock policy, but never reaches through a shared
//! mutable context to alter this state.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ConsumedPath, OutputHash, ScratchDir, StepChange, StepMemo};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    pub(crate) step_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) materialized_memos: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) memo_output_fingerprints: BTreeMap<String, BTreeMap<String, String>>,
}

pub(crate) struct Workspace {
    work: PathBuf,
    staged: PathBuf,
    state: Option<PathBuf>,
    // Keeps a cold workspace alive for the duration of its build.
    _temporary: Option<ScratchDir>,
}

impl Workspace {
    pub(crate) fn persistent(base: &Path, directory: &Path, builder: &str) -> Result<Self> {
        let identity = workspace_identity(directory, builder);
        let root = base.join(identity);
        let work = root.join("work");
        let staged = root.join("staged");
        let state = root.join("state.json");
        fs::create_dir_all(&work)
            .with_context(|| format!("creating persistent builder workspace {}", work.display()))?;
        fs::create_dir_all(&staged)
            .with_context(|| format!("creating builder staging records {}", staged.display()))?;
        Ok(Self {
            work,
            staged,
            state: Some(state),
            _temporary: None,
        })
    }

    pub(crate) fn cold() -> Result<Self> {
        let temporary =
            ScratchDir::new("cix-build-cold-").context("creating cold builder workspace")?;
        let staged = temporary.path().join("staged");
        let work = temporary.path().join("work");
        fs::create_dir_all(&staged)?;
        fs::create_dir_all(&work)?;
        Ok(Self {
            work,
            staged,
            state: None,
            _temporary: Some(temporary),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.work
    }

    pub(crate) fn load_state(&self) -> State {
        self.state
            .as_deref()
            .and_then(|path| serde_json::from_slice(&fs::read(path).ok()?).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save_state(&self, state: &State) -> Result<()> {
        let Some(path) = &self.state else {
            return Ok(());
        };
        let temporary = path.with_extension("json.next");
        fs::write(&temporary, serde_json::to_vec_pretty(state)?)
            .with_context(|| format!("writing builder workspace state {}", temporary.display()))?;
        fs::rename(&temporary, path)
            .with_context(|| format!("replacing builder workspace state {}", path.display()))
    }

    pub(crate) fn stage_input(&self, source: &Path, dst: &str, index: usize) -> Result<()> {
        let baseline = self.staged.join(format!("step-{index}"));
        stage_input(source, dst, &self.work, &baseline)
    }

    pub(crate) fn replace_tree(&self, source: &Path) -> Result<()> {
        replace_tree(source, &self.work)
    }

    pub(crate) fn restore_snapshot(&self, snapshot: &Path) -> Result<()> {
        if !ensure_store_path(
            snapshot
                .to_str()
                .context("FETCH snapshot path is not UTF-8")?,
        )? {
            bail!(
                "pinned FETCH snapshot {} is unavailable locally; run --update-lock to refresh it",
                snapshot.display()
            );
        }
        self.replace_tree(snapshot)?;
        make_writable(&self.work)
    }

    pub(crate) fn store_consumed_paths(
        &self,
        needed: impl Iterator<Item = String>,
    ) -> Result<BTreeMap<String, ConsumedPath>> {
        store_consumed_paths(self.path(), needed)
    }

    pub(crate) fn add_step_output_snapshot(
        &self,
        changes: &BTreeMap<String, StepChange>,
        excluded: &BTreeSet<String>,
    ) -> Result<String> {
        add_step_output_snapshot(self.path(), changes, excluded)
    }

    pub(crate) fn apply_memo(
        &self,
        memo: &StepMemo,
        output_fingerprints: Option<&BTreeMap<String, String>>,
    ) -> Result<()> {
        apply_step_memo(memo, self.path(), output_fingerprints)
    }

    pub(crate) fn revert_memo(&self, memo: &StepMemo) -> Result<()> {
        revert_step_writes(memo, self.path())
    }

    pub(crate) fn memo_replayable(&self, memo: &StepMemo) -> Result<bool> {
        if memo.changes.is_empty() {
            return Ok(true);
        }
        let Some(snapshot) = memo.output_snapshot.as_deref() else {
            return Ok(false);
        };
        ensure_store_path(snapshot)
    }

    pub(crate) fn memo_self_state_matches(
        &self,
        memo: &StepMemo,
        current: &BTreeMap<String, crate::ReadDependency>,
        output_fingerprints: Option<&BTreeMap<String, String>>,
    ) -> Result<bool> {
        Ok(
            memo_write_set_matches_workspace(memo, self.path(), output_fingerprints)?
                && memo_self_reads_match(memo, self.path(), current, output_fingerprints)?,
        )
    }

    pub(crate) fn output_hashes(
        &self,
        changes: &BTreeMap<String, StepChange>,
    ) -> Result<BTreeMap<String, OutputHash>> {
        memo_output_hashes(self.path(), changes)
    }

    pub(crate) fn output_fingerprints(
        &self,
        changes: &BTreeMap<String, StepChange>,
    ) -> Result<BTreeMap<String, String>> {
        memo_output_fingerprints(self.path(), changes)
    }

    #[cfg(test)]
    pub(crate) fn borrowed_for_test(path: &Path) -> Self {
        Self {
            work: path.to_owned(),
            staged: path.join(".cix-test-staged"),
            state: None,
            _temporary: None,
        }
    }
}

pub(crate) fn ensure_store_path(path: &str) -> Result<bool> {
    if Path::new(path).exists() {
        return Ok(true);
    }
    cix_common::record_nix_subprocess();
    let output = Command::new("nix-store")
        .args(["--realise", path])
        .output()
        .with_context(|| format!("asking substituters for memo output {path}"))?;
    Ok(output.status.success() && Path::new(path).exists())
}

pub(crate) fn nar_hash(path: &Path) -> Result<String> {
    let path_text = path.to_str().context("path is not valid UTF-8")?;
    Ok(
        cix_common::nix(&["hash", "path", "--mode", "nar", path_text])?
            .trim()
            .to_owned(),
    )
}

pub(crate) fn add_store_object(path: &Path, name: &str) -> Result<String> {
    let path = path
        .to_str()
        .context("store input path is not valid UTF-8")?;
    cix_common::nix(&["store", "add", "--mode", "nar", "--name", name, path])?
        .lines()
        .last()
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .context("nix store add did not return a store path")
}

pub(crate) fn materialize_view(paths: &BTreeMap<String, ConsumedPath>) -> Result<String> {
    if let Some(whole) = paths.get(".") {
        return Ok(whole.store_path.clone());
    }
    let view = ScratchDir::new("cix-build-view-").context("creating consumed-path view")?;
    for (path, consumed) in paths {
        copy_node(Path::new(&consumed.store_path), &view.path().join(path))?;
    }
    add_store_object(view.path(), "cix-build-view")
}

pub(crate) fn memo_has_paths(
    entry: Option<&crate::MemoEntry>,
    needed: impl Iterator<Item = String>,
) -> Result<bool> {
    let Some(entry) = entry else {
        return Ok(false);
    };
    for path in needed {
        let Some(consumed) = entry.paths.get(&path) else {
            return Ok(false);
        };
        if !ensure_store_path(&consumed.store_path)?
            || nar_hash(Path::new(&consumed.store_path))? != consumed.nar_hash
        {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn memo_output_hashes(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
) -> Result<BTreeMap<String, OutputHash>> {
    changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| {
            Ok((
                path.clone(),
                OutputHash {
                    content: node_content_hash(&workspace.join(path))?
                        .context("memo output disappeared")?,
                },
            ))
        })
        .collect()
}

pub(crate) fn memo_output_fingerprints(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
) -> Result<BTreeMap<String, String>> {
    changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| {
            Ok((
                path.clone(),
                node_fingerprint(&workspace.join(path))?.context("memo output disappeared")?,
            ))
        })
        .collect()
}

fn memo_write_set_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    for (path, change) in &memo.changes {
        let workspace_path = workspace.join(path);
        match change {
            StepChange::Absent => match fs::symlink_metadata(&workspace_path) {
                Ok(_) => return Ok(false),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            },
            StepChange::Present | StepChange::Subtree { .. } | StepChange::Directory { .. } => {
                if !memo_output_matches_workspace(memo, workspace, path, output_fingerprints)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn memo_self_reads_match(
    memo: &StepMemo,
    workspace: &Path,
    current: &BTreeMap<String, crate::ReadDependency>,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    for (path, recorded) in &memo.reads {
        if current.get(path) != Some(recorded)
            && !memo_path_matches_workspace(memo, workspace, path, output_fingerprints)?
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn memo_path_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    path: &str,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    let Some(snapshot) = memo.output_snapshot.as_deref() else {
        return Ok(false);
    };
    if !memo
        .changes
        .keys()
        .any(|written| same_or_descendant(path, written) || same_or_descendant(written, path))
    {
        return Ok(false);
    }
    if let Some((root, _)) = memo
        .output_hashes
        .iter()
        .find(|(root, _)| same_or_descendant(path, root))
    {
        return memo_output_matches_workspace(memo, workspace, root, output_fingerprints);
    }
    Ok(node_content_hash(&Path::new(snapshot).join(path))?
        == node_content_hash(&workspace.join(path))?)
}

fn memo_output_matches_workspace(
    memo: &StepMemo,
    workspace: &Path,
    path: &str,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<bool> {
    let Some(expected) = memo.output_hashes.get(path) else {
        return Ok(false);
    };
    let current = workspace.join(path);
    let fingerprint = node_fingerprint(&current)?;
    if fingerprint.as_ref().is_some_and(|fingerprint| {
        output_fingerprints.and_then(|fingerprints| fingerprints.get(path)) == Some(fingerprint)
    }) {
        return Ok(true);
    }
    crate::cix_timing!(
        "CIX timing memo-output-fingerprint-miss path={} actual={}",
        path,
        fingerprint.as_deref().unwrap_or("<absent>")
    );
    Ok(node_content_hash(&current)? == Some(expected.content.clone()))
}

fn node_content_hash(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut digest = Sha256::new();
    digest.update((metadata.permissions().mode() & 0o111).to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if metadata.is_file() {
        digest.update(b"file\0");
        io::copy(&mut fs::File::open(path)?, &mut digest)?;
    } else if metadata.is_dir() {
        digest.update(b"directory\0");
        let mut entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            digest.update(entry.file_name().as_encoded_bytes());
            digest.update([0]);
            digest.update(
                node_content_hash(&entry.path())?
                    .unwrap_or_default()
                    .as_bytes(),
            );
            digest.update([0]);
        }
    } else {
        bail!(
            "unsupported special file in memo output: {}",
            path.display()
        );
    }
    Ok(Some(hex_hash(&digest.finalize())))
}

fn node_fingerprint(path: &Path) -> Result<Option<String>> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut digest = Sha256::new();
    digest.update(metadata.dev().to_le_bytes());
    digest.update(metadata.permissions().mode().to_le_bytes());
    digest.update(metadata.len().to_le_bytes());
    digest.update(metadata.ino().to_le_bytes());
    digest.update(metadata.mtime_nsec().to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
    } else if metadata.is_file() {
        digest.update(b"file\0");
    } else if metadata.is_dir() {
        digest.update(b"directory\0");
    } else {
        bail!(
            "unsupported special file in memo output: {}",
            path.display()
        );
    }
    Ok(Some(hex_hash(&digest.finalize())))
}

pub(crate) fn revert_step_writes(memo: &StepMemo, workspace: &Path) -> Result<()> {
    let mut paths = memo
        .changes
        .iter()
        .filter(|(_, change)| !matches!(change, StepChange::Absent))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| path.matches('/').count());
    let mut reverted = Vec::<&str>::new();
    for path in paths {
        if reverted
            .iter()
            .any(|parent| same_or_descendant(path, parent))
        {
            continue;
        }
        remove_path_if_present(&workspace.join(path))?;
        reverted.push(path);
    }
    Ok(())
}

fn apply_step_memo(
    memo: &StepMemo,
    workspace: &Path,
    output_fingerprints: Option<&BTreeMap<String, String>>,
) -> Result<()> {
    if memo.changes.is_empty() {
        return Ok(());
    }
    let snapshot = Path::new(
        memo.output_snapshot
            .as_deref()
            .context("step memo with filesystem changes has no output snapshot")?,
    );
    let mut absent = memo
        .changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Absent))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    absent.sort_by_key(|path| std::cmp::Reverse(path.matches('/').count()));
    for relative in absent {
        remove_path_if_present(&workspace.join(relative))?;
    }
    let mut present = memo
        .changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Present | StepChange::Subtree { .. }))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    present.sort_by_key(|path| path.matches('/').count());
    let apply_started = Instant::now();
    let mut synced = Vec::<&str>::new();
    for relative in present {
        if synced.iter().any(|parent| {
            relative
                .strip_prefix(*parent)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            continue;
        }
        if memo_output_matches_workspace(memo, workspace, relative, output_fingerprints)? {
            continue;
        }
        sync_replay_node(&snapshot.join(relative), &workspace.join(relative))?;
        synced.push(relative);
    }
    crate::cix_timing!(
        "CIX timing memo-apply roots={} wall_ms={}",
        synced.len(),
        apply_started.elapsed().as_millis()
    );
    for (relative, change) in &memo.changes {
        let mode = match change {
            StepChange::Directory { mode } | StepChange::Subtree { mode } => mode,
            _ => continue,
        };
        let path = if relative == "." {
            workspace.to_owned()
        } else {
            workspace.join(relative)
        };
        fs::set_permissions(path, fs::Permissions::from_mode(*mode))?;
    }
    Ok(())
}

pub(crate) fn add_step_output_snapshot(
    workspace: &Path,
    changes: &BTreeMap<String, StepChange>,
    excluded: &BTreeSet<String>,
) -> Result<String> {
    let delta = ScratchDir::new("cix-step-delta-").context("creating step output delta")?;
    let mut present = changes
        .iter()
        .filter(|(_, change)| matches!(change, StepChange::Present | StepChange::Subtree { .. }))
        .map(|(path, _)| path.as_str())
        .collect::<Vec<_>>();
    present.sort_by_key(|path| path.matches('/').count());
    let mut copied = Vec::<&str>::new();
    for relative in present {
        if copied.iter().any(|parent| {
            relative
                .strip_prefix(*parent)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            continue;
        }
        copy_node(&workspace.join(relative), &delta.path().join(relative))?;
        copied.push(relative);
    }
    for relative in excluded {
        remove_path_if_present(&delta.path().join(relative))?;
    }
    add_store_object(delta.path(), "cix-step-output")
}

pub(crate) fn store_consumed_paths(
    workspace: &Path,
    needed: impl Iterator<Item = String>,
) -> Result<BTreeMap<String, ConsumedPath>> {
    let mut paths = BTreeMap::new();
    for path in needed {
        let source = if path == "." {
            workspace.to_owned()
        } else {
            workspace.join(&path)
        };
        if !source.exists() && fs::symlink_metadata(&source).is_err() {
            bail!("consumed builder path {path:?} does not exist");
        }
        paths.insert(
            path,
            ConsumedPath {
                nar_hash: nar_hash(&source)?,
                store_path: add_store_object(&source, "cix-build-consumed")?,
            },
        );
    }
    Ok(paths)
}

pub(crate) fn workspace_identity(directory: &Path, builder: &str) -> String {
    hex_hash(format!("{}\0{builder}", directory.to_string_lossy()).as_bytes())
}

pub(crate) fn stage_input(
    source: &Path,
    dst: &str,
    workspace: &Path,
    baseline: &Path,
) -> Result<()> {
    let first_application = !baseline.exists();
    let next = baseline.with_extension("next");
    remove_path_if_present(&next)?;
    fs::create_dir_all(&next)?;
    copy_input(source, dst, &next)?;
    sync_directories(
        baseline.exists().then_some(baseline),
        &next,
        workspace,
        first_application,
    )?;
    make_writable(workspace)?;
    remove_path_if_present(baseline)?;
    fs::rename(&next, baseline).with_context(|| {
        format!(
            "replacing staged-input record {} with {}",
            baseline.display(),
            next.display()
        )
    })?;
    make_writable(baseline)
}

fn replace_tree(source: &Path, destination: &Path) -> Result<()> {
    remove_path_if_present(destination)?;
    fs::create_dir_all(destination)?;
    copy_tree(source, destination)
}

pub(crate) fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    for entry in fs::read_dir(source)
        .with_context(|| format!("reading snapshot directory {}", source.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)?;
        if metadata.file_type().is_symlink() {
            symlink(fs::read_link(&source_path)?, &destination_path)?;
        } else if metadata.is_dir() {
            fs::create_dir(&destination_path)?;
            copy_tree(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else if metadata.is_file() {
            fs::copy(&source_path, &destination_path)?;
            fs::set_permissions(&destination_path, metadata.permissions())?;
        } else {
            bail!(
                "unsupported special file in build snapshot: {}",
                source_path.display()
            );
        }
    }
    Ok(())
}

pub(crate) fn replace_tree_at(source: &Path, destination: &Path) -> Result<()> {
    replace_tree(source, destination)
}

fn copy_input(source: &Path, dst: &str, workdir: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)
        .with_context(|| format!("reading COPY source {}", source.display()))?;
    let destination = if dst == "." && !metadata.is_dir() {
        workdir.join(
            source
                .file_name()
                .context("COPY source has no final path component")?,
        )
    } else {
        workdir.join(dst)
    };
    if metadata.is_dir() {
        if dst == "." {
            copy_tree(source, workdir)?;
        } else {
            fs::create_dir(&destination)
                .with_context(|| format!("creating COPY directory {}", destination.display()))?;
            copy_tree(source, &destination)?;
            fs::set_permissions(&destination, metadata.permissions())?;
        }
        return Ok(());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating COPY destination {}", parent.display()))?;
    }
    if metadata.file_type().is_symlink() {
        symlink(fs::read_link(source)?, &destination).with_context(|| {
            format!(
                "copying symlink {} to build workdir destination {}",
                source.display(),
                destination.display()
            )
        })?;
        return Ok(());
    }
    if !metadata.is_file() {
        bail!("COPY source {} is a special file", source.display());
    }
    fs::copy(source, &destination).with_context(|| {
        format!(
            "copying {} to build workdir destination {}",
            source.display(),
            destination.display()
        )
    })?;
    fs::set_permissions(&destination, metadata.permissions())?;
    Ok(())
}

fn sync_directories(
    old: Option<&Path>,
    new: &Path,
    workspace: &Path,
    first_application: bool,
) -> Result<()> {
    let mut names = BTreeSet::new();
    if let Some(old) = old {
        for entry in fs::read_dir(old)? {
            names.insert(entry?.file_name());
        }
    }
    for entry in fs::read_dir(new)? {
        names.insert(entry?.file_name());
    }
    for name in names {
        sync_node(
            old.map(|root| root.join(&name)).as_deref(),
            Some(&new.join(&name)),
            &workspace.join(&name),
            first_application,
        )?;
    }
    Ok(())
}

fn sync_node(
    old: Option<&Path>,
    new: Option<&Path>,
    workspace: &Path,
    first_application: bool,
) -> Result<()> {
    let old = old.filter(|path| fs::symlink_metadata(path).is_ok());
    let new = new.filter(|path| fs::symlink_metadata(path).is_ok());
    let work_exists = fs::symlink_metadata(workspace).is_ok();
    match (old, new, work_exists) {
        (None, Some(new), false) => copy_node(new, workspace),
        (None, Some(new), true) if first_application && new.is_dir() && workspace.is_dir() => {
            sync_directories(None, new, workspace, true)
        }
        (None, Some(new), true) if first_application => {
            remove_path_if_present(workspace)?;
            copy_node(new, workspace)
        }
        (None, Some(_), true) | (None, None, _) | (Some(_), _, false) => Ok(()),
        (Some(old), Some(new), true) if nodes_equal(old, new)? => Ok(()),
        (Some(old), Some(new), true) if old.is_dir() && new.is_dir() && workspace.is_dir() => {
            sync_directories(Some(old), new, workspace, first_application)
        }
        (Some(old), None, true) if old.is_dir() && workspace.is_dir() => {
            let empty = tempfile::tempdir()?;
            sync_directories(Some(old), empty.path(), workspace, first_application)?;
            if fs::read_dir(workspace)?.next().is_none() {
                fs::remove_dir(workspace)?;
            }
            Ok(())
        }
        (Some(old), new, true) if nodes_equal(old, workspace)? => {
            remove_path_if_present(workspace)?;
            if let Some(new) = new {
                copy_node(new, workspace)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn sync_replay_node(source: &Path, destination: &Path) -> Result<()> {
    let source_meta = fs::symlink_metadata(source)?;
    let destination_meta = fs::symlink_metadata(destination).ok();
    if source_meta.is_dir() && !source_meta.file_type().is_symlink() {
        match &destination_meta {
            Some(existing) if existing.is_dir() && !existing.file_type().is_symlink() => {
                let mut names = BTreeSet::new();
                for entry in fs::read_dir(source)? {
                    names.insert(entry?.file_name());
                }
                for entry in fs::read_dir(destination)? {
                    let name = entry?.file_name();
                    if !names.contains(&name) {
                        remove_path_if_present(&destination.join(name))?;
                    }
                }
                for name in names {
                    sync_replay_node(&source.join(&name), &destination.join(&name))?;
                }
                Ok(())
            }
            _ => {
                remove_path_if_present(destination)?;
                copy_node(source, destination)?;
                make_writable(destination)
            }
        }
    } else if destination_meta.is_some() && nodes_equal(source, destination)? {
        Ok(())
    } else {
        remove_path_if_present(destination)?;
        copy_node(source, destination)?;
        make_writable(destination)
    }
}

fn nodes_equal(left: &Path, right: &Path) -> Result<bool> {
    let left_meta = fs::symlink_metadata(left)?;
    let right_meta = fs::symlink_metadata(right)?;
    if left_meta.file_type().is_symlink() || right_meta.file_type().is_symlink() {
        return Ok(left_meta.file_type().is_symlink()
            && right_meta.file_type().is_symlink()
            && fs::read_link(left)? == fs::read_link(right)?);
    }
    if left_meta.is_file() || right_meta.is_file() {
        return Ok(left_meta.is_file()
            && right_meta.is_file()
            && (left_meta.permissions().mode() & 0o111)
                == (right_meta.permissions().mode() & 0o111)
            && fs::read(left)? == fs::read(right)?);
    }
    if !left_meta.is_dir() || !right_meta.is_dir() {
        return Ok(false);
    }
    let mut left_names = fs::read_dir(left)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<Vec<_>>>()?;
    let mut right_names = fs::read_dir(right)?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<io::Result<Vec<_>>>()?;
    left_names.sort();
    right_names.sort();
    if left_names != right_names {
        return Ok(false);
    }
    for name in left_names {
        if !nodes_equal(&left.join(&name), &right.join(&name))? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn copy_node(source: &Path, destination: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if metadata.file_type().is_symlink() {
        symlink(fs::read_link(source)?, destination)?;
    } else if metadata.is_dir() {
        fs::create_dir(destination)?;
        copy_tree(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
    } else if metadata.is_file() {
        fs::copy(source, destination)?;
        fs::set_permissions(destination, metadata.permissions())?;
    } else {
        bail!("unsupported special file {}", source.display());
    }
    Ok(())
}

pub(crate) fn make_writable(path: &Path) -> Result<()> {
    let root_metadata = fs::symlink_metadata(path)?;
    if root_metadata.file_type().is_symlink() {
        return Ok(());
    }
    if !root_metadata.is_dir() {
        let mut permissions = root_metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o200);
        return Ok(fs::set_permissions(path, permissions)?);
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path)?;
        if metadata.is_dir() {
            make_writable(&entry_path)?;
        }
        if !metadata.file_type().is_symlink() {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() | 0o200);
            fs::set_permissions(&entry_path, permissions)?;
        }
    }
    let mut permissions = root_metadata.permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

pub(crate) fn remove_path_if_present(path: &Path) -> Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("reading {}", path.display())),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        make_writable(path)?;
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .with_context(|| format!("removing {}", path.display()))
}

fn same_or_descendant(candidate: &str, root: &str) -> bool {
    candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn hex_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
