use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};

const OPERATOR_PATH: [&str; 2] = ["/usr/bin", "/bin"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellSource {
    ServicePath,
    OperatorPath,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Shell {
    pub path: PathBuf,
    pub source: ShellSource,
}

pub(crate) fn resolve_shell(env: &BTreeMap<String, String>) -> Result<Shell> {
    let (path, source) = resolve_bare_program("sh", env, OPERATOR_PATH.iter().map(Path::new))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no `sh` found on the service's recorded/generated PATH or operator fallback /usr/bin:/bin; pass an explicit command after `--`"
            )
        })?;
    Ok(Shell {
        path,
        source: match source {
            ProgramSource::ServicePath => ShellSource::ServicePath,
            ProgramSource::OperatorPath => ShellSource::OperatorPath,
        },
    })
}

pub(crate) fn resolve_program(program: &str, env: &BTreeMap<String, String>) -> Result<PathBuf> {
    let path = Path::new(program);
    if program.contains('/') {
        if is_executable(path) {
            return Ok(path.to_owned());
        }
        bail!("command {program:?} is not an executable file");
    }

    resolve_bare_program(program, env, OPERATOR_PATH.iter().map(Path::new))
        .map(|(path, _)| path)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "command {program:?} was not found on the service's recorded/generated PATH or operator fallback /usr/bin:/bin"
            )
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProgramSource {
    ServicePath,
    OperatorPath,
}

fn resolve_bare_program<'a>(
    program: &str,
    env: &BTreeMap<String, String>,
    operator_path: impl IntoIterator<Item = &'a Path>,
) -> Option<(PathBuf, ProgramSource)> {
    let service_path = env
        .get("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(path));
    for directory in service_path {
        let candidate = directory.join(program);
        if is_executable(&candidate) {
            return Some((candidate, ProgramSource::ServicePath));
        }
    }
    for directory in operator_path {
        let candidate = directory.join(program);
        if is_executable(&candidate) {
            return Some((candidate, ProgramSource::OperatorPath));
        }
    }
    None
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
    fn effective_path_prefers_service_path_then_operator_path_then_errors() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        let operator = temp.path().join("operator");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::create_dir_all(&operator).unwrap();
        executable(&second.join("sh"));
        executable(&operator.join("sh"));

        let env = BTreeMap::from([(
            "PATH".into(),
            std::env::join_paths([&first, &second])
                .unwrap()
                .into_string()
                .unwrap(),
        )]);
        assert_eq!(
            resolve_bare_program("sh", &env, [operator.as_path()]),
            Some((second.join("sh"), ProgramSource::ServicePath))
        );

        let empty = BTreeMap::new();
        assert_eq!(
            resolve_bare_program("sh", &empty, [operator.as_path()]),
            Some((operator.join("sh"), ProgramSource::OperatorPath))
        );
        fs::remove_file(operator.join("sh")).unwrap();
        assert_eq!(
            resolve_bare_program("sh", &empty, [operator.as_path()]),
            None
        );
    }

    #[test]
    fn empty_environment_resolves_shell_and_explicit_command_from_operator_path() {
        let temp = tempfile::tempdir().unwrap();
        let operator = temp.path().join("operator");
        fs::create_dir_all(&operator).unwrap();
        executable(&operator.join("id"));

        assert_eq!(
            resolve_bare_program("id", &BTreeMap::new(), [operator.as_path()]),
            Some((operator.join("id"), ProgramSource::OperatorPath))
        );

        let shell = resolve_shell(&BTreeMap::new()).unwrap();
        assert_eq!(shell.source, ShellSource::OperatorPath);
        assert!(shell.path == Path::new("/usr/bin/sh") || shell.path == Path::new("/bin/sh"));

        let command = resolve_program("id", &BTreeMap::new()).unwrap();
        assert!(command == Path::new("/usr/bin/id") || command == Path::new("/bin/id"));
    }
}
