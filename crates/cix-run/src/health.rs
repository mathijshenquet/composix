//! Native health property assembly.
//!
//! This module owns the readiness/liveness-to-systemd projection; the unit
//! conductor retains the surrounding property order.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

use crate::spec::{format_duration, parse_duration, Probe, ProbeType, Service};
use crate::unit::{exec_command, UnitCompileOptions};

pub(crate) fn add_properties(
    properties: &mut Vec<(String, String)>,
    service: &Service,
    options: &UnitCompileOptions,
) -> Result<()> {
    if let Some(readiness) = &service.readiness {
        if readiness.probe.probe_type == ProbeType::Notify {
            properties
                .iter_mut()
                .find(|(name, _)| name == "Type")
                .expect("Type property exists")
                .1 = "notify".into();
        } else {
            properties.push((
                "ExecStartPost".into(),
                probe_command(options, "await", &readiness.probe, None)?,
            ));
        }
        properties.push(("TimeoutStartSec".into(), readiness.timeout.clone()));
        properties.push(("TimeoutStopSec".into(), readiness.timeout.clone()));
    }

    if service.liveness.is_some()
        && service
            .readiness
            .as_ref()
            .is_some_and(|readiness| readiness.probe.probe_type != ProbeType::Notify)
    {
        properties.push(("NotifyAccess".into(), "all".into()));
    }

    if let Some(liveness) = &service.liveness {
        let interval = parse_duration(&liveness.interval).context("invalid liveness interval")?;
        let watchdog = interval
            .checked_mul(3)
            .context("liveness watchdog window is too large")?;
        properties.push(("WatchdogSec".into(), format_duration(watchdog)));
        if liveness.probe.probe_type != ProbeType::Notify {
            if !properties.iter().any(|(name, _)| name == "NotifyAccess") {
                properties.push(("NotifyAccess".into(), "all".into()));
            }
            properties.push((
                "ExecStartPost".into(),
                probe_command(options, "pinger", &liveness.probe, Some(&liveness.interval))?,
            ));
        }
        properties.extend([
            ("Restart".into(), "on-failure".into()),
            ("RestartSec".into(), liveness.interval.clone()),
            ("StartLimitIntervalSec".into(), "5min".into()),
            ("StartLimitBurst".into(), "5".into()),
        ]);
    }
    Ok(())
}

fn probe_command(
    options: &UnitCompileOptions,
    mode: &str,
    probe: &Probe,
    interval: Option<&str>,
) -> Result<String> {
    let binary = options
        .probe_binary
        .clone()
        .map(Ok)
        .unwrap_or_else(runtime_probe_binary)?;
    validate_runtime_helper(&binary)?;
    let probe_type = match probe.probe_type {
        ProbeType::Http => "http",
        ProbeType::Tcp => "tcp",
        ProbeType::Notify => bail!("notify probes do not use the cix probe adapter"),
    };
    let target = probe.target.as_deref().expect("validated adapter target");
    let mut command = vec![
        binary.to_string_lossy().into_owned(),
        "probe".into(),
        mode.into(),
        probe_type.into(),
        target.into(),
    ];
    if let Some(interval) = interval {
        command.extend(["--every".into(), interval.into()]);
    }
    Ok(exec_command(&command))
}

fn runtime_probe_binary() -> Result<PathBuf> {
    if let Some(binary) = std::env::var_os("CIX_RUNTIME_HELPER") {
        return Ok(PathBuf::from(binary));
    }
    let binary = std::env::current_exe().context("resolving the cix runtime helper")?;
    if binary.starts_with("/nix/store") || !binary.starts_with(home_directory()) {
        return Ok(binary);
    }
    bail!(
        "native probes need an installed runtime helper outside the workspace; run the packaged cix or set CIX_RUNTIME_HELPER to its absolute store path (current executable is {})",
        binary.display()
    )
}

fn validate_runtime_helper(binary: &Path) -> Result<()> {
    if !binary.is_absolute() {
        bail!("cix probe binary {} is not absolute", binary.display());
    }
    if binary.starts_with(home_directory()) {
        bail!(
            "cix probe helper {} is workspace-local or home-local and unavailable under ProtectHome; use an installed store path",
            binary.display()
        );
    }
    Ok(())
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_probe_helpers_are_refused_before_unit_generation() {
        let workspace_binary = home_directory().join("project/target/debug/cix");
        let error = validate_runtime_helper(&workspace_binary)
            .unwrap_err()
            .to_string();
        assert!(error.contains("ProtectHome"), "{error}");
        validate_runtime_helper(Path::new("/nix/store/example-cix/bin/cix")).unwrap();
    }
}
