use std::{
    collections::BTreeSet,
    fs,
    io::ErrorKind,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{bail, Context, Result};
use cix_run::capabilities::HostCapabilities;

use crate::{
    build_generation,
    generation::{Manifest, UnitKind},
    load_and_check, Compose, UpdateRequest,
};

const PROFILE_DIRECTORY: &str = "/nix/var/nix/profiles";
const GC_ROOT_DIRECTORY: &str = "/var/lib/cix-compose/gcroots";
const UNIT_DIRECTORY: &str = "/etc/systemd/system";

pub fn check(compose_path: &Path) -> Result<()> {
    let checked = load_and_check(compose_path, UpdateRequest::None)?;
    println!(
        "compose {}: {} services, {} edges, valid",
        checked.compose.name,
        checked.compose.services.len(),
        checked.compose.edges.len()
    );
    Ok(())
}

pub fn diff(compose_path: &Path) -> Result<()> {
    let checked = load_and_check(compose_path, UpdateRequest::None)?;
    let old = current_generation(&checked.compose.name)?;
    let old_manifest = old.as_deref().map(load_manifest).transpose()?;
    let capabilities = capabilities_for_diff(old_manifest.as_ref());
    let built = build_generation(&checked, compose_path, &capabilities)?;
    let report = compare_generations(old.as_deref(), &built.store_path)?;
    if report.is_empty() {
        println!("no changes");
    } else {
        for line in report {
            println!("{line}");
        }
    }
    Ok(())
}

pub fn up(compose_path: &Path, update: UpdateRequest) -> Result<()> {
    require_root("cix up")?;
    let checked = load_and_check(compose_path, update)?;
    let lock_path = Compose::lock_path(compose_path);
    checked.lock.write(&lock_path)?;
    let capabilities = HostCapabilities::probe()?;
    let built = build_generation(&checked, compose_path, &capabilities)?;
    warn_degradations(&built.manifest);
    let old = current_generation(&checked.compose.name)?;
    register_generation_gc_roots(&checked.compose.name, &built.store_path, &built.manifest)?;
    set_profile(&checked.compose.name, &built.store_path)?;
    activate_generation(&checked.compose.name, old.as_deref(), &built.store_path)?;
    println!(
        "activated {} from {}",
        checked.compose.name,
        built.store_path.display()
    );
    Ok(())
}

fn warn_degradations(manifest: &Manifest) {
    for degradation in &manifest.degradations {
        eprintln!(
            "warning: unit {}: dropped {}: {}; this service shares the host PID namespace (D36 degraded fallback)",
            degradation.unit, degradation.property, degradation.reason
        );
    }
}

fn capabilities_for_diff(manifest: Option<&Manifest>) -> HostCapabilities {
    manifest
        .and_then(|manifest| {
            manifest
                .degradations
                .iter()
                .find(|degradation| degradation.property == "PrivatePIDs=yes")
        })
        .map(|degradation| {
            HostCapabilities::private_pids_with_persistent_directories_unsupported(
                &degradation.reason,
            )
        })
        .unwrap_or_else(HostCapabilities::all_supported)
}

pub fn rollback(name: &str) -> Result<()> {
    require_root("cix rollback")?;
    validate_composite_name(name)?;
    let old = current_generation(name)?
        .with_context(|| format!("composite {name:?} has no profile to roll back"))?;
    nix_env(&["-p", &profile_text(name), "--rollback"])?;
    let new = current_generation(name)?
        .with_context(|| format!("rollback left composite {name:?} without a generation"))?;
    if old == new {
        bail!("composite {name:?} has no previous generation");
    }
    activate_generation(name, Some(&old), &new)?;
    println!("rolled back {name} to {}", new.display());
    Ok(())
}

pub fn down(name: &str) -> Result<()> {
    require_root("cix down")?;
    validate_composite_name(name)?;
    let generation =
        current_generation(name)?.with_context(|| format!("composite {name:?} has no profile"))?;
    let manifest = load_manifest(&generation)?;
    let target = format!("cix-{name}.target");
    let _ = systemctl(&["stop", &target]);
    let mut units = manifest.units.keys().cloned().collect::<Vec<_>>();
    units.sort_by_key(|unit| stop_order(unit));
    if !units.is_empty() {
        let mut arguments = vec!["stop"];
        arguments.extend(units.iter().map(String::as_str));
        systemctl(&arguments)?;
    }
    for unit in manifest.units.keys() {
        unlink_managed_unit(unit, Some(&generation))?;
    }
    systemctl(&["daemon-reload"])?;
    cleanup_edge_destinations(&generation)?;
    println!("stopped {name}; profile retained");
    Ok(())
}

fn activate_generation(name: &str, old: Option<&Path>, new: &Path) -> Result<()> {
    let new_manifest = load_manifest(new)?;
    if new_manifest.name != name {
        bail!(
            "generation {} belongs to composite {:?}, expected {name:?}",
            new.display(),
            new_manifest.name
        );
    }
    let old_manifest = old.map(load_manifest).transpose()?;
    let changes = generation_changes(old, old_manifest.as_ref(), new, &new_manifest)?;
    apply_sysusers(name, new)?;
    let target = format!("cix-{name}.target");
    let was_active = systemctl_is_active(&target)?;

    for unit in &changes.removed {
        let _ = systemctl(&["stop", unit]);
        unlink_managed_unit(unit, old)?;
    }
    if !changes.removed.is_empty() {
        if let Some(old) = old {
            cleanup_edge_destinations(old)?;
        }
    }
    for unit in new_manifest.units.keys() {
        link_managed_unit(unit, old, new)?;
    }
    systemctl(&["daemon-reload"])?;
    systemctl(&["start", &target])?;

    if was_active {
        let mut infrastructure = Vec::new();
        let mut services = Vec::new();
        for unit in &changes.changed {
            match new_manifest.units[unit].kind {
                UnitKind::Edge | UnitKind::Socket | UnitKind::Timer => {
                    infrastructure.push(unit.as_str())
                }
                UnitKind::Service if !new_manifest.units[unit].scheduled => {
                    services.push(unit.as_str())
                }
                UnitKind::Service => {}
                UnitKind::Slice | UnitKind::Target => {}
            }
        }
        if !infrastructure.is_empty() {
            let mut arguments = vec!["restart"];
            arguments.extend(infrastructure);
            systemctl(&arguments)?;
        }
        if !services.is_empty() {
            let mut arguments = vec!["restart"];
            arguments.extend(services);
            systemctl(&arguments)?;
        }
    }

    let statuses = new_manifest
        .units
        .iter()
        .filter(|(_, manifest)| manifest.kind == UnitKind::Service && !manifest.scheduled)
        .map(|(unit, _)| unit)
        .map(|unit| systemctl_is_failed(unit).map(|failed| (unit, failed)))
        .collect::<Result<Vec<_>>>()?;
    let failed = statuses
        .into_iter()
        .filter_map(|(unit, failed)| failed.then_some(unit))
        .collect::<Vec<_>>();
    if !failed.is_empty() {
        bail!(
            "cix up failed because these units failed during activation: {}",
            failed.into_iter().cloned().collect::<Vec<_>>().join(" ")
        );
    }
    Ok(())
}

struct GenerationChanges {
    removed: Vec<String>,
    changed: Vec<String>,
}

fn generation_changes(
    old_path: Option<&Path>,
    old: Option<&Manifest>,
    new_path: &Path,
    new: &Manifest,
) -> Result<GenerationChanges> {
    let old_units: BTreeSet<String> = old
        .map(|manifest| manifest.units.keys().cloned().collect())
        .unwrap_or_default();
    let new_units = new.units.keys().cloned().collect::<BTreeSet<_>>();
    let removed = old_units
        .difference(&new_units)
        .cloned()
        .collect::<Vec<_>>();
    let mut changed = Vec::new();
    if let Some(old_path) = old_path {
        for unit in old_units.intersection(&new_units) {
            if fs::read(old_path.join("units").join(unit))?
                != fs::read(new_path.join("units").join(unit))?
            {
                changed.push(unit.clone());
            }
        }
    }
    Ok(GenerationChanges { removed, changed })
}

fn compare_generations(old: Option<&Path>, new: &Path) -> Result<Vec<String>> {
    let new_manifest = load_manifest(new)?;
    let old_manifest = old.map(load_manifest).transpose()?;
    let old_units: BTreeSet<String> = old_manifest
        .as_ref()
        .map(|manifest| manifest.units.keys().cloned().collect())
        .unwrap_or_default();
    let new_units = new_manifest.units.keys().cloned().collect::<BTreeSet<_>>();
    let mut report = Vec::new();
    for unit in old_units.difference(&new_units) {
        report.push(format!("unit removed: {unit}"));
    }
    for unit in new_units.difference(&old_units) {
        report.push(format!("unit added: {unit}"));
    }
    if let Some(old_path) = old {
        for unit in old_units.intersection(&new_units) {
            if fs::read(old_path.join("units").join(unit))?
                != fs::read(new.join("units").join(unit))?
            {
                report.push(format!("unit changed: {unit}"));
            }
        }
    }

    let old_services = old_manifest.as_ref().map(|manifest| &manifest.services);
    let names = old_services
        .into_iter()
        .flat_map(|services| services.keys())
        .chain(new_manifest.services.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        let old_path = old_services
            .and_then(|services| services.get(&name))
            .map(|service| service.store_path.as_str())
            .unwrap_or("-");
        let new_path = new_manifest
            .services
            .get(&name)
            .map(|service| service.store_path.as_str())
            .unwrap_or("-");
        if old_path != new_path {
            report.push(format!("service {name}: {old_path} -> {new_path}"));
        }
        let old_shm = old_services
            .and_then(|services| services.get(&name))
            .and_then(|service| service.shm.as_deref());
        let new_shm = new_manifest
            .services
            .get(&name)
            .and_then(|service| service.shm.as_deref());
        if old_shm != new_shm {
            report.push(format!(
                "service {name}: shm {} -> {}",
                old_shm.unwrap_or("default"),
                new_shm.unwrap_or("default")
            ));
        }
    }
    Ok(report)
}

fn load_manifest(generation: &Path) -> Result<Manifest> {
    let path = generation.join("manifest.json");
    let contents = fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&contents).with_context(|| format!("parsing {}", path.display()))
}

fn profile_path(name: &str) -> PathBuf {
    Path::new(PROFILE_DIRECTORY).join(format!("cix-compose-{name}"))
}

fn profile_text(name: &str) -> String {
    profile_path(name).to_string_lossy().into_owned()
}

fn current_generation(name: &str) -> Result<Option<PathBuf>> {
    let profile = profile_path(name);
    match fs::canonicalize(&profile) {
        Ok(path) => Ok(Some(path)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("resolving profile {}", profile.display()))
        }
    }
}

fn set_profile(name: &str, generation: &Path) -> Result<()> {
    let generation = generation.to_string_lossy().into_owned();
    nix_env(&["-p", &profile_text(name), "--set", &generation])
}

fn register_generation_gc_roots(name: &str, generation: &Path, manifest: &Manifest) -> Result<()> {
    let generation_name = generation
        .file_name()
        .context("compose generation has no store-path name")?;
    let directory = Path::new(GC_ROOT_DIRECTORY)
        .join(name)
        .join(generation_name);
    fs::create_dir_all(&directory)
        .with_context(|| format!("creating compose GC-root directory {}", directory.display()))?;
    register_gc_root(&directory.join("generation.root"), generation)?;
    for (service, entry) in &manifest.services {
        register_gc_root(
            &directory.join(format!("{service}.root")),
            Path::new(&entry.store_path),
        )?;
    }
    Ok(())
}

fn register_gc_root(link: &Path, store_path: &Path) -> Result<()> {
    match fs::symlink_metadata(link) {
        Ok(_) => fs::remove_file(link)
            .with_context(|| format!("replacing compose GC root {}", link.display()))?,
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error)
                .with_context(|| format!("inspecting compose GC root {}", link.display()))
        }
    }
    let link = link
        .to_str()
        .context("compose GC-root path is not valid UTF-8")?;
    let store_path = store_path
        .to_str()
        .context("compose store path is not valid UTF-8")?;
    nix_store(&["--add-root", link, "--indirect", "--realise", store_path])
}

fn apply_sysusers(name: &str, generation: &Path) -> Result<()> {
    let fragment = generation
        .join("sysusers.d")
        .join(format!("cix-{name}.conf"));
    command("systemd-sysusers", &[fragment.to_string_lossy().as_ref()]).map(|_| ())
}

fn link_managed_unit(unit: &str, old: Option<&Path>, new: &Path) -> Result<()> {
    let destination = Path::new(UNIT_DIRECTORY).join(unit);
    if fs::symlink_metadata(&destination).is_ok() && !managed_link_matches(&destination, old)? {
        bail!(
            "refusing to replace unmanaged systemd unit {}",
            destination.display()
        );
    }
    let source = new.join("units").join(unit);
    let temporary = Path::new(UNIT_DIRECTORY).join(format!(".{unit}.cix-tmp"));
    match fs::remove_file(&temporary) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("removing stale temporary unit link"),
    }
    symlink(&source, &temporary)
        .with_context(|| format!("linking generated unit {}", source.display()))?;
    fs::rename(&temporary, &destination)
        .with_context(|| format!("installing generated unit {}", destination.display()))
}

fn unlink_managed_unit(unit: &str, generation: Option<&Path>) -> Result<()> {
    let path = Path::new(UNIT_DIRECTORY).join(unit);
    match fs::symlink_metadata(&path) {
        Ok(_) if managed_link_matches(&path, generation)? => fs::remove_file(&path)
            .with_context(|| format!("unlinking generated unit {}", path.display())),
        Ok(_) => bail!(
            "refusing to unlink unmanaged systemd unit {}",
            path.display()
        ),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("inspecting {}", path.display())),
    }
}

fn managed_link_matches(link: &Path, generation: Option<&Path>) -> Result<bool> {
    let metadata = fs::symlink_metadata(link)?;
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let target = fs::read_link(link)?;
    Ok(generation.is_some_and(|generation| target.starts_with(generation.join("units"))))
}

fn systemctl_is_active(unit: &str) -> Result<bool> {
    let output = Command::new("systemctl")
        .args(["is-active", "--quiet", unit])
        .output()
        .context("invoking systemctl is-active")?;
    Ok(output.status.success())
}

fn systemctl_is_failed(unit: &str) -> Result<bool> {
    let output = Command::new("systemctl")
        .args(["is-failed", "--quiet", unit])
        .output()
        .context("invoking systemctl is-failed")?;
    Ok(output.status.success())
}

fn systemctl(arguments: &[&str]) -> Result<()> {
    command("systemctl", arguments).map(|_| ())
}

fn nix_env(arguments: &[&str]) -> Result<()> {
    match command("nix-env", arguments) {
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == ErrorKind::NotFound) =>
        {
            command("/nix/var/nix/profiles/default/bin/nix-env", arguments).map(|_| ())
        }
        result => result.map(|_| ()),
    }
}

fn nix_store(arguments: &[&str]) -> Result<()> {
    match command("nix-store", arguments) {
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|io| io.kind() == ErrorKind::NotFound) =>
        {
            command("/nix/var/nix/profiles/default/bin/nix-store", arguments).map(|_| ())
        }
        result => result.map(|_| ()),
    }
}

fn command(program: &str, arguments: &[&str]) -> Result<Output> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .with_context(|| format!("invoking {program} {}", arguments.join(" ")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let message = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        bail!(
            "{program} {} failed ({}): {message}",
            arguments.join(" "),
            output.status
        );
    }
    Ok(output)
}

fn require_root(operation: &str) -> Result<()> {
    let output = command("id", &["-u"])?;
    if String::from_utf8_lossy(&output.stdout).trim() != "0" {
        bail!(
            "{operation} manages the system manager, /etc/systemd/system, and root profiles; run it as root (for example with sudo)"
        );
    }
    Ok(())
}

fn validate_composite_name(name: &str) -> Result<()> {
    if name.is_empty()
        || !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        bail!("invalid composite name {name:?}");
    }
    Ok(())
}

fn stop_order(unit: &str) -> u8 {
    if unit.ends_with(".target") {
        0
    } else if unit.ends_with(".socket") || unit.ends_with(".timer") {
        1
    } else if unit.contains("-edge-") {
        3
    } else if unit.ends_with(".slice") {
        4
    } else {
        2
    }
}

fn cleanup_edge_destinations(generation: &Path) -> Result<()> {
    let compose = Compose::load(&generation.join("compose.json"))?;
    let paths = compose
        .edges
        .values()
        .flat_map(|edge| {
            std::iter::once(edge.producer.path.as_path()).chain(
                edge.consumers
                    .values()
                    .filter_map(|consumer| consumer.path.as_deref()),
            )
        })
        .filter(|path| path.parent() == Some(Path::new("/run")))
        .collect::<BTreeSet<_>>();
    for path in paths {
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("removing edge mountpoint {}", path.display()))
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::generation::{ManifestDegradation, ManifestService, ManifestUnit};

    use super::*;

    fn generation(
        root: &Path,
        service_path: &str,
        service_text: &str,
        extra: Option<&str>,
    ) -> PathBuf {
        let path = root.join(service_path.trim_start_matches("/nix/store/"));
        fs::create_dir_all(path.join("units")).unwrap();
        let mut units = BTreeMap::from([(
            "cix-stack-web.service".into(),
            ManifestUnit {
                kind: UnitKind::Service,
                service: Some("web".into()),
                scheduled: false,
            },
        )]);
        fs::write(path.join("units/cix-stack-web.service"), service_text).unwrap();
        if let Some(name) = extra {
            units.insert(
                name.into(),
                ManifestUnit {
                    kind: UnitKind::Socket,
                    service: Some("web".into()),
                    scheduled: false,
                },
            );
            fs::write(path.join("units").join(name), "socket").unwrap();
        }
        let manifest = Manifest {
            name: "stack".into(),
            units,
            services: BTreeMap::from([(
                "web".into(),
                ManifestService {
                    item_service: "app".into(),
                    store_path: service_path.into(),
                    nar_hash: service_path.into(),
                    shm: None,
                },
            )]),
            degradations: Vec::new(),
        };
        fs::write(
            path.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        path
    }

    #[test]
    fn diff_reports_unit_and_store_item_changes() {
        let directory = tempfile::tempdir().unwrap();
        let old = generation(
            directory.path(),
            "/nix/store/old-web",
            "old service",
            Some("cix-stack-old.socket"),
        );
        let new = generation(
            directory.path(),
            "/nix/store/new-web",
            "new service",
            Some("cix-stack-new.socket"),
        );
        let report = compare_generations(Some(&old), &new).unwrap();
        assert_eq!(
            report,
            [
                "unit removed: cix-stack-old.socket",
                "unit added: cix-stack-new.socket",
                "unit changed: cix-stack-web.service",
                "service web: /nix/store/old-web -> /nix/store/new-web",
            ]
        );
    }

    #[test]
    fn diff_reuses_the_active_generation_capability_decision() {
        let directory = tempfile::tempdir().unwrap();
        let generation = generation(directory.path(), "/nix/store/old-web", "old service", None);
        let mut manifest = load_manifest(&generation).unwrap();
        manifest.degradations.push(ManifestDegradation {
            unit: "cix-stack-web.service".into(),
            property: "PrivatePIDs=yes".into(),
            reason: "synthetic realization failure".into(),
        });

        assert_eq!(
            capabilities_for_diff(Some(&manifest))
                .private_pids_with_persistent_directories
                .unsupported_reason(),
            Some("synthetic realization failure")
        );
        assert_eq!(
            capabilities_for_diff(None),
            HostCapabilities::all_supported()
        );
    }
}
