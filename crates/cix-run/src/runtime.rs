use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

use crate::app;
use crate::manager;
use crate::spec::{DataDir, ManifestKind, Service};
use crate::target::{resolve_service, InstallableResolver};
use crate::unit::{UnitCompileOptions, UnitNaming};

pub use crate::manager::{start_service, stop_service, StartedUnit};
pub struct RunOptions {
    pub installable: String,
    pub env: Vec<String>,
    pub port: Vec<String>,
    pub dirs: Vec<String>,
    pub identity: Option<String>,
    pub detach: bool,
    pub schedule: Option<String>,
    pub closed_root: bool,
    pub user: bool,
}

pub fn run(options: RunOptions, resolver: &dyn InstallableResolver) -> Result<()> {
    let mut target = resolve_service(resolver, &options.installable)?;
    if !options.user && manager::current_uid()? != 0 {
        bail!(
            "cix run targets the system manager and must run as root; use sudo, or pass --user for explicitly degraded dev mode"
        );
    }
    if options.user {
        if options.closed_root {
            eprintln!(
                "warning: --user is degraded development mode; CIP-84 keeps the sealed root for dev/prod parity, but the user manager may still reject individual namespace controls through the D13 fallback"
            );
        } else {
            eprintln!(
                "warning: --user is degraded development mode; the system manager with DynamicUser is the supported runtime target; filesystem mounts cannot be projected and CIX_APP names the real store path"
            );
        }
    }
    match target.kind {
        ManifestKind::Service => {
            if options.schedule.is_some() {
                bail!("cix run --schedule is only valid for manifest kind app");
            }
            let directory_options = materialize_run_directories(&mut target.service, &options)?;
            manager::run_resolved(
                target.output,
                &target.name,
                &target.service,
                &options,
                directory_options,
            )
        }
        ManifestKind::App => match options.schedule.as_deref() {
            Some(schedule) => app::schedule_app(target, &options, schedule),
            None => app::run_app(target, &options),
        },
    }
}

pub(crate) fn materialize_run_directories(
    service: &mut Service,
    options: &RunOptions,
) -> Result<UnitCompileOptions> {
    if options.dirs.is_empty() {
        return Ok(UnitCompileOptions::cix_run("service"));
    }
    let mut declarations = declared_directories(service);
    let mut extra_properties = Vec::new();
    let mut unit_properties = Vec::new();
    for argument in &options.dirs {
        let (selector, value) = argument.split_once('=').with_context(|| {
            format!("--dir {argument:?}: expected PATH=host:/path or PATH=as:role")
        })?;
        let path = select_run_directory(selector, &declarations)?;
        if value.starts_with("host-idmap:") {
            bail!(
                "--dir PATH=host-idmap:... was retired by CIP-81; write the same directory materialization in an anonymous compose JSON and run `cix run --compose <file|->`"
            );
        }
        let (_role, writable) = declarations
            .remove(&path)
            .expect("selected declaration exists");
        if let Some(host) = value.strip_prefix("host:") {
            let idmap = false;
            let host = PathBuf::from(host);
            if !host.is_absolute() {
                bail!("--dir {argument:?}: host backing path must be absolute");
            }
            if !options.user && options.identity.is_none() {
                bail!("--dir {argument:?}: host backing requires --identity for a static host identity (D48d)");
            }
            let metadata = fs::metadata(&host).with_context(|| {
                format!(
                    "--dir {argument:?}: host backing {} must pre-exist",
                    host.display()
                )
            })?;
            if !metadata.is_dir() {
                bail!(
                    "--dir {argument:?}: host backing {} must be a directory",
                    host.display()
                );
            }
            extra_properties.push((
                if writable {
                    "BindPaths"
                } else {
                    "BindReadOnlyPaths"
                }
                .into(),
                if idmap {
                    format!("{}:{}:idmap", host.display(), path.display())
                } else {
                    format!("{}:{}", host.display(), path.display())
                },
            ));
            unit_properties.push(("RequiresMountsFor".into(), host.display().to_string()));
            remove_run_directory(service, &path);
        } else if let Some(role) = value.strip_prefix("as:") {
            let role = parse_run_role(role)?;
            remove_run_directory(service, &path);
            insert_run_directory(service, role, path);
        } else if let Some(name) = value.strip_prefix("shared:") {
            if name.is_empty()
                || !name.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
            {
                bail!("--dir {argument:?}: shared name must be lowercase ASCII letters, digits, '.', '_', or '-'");
            }
            if !options.user {
                let (group, host) = prepare_run_shared_directory(name)?;
                extra_properties.push(("SupplementaryGroups".into(), group));
                extra_properties.push(("UMask".into(), "0002".into()));
                extra_properties.push((
                    "BindPaths".into(),
                    format!("{}:{}", host.display(), path.display()),
                ));
                unit_properties.push(("RequiresMountsFor".into(), host.display().to_string()));
                remove_run_directory(service, &path);
            } else {
                bail!("--dir {argument:?}: shared directories require the system manager; --user has no cix identity registry");
            }
        } else {
            bail!("--dir {argument:?}: expected host:/path, shared:name, or as:role");
        }
    }
    if let Some(identity) = &options.identity {
        extra_properties.extend([
            ("DynamicUser".into(), "no".into()),
            ("User".into(), identity.clone()),
            ("Group".into(), identity.clone()),
        ]);
    }
    Ok(UnitCompileOptions {
        naming: UnitNaming::cix_run("service"),
        extra_properties,
        unit_properties,
        log_fields: vec![("CIX_RUN".into(), "cix-run.service".into())],
        probe_binary: None,
        closed_root: None,
    })
}

fn prepare_run_shared_directory(name: &str) -> Result<(String, PathBuf)> {
    let group = format!("cix-rs-{:016x}", run_directory_hash(name));
    let temporary =
        tempfile::NamedTempFile::new().context("creating cix-run shared-group registry")?;
    fs::write(temporary.path(), format!("g {group} - -\n"))?;
    let output = Command::new("systemd-sysusers")
        .arg(temporary.path())
        .output()
        .context("applying cix-run shared-group registry")?;
    if !output.status.success() {
        bail!(
            "creating cix-run shared group {group}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let path = Path::new("/var/lib/cix-run/shared").join(name);
    fs::create_dir_all(&path)
        .with_context(|| format!("creating cix-run shared directory {}", path.display()))?;
    let output = Command::new("chown")
        .arg(format!("root:{group}"))
        .arg(&path)
        .output()
        .with_context(|| format!("setting cix-run shared directory group {}", path.display()))?;
    if !output.status.success() {
        bail!(
            "setting cix-run shared directory group: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o2770))?;
    Ok((group, path))
}

fn run_directory_hash(name: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in b"cix-run\0".iter().copied().chain(name.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn declared_directories(
    service: &Service,
) -> std::collections::BTreeMap<PathBuf, (Option<&'static str>, bool)> {
    let mut paths = std::collections::BTreeMap::new();
    for (role, values) in [
        (Some("state"), service.dirs.state.as_slice()),
        (Some("cache"), service.dirs.cache.as_slice()),
        (Some("logs"), service.dirs.logs.as_slice()),
        (Some("config"), service.dirs.config.as_slice()),
        (Some("run"), service.dirs.run.as_deref().unwrap_or_default()),
    ] {
        for path in values {
            paths.insert(path.clone(), (role, true));
        }
    }
    for DataDir { path, ro } in &service.dirs.data {
        paths.insert(path.clone(), (None, !ro));
    }
    paths
}

fn select_run_directory(
    selector: &str,
    declarations: &std::collections::BTreeMap<PathBuf, (Option<&'static str>, bool)>,
) -> Result<PathBuf> {
    if selector.starts_with('/') {
        let path = PathBuf::from(selector);
        if declarations.contains_key(&path) {
            return Ok(path);
        }
        bail!("--dir {selector}: path is not declared by the item");
    }
    let matching = declarations
        .iter()
        .filter(|(_, (role, _))| *role == Some(selector))
        .map(|(path, _)| path)
        .collect::<Vec<_>>();
    match matching.as_slice() {
        [path] => Ok((*path).clone()),
        [] => bail!("--dir {selector}: item has no declared {selector} directory"),
        _ => bail!("--dir {selector}: role names are only unambiguous for one declared path; use the absolute app path"),
    }
}

fn remove_run_directory(service: &mut Service, path: &Path) {
    service.dirs.state.retain(|candidate| candidate != path);
    service.dirs.cache.retain(|candidate| candidate != path);
    service.dirs.logs.retain(|candidate| candidate != path);
    service.dirs.config.retain(|candidate| candidate != path);
    if let Some(run) = &mut service.dirs.run {
        run.retain(|candidate| candidate != path);
    }
    service.dirs.data.retain(|candidate| candidate.path != path);
}

fn parse_run_role(role: &str) -> Result<&'static str> {
    match role {
        "state" => Ok("state"),
        "cache" => Ok("cache"),
        "logs" => Ok("logs"),
        "config" => Ok("config"),
        "run" => Ok("run"),
        _ => bail!("--dir as:{role}: expected state, cache, logs, config, or run"),
    }
}

fn insert_run_directory(service: &mut Service, role: &str, path: PathBuf) {
    match role {
        "state" => service.dirs.state.push(path),
        "cache" => service.dirs.cache.push(path),
        "logs" => service.dirs.logs.push(path),
        "config" => service.dirs.config.push(path),
        "run" => service.dirs.run.get_or_insert_with(Vec::new).push(path),
        _ => unreachable!("validated role"),
    }
}
