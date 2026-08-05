use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::lock::FileFingerprint;
use crate::{ReadDependency, StepChange};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Observation {
    pub(crate) listed: bool,
    content: bool,
    negative: bool,
    written: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Capture {
    observations: BTreeMap<String, Observation>,
    pub(crate) writes: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct FailureTrace {
    pub(crate) work_execs: Vec<String>,
    pub(crate) exec_enoent: BTreeSet<String>,
    pub(crate) missing_loaders: BTreeSet<String>,
    pub(crate) missing_sonames: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub(crate) struct ValidationMetrics {
    pub(crate) rehashed_files: usize,
    pub(crate) rehashed_bytes: u64,
}

#[derive(Debug, Default)]
pub(crate) struct RecordingMetrics {
    pub(crate) reused: usize,
    pub(crate) hashed_files: usize,
    pub(crate) hashed_bytes: u64,
    pub(crate) hashed_directories: usize,
}

pub(crate) fn parse(trace: &str) -> Capture {
    let mut observations = BTreeMap::new();
    let mut cwd = BTreeMap::<u32, PathBuf>::new();
    for line in trace.lines() {
        let Some((pid, call)) = split_pid(line) else {
            continue;
        };
        let Some(open) = call.find('(') else {
            continue;
        };
        let syscall = &call[..open];
        let arguments = &call[open + 1..];
        let succeeded = !call.contains(" = -1 ");

        if matches!(syscall, "clone" | "clone3" | "fork" | "vfork") && succeeded {
            if let Some(child) = return_value(call).and_then(|value| value.parse::<u32>().ok()) {
                if let Some(parent_cwd) = cwd.get(&pid).cloned() {
                    cwd.insert(child, parent_cwd);
                }
            }
            continue;
        }

        let quoted = quoted_strings(arguments);
        if syscall == "chdir" && succeeded {
            let base = annotated_base(arguments);
            if let Some(path) = quoted.first().and_then(|path| {
                resolve_path(
                    path,
                    base.as_deref()
                        .or_else(|| cwd.get(&pid).map(PathBuf::as_path)),
                )
            }) {
                cwd.insert(pid, path);
            }
        } else if syscall == "fchdir" && succeeded {
            if let Some(path) = annotated_fd_path(arguments) {
                cwd.insert(pid, path);
            }
        }

        let negative = call.contains(" = -1 ENOENT ");
        let listed = matches!(syscall, "getdents" | "getdents64");
        let open_path =
            matches!(syscall, "open" | "openat" | "openat2") && arguments.contains("O_PATH");
        let open_read = matches!(syscall, "open" | "openat" | "openat2")
            && !arguments.contains("O_WRONLY")
            && !open_path;
        let written = succeeded
            && (syscall == "creat"
                || matches!(syscall, "open" | "openat" | "openat2")
                    && (arguments.contains("O_WRONLY") || arguments.contains("O_RDWR"))
                || matches!(
                    syscall,
                    "mkdir"
                        | "mkdirat"
                        | "rmdir"
                        | "unlink"
                        | "unlinkat"
                        | "truncate"
                        | "chmod"
                        | "fchmodat"
                        | "mknod"
                        | "mknodat"
                        | "rename"
                        | "renameat"
                        | "renameat2"
                        | "link"
                        | "linkat"
                        | "symlink"
                        | "symlinkat"
                ));
        let content = open_read
            || arguments.contains("O_APPEND")
            || matches!(syscall, "readlink" | "readlinkat" | "execve" | "execveat");
        let should_record = negative
            || listed
            || matches!(
                syscall,
                "access"
                    | "faccessat"
                    | "faccessat2"
                    | "stat"
                    | "lstat"
                    | "newfstatat"
                    | "statx"
                    | "readlink"
                    | "readlinkat"
                    | "execve"
                    | "execveat"
                    | "chdir"
                    | "fchdir"
            )
            || open_read
            || open_path
            || written;
        if !should_record {
            continue;
        }

        let absolute = if listed || syscall == "fchdir" {
            annotated_fd_path(arguments)
        } else {
            let base = annotated_base(arguments);
            quoted.first().and_then(|path| {
                resolve_path(
                    path,
                    base.as_deref()
                        .or_else(|| cwd.get(&pid).map(PathBuf::as_path)),
                )
            })
        };
        let Some(relative) = absolute.and_then(work_relative) else {
            continue;
        };
        observations
            .entry(relative)
            .and_modify(|observation: &mut Observation| {
                if !observation.written {
                    observation.listed |= listed;
                    observation.content |= content;
                    observation.negative |= negative;
                }
                observation.written |= written;
            })
            .or_insert(Observation {
                listed,
                content,
                negative,
                written,
            });
        if written
            && matches!(
                syscall,
                "rename" | "renameat" | "renameat2" | "link" | "linkat" | "symlink" | "symlinkat"
            )
        {
            let destination_base = annotated_base(arguments);
            if let Some(destination) = quoted.last().and_then(|path| {
                resolve_path(
                    path,
                    cwd.get(&pid)
                        .map(PathBuf::as_path)
                        .or(destination_base.as_deref()),
                )
            }) {
                if let Some(relative) = work_relative(destination) {
                    observations
                        .entry(relative)
                        .and_modify(|observation| observation.written = true)
                        .or_insert(Observation {
                            written: true,
                            ..Observation::default()
                        });
                }
            }
        }
    }
    let writes = observations
        .iter()
        .filter(|(_, observation)| observation.written)
        .map(|(path, _)| path.clone())
        .collect();
    observations
        .retain(|_, observation| !observation.written || observation.content || observation.listed);
    Capture {
        observations,
        writes,
    }
}

pub(crate) fn parse_failure(trace: &str) -> FailureTrace {
    let mut failure = FailureTrace::default();
    let mut cwd = BTreeMap::<u32, PathBuf>::new();
    for line in trace.lines() {
        let Some((pid, call)) = split_pid(line) else {
            continue;
        };
        let Some(open) = call.find('(') else {
            continue;
        };
        let syscall = &call[..open];
        let arguments = &call[open + 1..];
        let succeeded = !call.contains(" = -1 ");
        if matches!(syscall, "clone" | "clone3" | "fork" | "vfork") && succeeded {
            if let Some(child) = return_value(call).and_then(|value| value.parse::<u32>().ok()) {
                if let Some(parent_cwd) = cwd.get(&pid).cloned() {
                    cwd.insert(child, parent_cwd);
                }
            }
            continue;
        }

        let quoted = quoted_strings(arguments);
        let base = annotated_base(arguments);
        let absolute = quoted.first().and_then(|path| {
            resolve_path(
                path,
                base.as_deref()
                    .or_else(|| cwd.get(&pid).map(PathBuf::as_path))
                    // strace reports host PIDs while clone may return namespace PIDs;
                    // command descendants still inherit bubblewrap's /work cwd.
                    .or(Some(Path::new("/work"))),
            )
        });
        if syscall == "chdir" && succeeded {
            if let Some(path) = absolute.clone() {
                cwd.insert(pid, path);
            }
        } else if syscall == "fchdir" && succeeded {
            if let Some(path) = annotated_fd_path(arguments) {
                cwd.insert(pid, path);
            }
        }

        let enoent = call.contains(" = -1 ENOENT ");
        if matches!(syscall, "execve" | "execveat") {
            if let Some(relative) = absolute.clone().and_then(work_relative) {
                if !failure.work_execs.contains(&relative) {
                    failure.work_execs.push(relative.clone());
                }
                if enoent {
                    failure.exec_enoent.insert(relative);
                }
            }
        }
        if !enoent {
            continue;
        }
        let Some(path) = absolute else {
            continue;
        };
        if crate::fhs::loader_aliases()
            .iter()
            .any(|alias| Path::new(alias.interpreter) == path)
        {
            failure
                .missing_loaders
                .insert(path.to_string_lossy().into_owned());
        }
        if matches!(syscall, "open" | "openat" | "openat2") {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if name.starts_with("lib") && (name.ends_with(".so") || name.contains(".so.")) {
                failure.missing_sonames.insert(name.to_owned());
            }
        }
    }
    failure
}

pub(crate) fn read_dependencies(
    snapshot: &Path,
    capture: &Capture,
) -> Result<BTreeMap<String, ReadDependency>> {
    Ok(read_dependencies_with_known(snapshot, capture, &BTreeMap::new())?.0)
}

pub(crate) fn read_dependencies_with_known(
    snapshot: &Path,
    capture: &Capture,
    known: &BTreeMap<String, ReadDependency>,
) -> Result<(BTreeMap<String, ReadDependency>, RecordingMetrics)> {
    let mut metrics = RecordingMetrics::default();
    let dependencies = capture
        .observations
        .iter()
        .map(|(relative, observation)| {
            if let Some(dependency) = known
                .get(relative)
                .and_then(|dependency| reuse_dependency(dependency, observation))
            {
                metrics.reused += 1;
                return Ok((relative.clone(), dependency));
            }
            let path = relative_path(snapshot, relative);
            match fs::symlink_metadata(&path) {
                Ok(metadata)
                    if metadata.is_dir()
                        && !metadata.file_type().is_symlink()
                        && observation.listed =>
                {
                    metrics.hashed_directories += 1;
                }
                Ok(metadata)
                    if !metadata.is_dir()
                        && !metadata.file_type().is_symlink()
                        && observation.content =>
                {
                    metrics.hashed_files += 1;
                    metrics.hashed_bytes += metadata.len();
                }
                Ok(metadata) if metadata.file_type().is_symlink() && observation.content => {
                    metrics.hashed_files += 1;
                    metrics.hashed_bytes += metadata.len();
                }
                Ok(_) => {}
                Err(error) if path_is_missing(&error) => {}
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("reading traced path {}", path.display()))
                }
            }
            Ok((
                relative.clone(),
                dependency(&path, observation.listed, observation.content)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok((dependencies, metrics))
}

fn reuse_dependency(known: &ReadDependency, observation: &Observation) -> Option<ReadDependency> {
    match known {
        ReadDependency::File { .. } if observation.content => Some(known.clone()),
        ReadDependency::File { .. } | ReadDependency::FileExists
            if !observation.listed && !observation.content =>
        {
            Some(ReadDependency::FileExists)
        }
        ReadDependency::Directory { .. } if observation.listed => Some(known.clone()),
        ReadDependency::Directory { .. } | ReadDependency::DirectoryExists
            if !observation.listed =>
        {
            Some(ReadDependency::DirectoryExists)
        }
        ReadDependency::Subtree { .. } if observation.listed => Some(known.clone()),
        ReadDependency::Absent => Some(ReadDependency::Absent),
        _ => None,
    }
}

pub(crate) fn current_dependencies(
    workspace: &Path,
    recorded: &BTreeMap<String, ReadDependency>,
) -> Result<BTreeMap<String, ReadDependency>> {
    Ok(current_dependencies_with_metrics(workspace, recorded)?.0)
}

pub(crate) fn current_dependencies_with_metrics(
    workspace: &Path,
    recorded: &BTreeMap<String, ReadDependency>,
) -> Result<(BTreeMap<String, ReadDependency>, ValidationMetrics)> {
    let started = Instant::now();
    let mut metrics = ValidationMetrics::default();
    let dependencies = recorded
        .iter()
        .map(|(relative, recorded)| {
            let path = relative_path(workspace, relative);
            if matches!(recorded, ReadDependency::Subtree { .. }) {
                return Ok((relative.clone(), subtree_dependency(&path)?));
            }
            let listed = matches!(recorded, ReadDependency::Directory { .. });
            let content = matches!(recorded, ReadDependency::File { .. });
            if content {
                match fs::symlink_metadata(&path) {
                    Ok(metadata) => {
                        if let ReadDependency::File {
                            fingerprint: Some(fingerprint),
                            ..
                        } = recorded
                        {
                            if file_fingerprint(&metadata) == *fingerprint {
                                return Ok((relative.clone(), recorded.clone()));
                            }
                        }
                        metrics.rehashed_files += 1;
                        metrics.rehashed_bytes += metadata.len();
                    }
                    Err(error) if path_is_missing(&error) => {}
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("reading recorded dependency {}", path.display())
                        })
                    }
                }
            }
            Ok((relative.clone(), dependency(&path, listed, content)?))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    crate::cix_timing!(
        "CIX timing read-validation files={} bytes={} wall_ms={}",
        metrics.rehashed_files,
        metrics.rehashed_bytes,
        started.elapsed().as_millis()
    );
    Ok((dependencies, metrics))
}

pub(crate) fn record_workspace_fingerprints(
    workspace: &Path,
    dependencies: &mut BTreeMap<String, ReadDependency>,
    writes: &BTreeSet<String>,
) -> Result<()> {
    for (relative, dependency) in dependencies {
        let ReadDependency::File { fingerprint, .. } = dependency else {
            continue;
        };
        if writes.contains(relative) {
            continue;
        }
        let path = relative_path(workspace, relative);
        *fingerprint = Some(file_fingerprint(
            &fs::symlink_metadata(&path)
                .with_context(|| format!("reading workspace fingerprint {}", path.display()))?,
        ));
    }
    Ok(())
}

/// Replace a fully observed stable directory tree with one recursive digest.
/// A missing directory listing or a content-less child observation makes the
/// candidate ineligible, so a narrower later read remains per-path.
pub(crate) fn aggregate_full_read_subtrees(
    snapshot: &Path,
    dependencies: &mut BTreeMap<String, ReadDependency>,
) -> Result<()> {
    let mut candidates = dependencies
        .iter()
        .filter_map(|(path, dependency)| {
            matches!(dependency, ReadDependency::Directory { .. }).then_some(path.clone())
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| path.matches('/').count());

    let mut selected = Vec::<String>::new();
    for path in candidates {
        if selected.iter().any(|root| same_or_descendant(&path, root)) {
            continue;
        }
        let Some(hash) = recorded_subtree_hash(snapshot, &path, dependencies)? else {
            continue;
        };
        selected.push(path.clone());
        dependencies.retain(|candidate, dependency| {
            !same_or_descendant(candidate, &path)
                || !matches!(
                    dependency,
                    ReadDependency::Directory { .. } | ReadDependency::File { .. }
                )
        });
        dependencies.insert(path, ReadDependency::Subtree { hash });
    }
    Ok(())
}

/// Replace a complete output tree with one replay root. A deleted tree needs
/// no digest: one absent root has exactly the same replay meaning.
pub(crate) fn aggregate_full_change_subtrees(
    before: &Path,
    after: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    let mut candidates = changes.keys().cloned().collect::<Vec<_>>();
    candidates.sort_by_key(|path| path.matches('/').count());

    let mut selected = Vec::<String>::new();
    for path in candidates {
        if selected.iter().any(|root| same_or_descendant(&path, root)) {
            continue;
        }
        let before_path = relative_path(before, &path);
        let after_path = relative_path(after, &path);
        let after_metadata = metadata_if_present(&after_path)?;
        if let Some(metadata) = after_metadata {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                continue;
            }
            if !complete_present_tree(&after_path, &path, changes)? {
                continue;
            }
            selected.push(path.clone());
            changes.retain(|candidate, _| !same_or_descendant(candidate, &path));
            changes.insert(
                path,
                StepChange::Subtree {
                    mode: metadata.permissions().mode(),
                },
            );
        } else {
            let before_metadata = metadata_if_present(&before_path)?;
            if !before_metadata
                .is_some_and(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
                || !complete_absent_tree(&before_path, &path, changes)?
            {
                continue;
            }
            selected.push(path.clone());
            changes.retain(|candidate, _| !same_or_descendant(candidate, &path));
            changes.insert(path, StepChange::Absent);
        }
    }
    Ok(())
}

pub(crate) fn filesystem_changes(
    before: &Path,
    after: &Path,
    written: &BTreeSet<String>,
) -> Result<BTreeMap<String, StepChange>> {
    let mut changes = BTreeMap::new();
    for relative in written {
        let old_path = relative_path(before, relative);
        let new_path = relative_path(after, relative);
        let old = metadata_if_present(&old_path)?;
        let new = metadata_if_present(&new_path)?;
        let change = match (old, new) {
            (None, Some(_)) => StepChange::Present,
            (Some(_), None) => StepChange::Absent,
            (Some(old), Some(new))
                if old.is_dir() && new.is_dir() && !new.file_type().is_symlink() =>
            {
                StepChange::Directory {
                    mode: new.permissions().mode(),
                }
            }
            (Some(_), Some(_)) => StepChange::Present,
            (None, None) => continue,
        };
        changes.insert(relative.clone(), change);
    }
    Ok(changes)
}

fn dependency(path: &Path, listed: bool, content: bool) -> Result<ReadDependency> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if path_is_missing(&error) => return Ok(ReadDependency::Absent),
        Err(error) => {
            return Err(error).with_context(|| format!("reading traced path {}", path.display()))
        }
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        return if listed {
            Ok(ReadDependency::Directory {
                hash: directory_hash(path)
                    .with_context(|| format!("hashing traced directory {}", path.display()))?,
            })
        } else {
            Ok(ReadDependency::DirectoryExists)
        };
    }
    if content {
        Ok(ReadDependency::File {
            hash: read_hash(path, &metadata)
                .with_context(|| format!("hashing traced file {}", path.display()))?,
            fingerprint: None,
        })
    } else {
        Ok(ReadDependency::FileExists)
    }
}

fn subtree_dependency(path: &Path) -> Result<ReadDependency> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if path_is_missing(&error) => return Ok(ReadDependency::Absent),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("reading recorded subtree {}", path.display()))
        }
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        return Ok(ReadDependency::Subtree {
            hash: filesystem_subtree_hash(path)?,
        });
    }
    dependency(path, false, true)
}

fn recorded_subtree_hash(
    snapshot: &Path,
    relative: &str,
    dependencies: &BTreeMap<String, ReadDependency>,
) -> Result<Option<String>> {
    let path = relative_path(snapshot, relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if path_is_missing(&error) => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| format!("reading subtree {}", path.display()))
        }
    };
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || !matches!(
            dependencies.get(relative),
            Some(ReadDependency::Directory { .. })
        )
    {
        return Ok(None);
    }
    recorded_directory_digest(&path, relative, dependencies)
}

fn recorded_directory_digest(
    directory: &Path,
    relative: &str,
    dependencies: &BTreeMap<String, ReadDependency>,
) -> Result<Option<String>> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("opening observed subtree {}", directory.display()))?
        .collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut digest = Sha256::new();
    for entry in entries {
        let name = entry.file_name();
        let child_relative = child_relative(relative, &name);
        let child_path = entry.path();
        let child_metadata = entry.file_type()?;
        let (kind, hash) = if child_metadata.is_dir() {
            if !matches!(
                dependencies.get(&child_relative),
                Some(ReadDependency::Directory { .. })
            ) {
                return Ok(None);
            }
            let Some(hash) = recorded_directory_digest(&child_path, &child_relative, dependencies)?
            else {
                return Ok(None);
            };
            (b"directory".as_slice(), hash)
        } else if child_metadata.is_file() || child_metadata.is_symlink() {
            let Some(ReadDependency::File { hash, .. }) = dependencies.get(&child_relative) else {
                return Ok(None);
            };
            (
                if child_metadata.is_symlink() {
                    b"symlink".as_slice()
                } else {
                    b"file".as_slice()
                },
                hash.clone(),
            )
        } else {
            return Ok(None);
        };
        digest.update(name.as_encoded_bytes());
        digest.update([0]);
        digest.update(kind);
        digest.update([0]);
        digest.update(hash.as_bytes());
        digest.update([0]);
    }
    Ok(Some(hex(digest.finalize())))
}

fn filesystem_subtree_hash(directory: &Path) -> Result<String> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("opening recorded subtree {}", directory.display()))?
        .collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut digest = Sha256::new();
    for entry in entries {
        let name = entry.file_name();
        let path = entry.path();
        let metadata = entry.file_type()?;
        let (kind, hash) = if metadata.is_dir() {
            (b"directory".as_slice(), filesystem_subtree_hash(&path)?)
        } else if metadata.is_file() || metadata.is_symlink() {
            (
                if metadata.is_symlink() {
                    b"symlink".as_slice()
                } else {
                    b"file".as_slice()
                },
                read_hash(&path, &fs::symlink_metadata(&path)?)?,
            )
        } else {
            anyhow::bail!(
                "unsupported special file in recorded subtree {}",
                path.display()
            );
        };
        digest.update(name.as_encoded_bytes());
        digest.update([0]);
        digest.update(kind);
        digest.update([0]);
        digest.update(hash.as_bytes());
        digest.update([0]);
    }
    Ok(hex(digest.finalize()))
}

fn complete_present_tree(
    directory: &Path,
    relative: &str,
    changes: &BTreeMap<String, StepChange>,
) -> Result<bool> {
    if !changes
        .get(relative)
        .is_some_and(|change| !matches!(change, StepChange::Absent))
    {
        return Ok(false);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let child_relative = child_relative(relative, &entry.file_name());
        let metadata = entry.file_type()?;
        if metadata.is_dir() && !metadata.is_symlink() {
            if !complete_present_tree(&entry.path(), &child_relative, changes)? {
                return Ok(false);
            }
        } else if !changes
            .get(&child_relative)
            .is_some_and(|change| !matches!(change, StepChange::Absent))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn complete_absent_tree(
    directory: &Path,
    relative: &str,
    changes: &BTreeMap<String, StepChange>,
) -> Result<bool> {
    if !matches!(changes.get(relative), Some(StepChange::Absent)) {
        return Ok(false);
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let child_relative = child_relative(relative, &entry.file_name());
        if entry.file_type()?.is_dir() && !entry.file_type()?.is_symlink() {
            if !complete_absent_tree(&entry.path(), &child_relative, changes)? {
                return Ok(false);
            }
        } else if !matches!(changes.get(&child_relative), Some(StepChange::Absent)) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn child_relative(parent: &str, name: &std::ffi::OsStr) -> String {
    let name = name.to_string_lossy();
    if parent == "." {
        name.into_owned()
    } else {
        format!("{parent}/{name}")
    }
}

fn same_or_descendant(candidate: &str, root: &str) -> bool {
    root == "."
        || candidate == root
        || candidate
            .strip_prefix(root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn path_is_missing(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

fn metadata_if_present(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if path_is_missing(&error) => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading traced path {}", path.display())),
    }
}

fn file_fingerprint(metadata: &fs::Metadata) -> FileFingerprint {
    FileFingerprint {
        dev: metadata.dev(),
        inode: metadata.ino(),
        mtime_ns: metadata.mtime_nsec(),
        size: metadata.size(),
        len: metadata.len(),
        mode: metadata.mode(),
    }
}

fn read_hash(path: &Path, metadata: &fs::Metadata) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(metadata.mode().to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(
            fs::read_link(path)
                .with_context(|| format!("reading symlink {}", path.display()))?
                .as_os_str()
                .as_encoded_bytes(),
        );
        if let Ok(target) = fs::metadata(path) {
            if target.is_file() {
                let mut file = fs::File::open(path)
                    .with_context(|| format!("opening traced file {}", path.display()))?;
                io::copy(&mut file, &mut digest)
                    .with_context(|| format!("reading traced file {}", path.display()))?;
            }
        }
    } else {
        let mut file = fs::File::open(path)
            .with_context(|| format!("opening traced file {}", path.display()))?;
        io::copy(&mut file, &mut digest)
            .with_context(|| format!("reading traced file {}", path.display()))?;
    }
    Ok(hex(digest.finalize()))
}

fn directory_hash(path: &Path) -> Result<String> {
    let mut entries = fs::read_dir(path)
        .with_context(|| format!("opening traced directory {}", path.display()))?
        .collect::<io::Result<Vec<_>>>()
        .with_context(|| format!("reading traced directory {}", path.display()))?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut digest = Sha256::new();
    for entry in entries {
        digest.update(entry.file_name().as_encoded_bytes());
        digest.update([0]);
        let entry_path = entry.path();
        let kind = entry
            .file_type()
            .with_context(|| format!("reading traced entry type {}", entry_path.display()))?;
        digest.update(if kind.is_dir() {
            b"directory".as_slice()
        } else if kind.is_symlink() {
            b"symlink".as_slice()
        } else if kind.is_file() {
            b"file".as_slice()
        } else {
            b"special".as_slice()
        });
        digest.update([0]);
    }
    Ok(hex(digest.finalize()))
}

fn split_pid(line: &str) -> Option<(u32, &str)> {
    let split = line.find(char::is_whitespace)?;
    Some((line[..split].parse().ok()?, line[split..].trim_start()))
}

fn return_value(call: &str) -> Option<&str> {
    call.rsplit_once(" = ")?.1.split_whitespace().next()
}

fn annotated_base(arguments: &str) -> Option<PathBuf> {
    let prefix = arguments.split('"').next()?;
    annotated_path(prefix)
}

fn annotated_fd_path(arguments: &str) -> Option<PathBuf> {
    annotated_path(arguments.split(',').next().unwrap_or(arguments))
}

fn annotated_path(text: &str) -> Option<PathBuf> {
    let start = text.find('<')? + 1;
    let end = text[start..].find('>')? + start;
    let path = text[start..end].split("->").next()?.trim();
    path.starts_with('/').then(|| PathBuf::from(path))
}

fn quoted_strings(text: &str) -> Vec<PathBuf> {
    let bytes = text.as_bytes();
    let mut values = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }
        index += 1;
        let mut value = Vec::new();
        while index < bytes.len() && bytes[index] != b'"' {
            if bytes[index] != b'\\' {
                value.push(bytes[index]);
                index += 1;
                continue;
            }
            index += 1;
            if index >= bytes.len() {
                break;
            }
            match bytes[index] {
                b'n' => value.push(b'\n'),
                b'r' => value.push(b'\r'),
                b't' => value.push(b'\t'),
                b'\\' => value.push(b'\\'),
                b'"' => value.push(b'"'),
                digit @ b'0'..=b'7' => {
                    let mut number = digit - b'0';
                    let mut count = 1;
                    while count < 3
                        && index + 1 < bytes.len()
                        && matches!(bytes[index + 1], b'0'..=b'7')
                    {
                        index += 1;
                        number = number * 8 + (bytes[index] - b'0');
                        count += 1;
                    }
                    value.push(number);
                }
                other => value.push(other),
            }
            index += 1;
        }
        values.push(PathBuf::from(std::ffi::OsString::from_vec(value)));
        index += usize::from(index < bytes.len());
    }
    values
}

fn resolve_path(path: &Path, base: Option<&Path>) -> Option<PathBuf> {
    let joined = if path.is_absolute() {
        path.to_owned()
    } else {
        base?.join(path)
    };
    Some(normalize(&joined))
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::RootDir => normalized.push("/"),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(name) => normalized.push(name),
            Component::Prefix(_) => {}
        }
    }
    normalized
}

fn work_relative(path: PathBuf) -> Option<String> {
    let relative = path.strip_prefix("/work").ok()?;
    Some(if relative.as_os_str().is_empty() {
        ".".into()
    } else {
        encode_relative(relative)
    })
}

fn relative_path(root: &Path, relative: &str) -> PathBuf {
    if relative == "." {
        root.to_owned()
    } else if let Some(encoded) = relative.strip_prefix("@cix-path:") {
        root.join(std::ffi::OsString::from_vec(decode_hex(encoded)))
    } else {
        root.join(relative)
    }
}

fn encode_relative(relative: &Path) -> String {
    const PREFIX: &str = "@cix-path:";
    match relative.to_str() {
        Some(relative) if !relative.starts_with(PREFIX) => relative.to_owned(),
        _ => format!("{PREFIX}{}", hex(relative.as_os_str().as_encoded_bytes())),
    }
}

fn decode_hex(encoded: &str) -> Vec<u8> {
    encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16).unwrap_or(0) as u8;
            let low = (pair[1] as char).to_digit(16).unwrap_or(0) as u8;
            high << 4 | low
        })
        .collect()
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_workspace_reads_readdirs_and_negative_lookups() {
        let trace = r#"100 chdir("/work/sub") = 0
100 newfstatat(AT_FDCWD</work/sub>, "missing", 0x1, 0) = -1 ENOENT (No such file or directory)
100 openat(AT_FDCWD</work/sub>, "input", O_RDONLY|O_CLOEXEC) = 3</work/sub/input>
100 openat(AT_FDCWD</work/sub>, "output", O_WRONLY|O_CREAT, 0666) = 4</work/sub/output>
100 getdents64(5</work/listed>, 0x1, 32768) = 48
100 newfstatat(AT_FDCWD</work/sub>, "/nix/store/example", 0x1, 0) = 0
100 openat(AT_FDCWD</work/sub>, "/run/cix-credentials/private", O_RDONLY|O_CLOEXEC) = 6</run/cix-credentials/private>
"#;
        assert_eq!(
            parse(trace).observations,
            BTreeMap::from([
                (
                    "listed".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    }
                ),
                ("sub".into(), Observation::default()),
                (
                    "sub/input".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    }
                ),
                (
                    "sub/missing".into(),
                    Observation {
                        negative: true,
                        ..Observation::default()
                    }
                ),
            ])
        );
    }

    #[test]
    fn records_a_path_beneath_a_file_as_absent() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), "payload").unwrap();
        let capture = Capture {
            observations: BTreeMap::from([("file/child".into(), Observation::default())]),
            writes: BTreeSet::new(),
        };

        assert_eq!(
            read_dependencies(root.path(), &capture).unwrap(),
            BTreeMap::from([("file/child".into(), ReadDependency::Absent)])
        );
    }

    #[test]
    fn failure_parse_keeps_only_ephemeral_exec_loader_and_soname_facts() {
        let trace = r#"100 chdir("/work/bin") = 0
100 execve("./dart", ["./dart"], 0x1) = -1 ENOENT (No such file or directory)
100 openat(AT_FDCWD</work/bin>, "/lib64/ld-linux-x86-64.so.2", O_RDONLY) = -1 ENOENT (No such file or directory)
101 execve("/work/native", ["/work/native"], 0x1) = 0
101 openat(AT_FDCWD</work>, "/nix/store/lib/libextra.so.1", O_RDONLY|O_CLOEXEC) = -1 ENOENT (No such file or directory)
101 openat(AT_FDCWD</work>, "/nix/store/lib/not-a-library", O_RDONLY) = -1 ENOENT (No such file or directory)
"#;
        assert_eq!(
            parse_failure(trace),
            FailureTrace {
                work_execs: vec!["bin/dart".into(), "native".into()],
                exec_enoent: BTreeSet::from(["bin/dart".into()]),
                missing_loaders: BTreeSet::from(["/lib64/ld-linux-x86-64.so.2".into()]),
                missing_sonames: BTreeSet::from(["libextra.so.1".into()]),
            }
        );
    }

    #[test]
    fn hashes_files_listings_and_absence_from_the_incoming_snapshot() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), "one").unwrap();
        fs::write(root.path().join("metadata-only"), "one").unwrap();
        let non_utf8 = PathBuf::from(std::ffi::OsString::from_vec(b"non-utf8-\xff".to_vec()));
        fs::write(root.path().join(&non_utf8), "bytes").unwrap();
        let non_utf8_key = encode_relative(&non_utf8);
        fs::create_dir(root.path().join("dir")).unwrap();
        fs::write(root.path().join("dir/entry"), "bytes").unwrap();
        let observations = BTreeMap::from([
            (
                "file".into(),
                Observation {
                    content: true,
                    ..Observation::default()
                },
            ),
            (
                "dir".into(),
                Observation {
                    listed: true,
                    ..Observation::default()
                },
            ),
            ("metadata-only".into(), Observation::default()),
            (
                "missing".into(),
                Observation {
                    negative: true,
                    ..Observation::default()
                },
            ),
            (
                non_utf8_key.clone(),
                Observation {
                    content: true,
                    ..Observation::default()
                },
            ),
        ]);
        let dependencies = read_dependencies(
            root.path(),
            &Capture {
                observations,
                writes: BTreeSet::new(),
            },
        )
        .unwrap();
        assert!(matches!(dependencies["file"], ReadDependency::File { .. }));
        assert_eq!(dependencies["metadata-only"], ReadDependency::FileExists);
        assert!(matches!(
            dependencies["dir"],
            ReadDependency::Directory { .. }
        ));
        assert_eq!(dependencies["missing"], ReadDependency::Absent);
        assert!(matches!(
            dependencies[&non_utf8_key],
            ReadDependency::File { .. }
        ));
        fs::write(root.path().join("metadata-only"), "two").unwrap();
        assert_eq!(
            current_dependencies(root.path(), &dependencies).unwrap(),
            dependencies
        );
    }

    #[test]
    fn unchanged_workspace_files_reuse_the_recorded_content_hash() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), "one").unwrap();
        let capture = Capture {
            observations: BTreeMap::from([(
                "file".into(),
                Observation {
                    content: true,
                    ..Observation::default()
                },
            )]),
            writes: BTreeSet::new(),
        };
        let mut dependencies = read_dependencies(root.path(), &capture).unwrap();
        record_workspace_fingerprints(root.path(), &mut dependencies, &capture.writes).unwrap();

        let (current, metrics) =
            current_dependencies_with_metrics(root.path(), &dependencies).unwrap();
        assert_eq!(current, dependencies);
        assert_eq!(metrics.rehashed_files, 0);
        assert_eq!(metrics.rehashed_bytes, 0);
    }

    #[test]
    fn file_fingerprints_are_nonsemantic_validation_hints() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file");
        fs::write(&path, "one").unwrap();
        let capture = Capture {
            observations: BTreeMap::from([(
                "file".into(),
                Observation {
                    content: true,
                    ..Observation::default()
                },
            )]),
            writes: BTreeSet::new(),
        };
        let mut dependencies = read_dependencies(root.path(), &capture).unwrap();
        record_workspace_fingerprints(root.path(), &mut dependencies, &capture.writes).unwrap();

        let replacement = root.path().join("replacement");
        fs::write(&replacement, "one").unwrap();
        fs::rename(&replacement, &path).unwrap();
        let (current, metrics) =
            current_dependencies_with_metrics(root.path(), &dependencies).unwrap();
        assert_eq!(current, dependencies);
        assert_eq!(metrics.rehashed_files, 1);
        assert_eq!(metrics.rehashed_bytes, 3);

        fs::write(&path, "two").unwrap();
        assert_ne!(
            current_dependencies(root.path(), &dependencies).unwrap(),
            dependencies
        );

        fs::remove_file(&path).unwrap();
        assert_eq!(
            current_dependencies(root.path(), &dependencies).unwrap()["file"],
            ReadDependency::Absent
        );
    }

    #[test]
    fn recorder_reuses_only_known_dependencies_with_sufficient_strength() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("content"), "same bytes").unwrap();
        fs::write(root.path().join("promoted"), "needs hashing").unwrap();
        fs::create_dir(root.path().join("listing")).unwrap();
        fs::write(root.path().join("listing/entry"), "entry").unwrap();
        let capture = Capture {
            observations: BTreeMap::from([
                (
                    "content".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    },
                ),
                (
                    "promoted".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    },
                ),
                (
                    "listing".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    },
                ),
                ("missing".into(), Observation::default()),
            ]),
            writes: BTreeSet::new(),
        };
        let expected = read_dependencies(root.path(), &capture).unwrap();
        let known = BTreeMap::from([
            ("content".into(), expected["content"].clone()),
            ("promoted".into(), ReadDependency::FileExists),
            ("listing".into(), expected["listing"].clone()),
            ("missing".into(), ReadDependency::Absent),
        ]);

        let (actual, metrics) =
            read_dependencies_with_known(root.path(), &capture, &known).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(metrics.reused, 3);
        assert_eq!(metrics.hashed_files, 1);
        assert_eq!(metrics.hashed_bytes, 13);
        assert_eq!(metrics.hashed_directories, 0);
    }

    #[test]
    fn records_precise_filesystem_delta_roots() {
        let before = tempfile::tempdir().unwrap();
        let after = tempfile::tempdir().unwrap();
        fs::create_dir(before.path().join("kept")).unwrap();
        fs::create_dir(after.path().join("kept")).unwrap();
        fs::write(before.path().join("kept/changed"), "old").unwrap();
        fs::write(after.path().join("kept/changed"), "new").unwrap();
        fs::write(before.path().join("removed"), "old").unwrap();
        fs::create_dir(after.path().join("added")).unwrap();
        fs::write(after.path().join("added/child"), "new").unwrap();
        let writes = BTreeSet::from(["added".into(), "kept/changed".into(), "removed".into()]);
        assert_eq!(
            filesystem_changes(before.path(), after.path(), &writes).unwrap(),
            BTreeMap::from([
                ("added".into(), StepChange::Present),
                ("kept/changed".into(), StepChange::Present),
                ("removed".into(), StepChange::Absent),
            ])
        );
    }

    #[test]
    fn aggregates_only_complete_stable_read_subtrees() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("vendor/nested")).unwrap();
        fs::write(root.path().join("vendor/first"), "one").unwrap();
        fs::write(root.path().join("vendor/nested/second"), "two").unwrap();
        let complete = Capture {
            observations: BTreeMap::from([
                (
                    "vendor".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    },
                ),
                (
                    "vendor/first".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    },
                ),
                (
                    "vendor/nested".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    },
                ),
                (
                    "vendor/nested/second".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    },
                ),
            ]),
            writes: BTreeSet::new(),
        };
        let mut dependencies = read_dependencies(root.path(), &complete).unwrap();
        aggregate_full_read_subtrees(root.path(), &mut dependencies).unwrap();
        assert!(matches!(
            dependencies.get("vendor"),
            Some(ReadDependency::Subtree { .. })
        ));
        assert_eq!(
            current_dependencies(root.path(), &dependencies).unwrap(),
            dependencies
        );

        fs::write(root.path().join("vendor/nested/second"), "drifted").unwrap();
        assert_ne!(
            current_dependencies(root.path(), &dependencies).unwrap(),
            dependencies
        );

        let mut partial = read_dependencies(root.path(), &complete).unwrap();
        partial.remove("vendor/nested/second");
        aggregate_full_read_subtrees(root.path(), &mut partial).unwrap();
        assert!(!matches!(
            partial.get("vendor"),
            Some(ReadDependency::Subtree { .. })
        ));
    }

    #[test]
    fn aggregates_a_complete_workspace_root_but_keeps_absent_reads() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("vendor")).unwrap();
        fs::write(root.path().join("vendor/first"), "one").unwrap();
        let capture = Capture {
            observations: BTreeMap::from([
                (
                    ".".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    },
                ),
                (
                    "vendor".into(),
                    Observation {
                        listed: true,
                        ..Observation::default()
                    },
                ),
                (
                    "vendor/first".into(),
                    Observation {
                        content: true,
                        ..Observation::default()
                    },
                ),
                ("created-later".into(), Observation::default()),
            ]),
            writes: BTreeSet::new(),
        };
        let mut dependencies = read_dependencies(root.path(), &capture).unwrap();
        aggregate_full_read_subtrees(root.path(), &mut dependencies).unwrap();
        assert!(matches!(
            dependencies.get("."),
            Some(ReadDependency::Subtree { .. })
        ));
        assert_eq!(
            dependencies.get("created-later"),
            Some(&ReadDependency::Absent)
        );
        assert_eq!(dependencies.len(), 2);
    }

    #[test]
    fn aggregates_complete_output_tree_and_full_removal() {
        let before = tempfile::tempdir().unwrap();
        let after = tempfile::tempdir().unwrap();
        fs::create_dir_all(after.path().join("vendor/nested")).unwrap();
        fs::write(after.path().join("vendor/first"), "one").unwrap();
        fs::write(after.path().join("vendor/nested/second"), "two").unwrap();
        let mut changes = BTreeMap::from([
            ("vendor".into(), StepChange::Present),
            ("vendor/first".into(), StepChange::Present),
            ("vendor/nested".into(), StepChange::Present),
            ("vendor/nested/second".into(), StepChange::Present),
        ]);
        aggregate_full_change_subtrees(before.path(), after.path(), &mut changes).unwrap();
        assert!(matches!(
            changes.get("vendor"),
            Some(StepChange::Subtree { .. })
        ));
        assert_eq!(changes.len(), 1);

        let removed_before = tempfile::tempdir().unwrap();
        let removed_after = tempfile::tempdir().unwrap();
        fs::create_dir_all(removed_before.path().join("vendor/nested")).unwrap();
        fs::write(removed_before.path().join("vendor/first"), "one").unwrap();
        fs::write(removed_before.path().join("vendor/nested/second"), "two").unwrap();
        let mut removals = BTreeMap::from([
            ("vendor".into(), StepChange::Absent),
            ("vendor/first".into(), StepChange::Absent),
            ("vendor/nested".into(), StepChange::Absent),
            ("vendor/nested/second".into(), StepChange::Absent),
        ]);
        aggregate_full_change_subtrees(removed_before.path(), removed_after.path(), &mut removals)
            .unwrap();
        assert_eq!(
            removals,
            BTreeMap::from([("vendor".into(), StepChange::Absent)])
        );
    }

    #[test]
    fn aggregates_a_complete_workspace_output_tree() {
        let before = tempfile::tempdir().unwrap();
        let after = tempfile::tempdir().unwrap();
        fs::create_dir(after.path().join("vendor")).unwrap();
        fs::write(after.path().join("vendor/first"), "one").unwrap();
        let root_mode = fs::metadata(after.path()).unwrap().permissions().mode();
        let mut changes = BTreeMap::from([
            (".".into(), StepChange::Directory { mode: 0o755 }),
            ("vendor".into(), StepChange::Present),
            ("vendor/first".into(), StepChange::Present),
        ]);
        aggregate_full_change_subtrees(before.path(), after.path(), &mut changes).unwrap();
        assert_eq!(
            changes,
            BTreeMap::from([(".".into(), StepChange::Subtree { mode: root_mode })])
        );
    }
}
