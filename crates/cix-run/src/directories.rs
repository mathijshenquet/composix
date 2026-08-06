//! Managed-directory property assembly.
//!
//! This module owns directory materialization and its environment projection;
//! the unit conductor preserves their position in the generated unit.

use std::collections::BTreeSet;

use crate::spec::Dirs;

pub(crate) fn add_properties(
    properties: &mut Vec<(String, String)>,
    managed_base: &str,
    dirs: &Dirs,
    user: bool,
    bind: bool,
) {
    let mut additional_read_write_paths = BTreeSet::new();
    for (role, paths, directive, mode_directive, system_root, user_root) in [
        (
            "state",
            dirs.state.as_slice(),
            "StateDirectory",
            "StateDirectoryMode",
            "/var/lib",
            "%S",
        ),
        (
            "cache",
            dirs.cache.as_slice(),
            "CacheDirectory",
            "CacheDirectoryMode",
            "/var/cache",
            "%C",
        ),
        (
            "logs",
            dirs.logs.as_slice(),
            "LogsDirectory",
            "LogsDirectoryMode",
            "/var/log",
            "%L",
        ),
        (
            "config",
            dirs.config.as_slice(),
            "ConfigurationDirectory",
            "ConfigurationDirectoryMode",
            "/etc",
            "%E",
        ),
        (
            "run",
            dirs.run.as_deref().unwrap_or_default(),
            "RuntimeDirectory",
            "RuntimeDirectoryMode",
            "/run",
            "%t",
        ),
    ] {
        if paths.is_empty() {
            continue;
        }
        let mut directory_values = Vec::with_capacity(paths.len() + 1);
        let mut bind_values = Vec::new();
        // The managed root is an ownership anchor. Its id-mapped view must
        // exist before the explicit per-path binds project its subpaths.
        if bind && !user && role != "run" {
            directory_values.push(managed_base.to_owned());
        }
        for destination in paths {
            let mirror = destination
                .strip_prefix("/")
                .expect("validated absolute directory path")
                .to_string_lossy()
                .replace('%', "%%");
            let source = format!("{managed_base}/{mirror}");
            directory_values.push(source.clone());
            if bind {
                let root = if user { user_root } else { system_root };
                bind_values.push(format!(
                    "{root}/{source}:{}",
                    destination.to_string_lossy().replace('%', "%%")
                ));
            }
            if bind && !user && !destination.starts_with(system_root) {
                additional_read_write_paths
                    .insert(destination.to_string_lossy().replace('%', "%%"));
            }
        }
        if bind && !user && role != "run" {
            properties.push(("TemporaryFileSystem".into(), format!("{system_root}:ro")));
        }
        properties.push((directive.into(), directory_values.join(" ")));
        properties.push((mode_directive.into(), "0700".into()));
        for value in bind_values {
            properties.push(("BindPaths".into(), value));
        }
    }
    for path in additional_read_write_paths {
        properties.push(("ReadWritePaths".into(), path));
    }
}

pub(crate) fn add_environment(environment: &mut Vec<(String, String)>, dirs: &Dirs) {
    for (name, paths) in [
        ("STATE_DIRECTORY", dirs.state.as_slice()),
        ("CACHE_DIRECTORY", dirs.cache.as_slice()),
        ("LOGS_DIRECTORY", dirs.logs.as_slice()),
        ("CONFIGURATION_DIRECTORY", dirs.config.as_slice()),
        ("RUNTIME_DIRECTORY", dirs.run.as_deref().unwrap_or_default()),
    ] {
        if !paths.is_empty() {
            environment.push((
                name.into(),
                paths
                    .iter()
                    .map(|path| path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(":"),
            ));
        }
    }
}
