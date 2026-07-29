use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellSource {
    ServicePath,
    BinSh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shell {
    pub path: PathBuf,
    pub source: ShellSource,
}

pub(crate) fn resolve_shell(env: &BTreeMap<String, String>) -> Result<Shell> {
    resolve_shell_with_fallback(env, Path::new("/bin/sh"))
}

fn resolve_shell_with_fallback(env: &BTreeMap<String, String>, fallback: &Path) -> Result<Shell> {
    if let Some(path) = env.get("PATH") {
        for directory in std::env::split_paths(path) {
            let candidate = directory.join("sh");
            if is_executable(&candidate) {
                return Ok(Shell {
                    path: candidate,
                    source: ShellSource::ServicePath,
                });
            }
        }
    }
    if is_executable(fallback) {
        return Ok(Shell {
            path: fallback.to_owned(),
            source: ShellSource::BinSh,
        });
    }
    bail!("no `sh` found on the service PATH or at /bin/sh; pass an explicit command after `--`")
}

fn is_executable(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn executable(path: &Path) {
        fs::write(path, "#!/bin/sh\n").unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn shell_fallback_chain_prefers_service_path_then_bin_sh_then_errors() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        executable(&second.join("sh"));
        let fallback = temp.path().join("fallback-sh");
        executable(&fallback);

        let env = BTreeMap::from([(
            "PATH".into(),
            std::env::join_paths([&first, &second])
                .unwrap()
                .into_string()
                .unwrap(),
        )]);
        assert_eq!(
            resolve_shell_with_fallback(&env, &fallback).unwrap(),
            Shell {
                path: second.join("sh"),
                source: ShellSource::ServicePath,
            }
        );

        let empty = BTreeMap::new();
        assert_eq!(
            resolve_shell_with_fallback(&empty, &fallback).unwrap(),
            Shell {
                path: fallback.clone(),
                source: ShellSource::BinSh,
            }
        );
        fs::remove_file(&fallback).unwrap();
        let error = resolve_shell_with_fallback(&empty, &fallback).unwrap_err();
        assert!(error.to_string().contains("pass an explicit command"));
    }
}
