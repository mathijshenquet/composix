use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

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

pub(crate) fn read_dependencies(
    snapshot: &Path,
    capture: &Capture,
) -> Result<BTreeMap<String, ReadDependency>> {
    capture
        .observations
        .iter()
        .map(|(relative, observation)| {
            let path = relative_path(snapshot, relative);
            Ok((
                relative.clone(),
                dependency(&path, observation.listed, observation.content)?,
            ))
        })
        .collect()
}

pub(crate) fn current_dependencies(
    workspace: &Path,
    recorded: &BTreeMap<String, ReadDependency>,
) -> Result<BTreeMap<String, ReadDependency>> {
    recorded
        .iter()
        .map(|(relative, recorded)| {
            let path = relative_path(workspace, relative);
            let listed = matches!(recorded, ReadDependency::Directory { .. });
            let content = matches!(recorded, ReadDependency::File { .. });
            Ok((relative.clone(), dependency(&path, listed, content)?))
        })
        .collect()
}

pub(crate) fn filesystem_changes(
    before: &Path,
    after: &Path,
    written: &BTreeSet<String>,
) -> Result<BTreeMap<String, StepChange>> {
    let mut changes = BTreeMap::new();
    diff_directory(before, after, Path::new(""), &mut changes)?;
    let before_mode = fs::symlink_metadata(before)?.permissions().mode();
    let after_mode = fs::symlink_metadata(after)?.permissions().mode();
    if before_mode != after_mode {
        changes.insert(".".into(), StepChange::Directory { mode: after_mode });
    }
    for relative in written {
        let path = relative_path(after, relative);
        let change = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                StepChange::Directory {
                    mode: metadata.permissions().mode(),
                }
            }
            Ok(_) => StepChange::Present,
            Err(error) if error.kind() == io::ErrorKind::NotFound => StepChange::Absent,
            Err(error) => return Err(error.into()),
        };
        changes.entry(relative.clone()).or_insert(change);
    }
    Ok(changes)
}

fn diff_directory(
    before: &Path,
    after: &Path,
    relative: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    let before_directory = before.join(relative);
    let after_directory = after.join(relative);
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(&before_directory)? {
        names.insert(entry?.file_name());
    }
    for entry in fs::read_dir(&after_directory)? {
        names.insert(entry?.file_name());
    }
    for name in names {
        let child = relative.join(name);
        diff_node(before, after, &child, changes)?;
    }
    Ok(())
}

fn diff_node(
    before: &Path,
    after: &Path,
    relative: &Path,
    changes: &mut BTreeMap<String, StepChange>,
) -> Result<()> {
    let old = fs::symlink_metadata(before.join(relative)).ok();
    let new = fs::symlink_metadata(after.join(relative)).ok();
    let key = relative.to_string_lossy().into_owned();
    match (old, new) {
        (None, Some(_)) => {
            changes.insert(key, StepChange::Present);
        }
        (Some(_), None) => {
            changes.insert(key, StepChange::Absent);
        }
        (Some(old), Some(new)) if node_kind(&old) != node_kind(&new) => {
            changes.insert(key, StepChange::Present);
        }
        (Some(old), Some(new)) if old.is_dir() => {
            if old.permissions().mode() != new.permissions().mode() {
                changes.insert(
                    key,
                    StepChange::Directory {
                        mode: new.permissions().mode(),
                    },
                );
            }
            diff_directory(before, after, relative, changes)?;
        }
        (Some(old), Some(new)) => {
            let old_path = before.join(relative);
            let new_path = after.join(relative);
            if node_fingerprint(&old_path, &old)? != node_fingerprint(&new_path, &new)? {
                changes.insert(key, StepChange::Present);
            }
        }
        (None, None) => {}
    }
    Ok(())
}

fn dependency(path: &Path, listed: bool, content: bool) -> Result<ReadDependency> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(ReadDependency::Absent),
        Err(error) => {
            return Err(error).with_context(|| format!("reading traced path {}", path.display()))
        }
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        return if listed {
            Ok(ReadDependency::Directory {
                hash: directory_hash(path)?,
            })
        } else {
            Ok(ReadDependency::DirectoryExists)
        };
    }
    if content {
        Ok(ReadDependency::File {
            hash: read_hash(path, &metadata)?,
        })
    } else {
        Ok(ReadDependency::FileExists)
    }
}

fn read_hash(path: &Path, metadata: &fs::Metadata) -> Result<String> {
    let mut digest = Sha256::new();
    digest.update(metadata.mode().to_le_bytes());
    if metadata.file_type().is_symlink() {
        digest.update(b"symlink\0");
        digest.update(fs::read_link(path)?.as_os_str().as_encoded_bytes());
        if let Ok(target) = fs::metadata(path) {
            if target.is_file() {
                let mut file = fs::File::open(path)?;
                io::copy(&mut file, &mut digest)?;
            }
        }
    } else {
        let mut file = fs::File::open(path)?;
        io::copy(&mut file, &mut digest)?;
    }
    Ok(hex(digest.finalize()))
}

fn directory_hash(path: &Path) -> Result<String> {
    let mut entries = fs::read_dir(path)?.collect::<io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut digest = Sha256::new();
    for entry in entries {
        digest.update(entry.file_name().as_encoded_bytes());
        digest.update([0]);
        let kind = entry.file_type()?;
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

fn node_fingerprint(path: &Path, metadata: &fs::Metadata) -> Result<String> {
    read_hash(path, metadata)
}

fn node_kind(metadata: &fs::Metadata) -> u8 {
    if metadata.file_type().is_symlink() {
        0
    } else if metadata.is_dir() {
        1
    } else if metadata.is_file() {
        2
    } else {
        3
    }
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
        assert_eq!(
            filesystem_changes(before.path(), after.path(), &BTreeSet::new()).unwrap(),
            BTreeMap::from([
                ("added".into(), StepChange::Present),
                ("kept/changed".into(), StepChange::Present),
                ("removed".into(), StepChange::Absent),
            ])
        );
    }
}
