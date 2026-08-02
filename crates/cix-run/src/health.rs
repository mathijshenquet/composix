//! Native health property assembly.
//!
//! This module owns the readiness/liveness-to-systemd projection; the unit
//! conductor retains the surrounding property order.

use anyhow::{bail, Context, Result};

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
        .unwrap_or_else(std::env::current_exe)
        .context("resolving the cix binary for a native health probe")?;
    if !binary.is_absolute() {
        bail!("cix probe binary {} is not absolute", binary.display());
    }
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
