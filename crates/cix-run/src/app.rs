use std::path::Path;
use std::process::{self, ExitStatus};

use anyhow::{bail, Result};

use crate::config::ResolvedConfig;
use crate::degradation::{warn_degradations, without_properties, without_user_capability_controls};
use crate::manager::{
    build_runtime_unit, capability_failure, namespace_failure, nonce, run_transient_app,
    start_scheduled_app, with_unit_diagnostics,
};
use crate::runtime::RunOptions;
use crate::target::ResolvedService;
use crate::unit::{build_unit, UnitDefinition, UnitMode};

pub(crate) fn run_app(target: ResolvedService, options: &RunOptions) -> Result<()> {
    if options.detach {
        bail!("cix run --detach is not valid for manifest kind app; apps run to completion");
    }
    if !options.port.is_empty() {
        bail!("cix run -p/--port is not valid for manifest kind app (D47)");
    }
    let config = ResolvedConfig::resolve(&target.service, &options.env, &[])?;
    let mode = if options.user {
        UnitMode::UserFull
    } else {
        UnitMode::System
    };
    let definition = build_runtime_unit(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        options.closed_root,
        &format!("cix-run-{}-app.service", target.name),
    )?;
    warn_degradations(&definition.degradations);
    if !options.user {
        let name = format!("cix-run-{}-{}.service", target.name, nonce());
        let result = run_transient_app(&name, false, &target.output, &definition)?;
        if result.status.success() {
            return Ok(());
        }
        let error = with_unit_diagnostics(
            anyhow::anyhow!("app unit {name} failed: {}", result.stderr.trim()),
            &name,
            false,
        );
        if !namespace_failure(&error) {
            return finish_app(result.status);
        }
        eprintln!("warning: the system manager rejected PrivatePIDs isolation ({error:#})");
        eprintln!(
            "warning: retrying without PrivatePIDs; this app shares the host PID namespace (D36 degraded fallback)"
        );
        let fallback = without_properties(&definition, &["PrivatePIDs"]);
        return finish_app(
            run_transient_app(
                &format!("cix-run-{}-{}.service", target.name, nonce()),
                false,
                &target.output,
                &fallback,
            )?
            .status,
        );
    }

    let (status, error) = failed_app_attempt(&target.name, true, &target.output, &definition)?;
    if status.success() {
        return Ok(());
    }
    if capability_failure(&error) {
        eprintln!("warning: user manager rejected capability controls ({error:#})");
        eprintln!(
            "warning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, ProtectKernelLogs, and PrivateDevices"
        );
        let without_capabilities = without_user_capability_controls(&definition);
        let (retry_status, retry_error) =
            failed_app_attempt(&target.name, true, &target.output, &without_capabilities)?;
        if retry_status.success() {
            return Ok(());
        }
        if !namespace_failure(&retry_error) {
            return finish_app(retry_status);
        }
        return run_app_degraded(&target, &config, retry_error);
    }
    if namespace_failure(&error) {
        return run_app_degraded(&target, &config, error);
    }
    finish_app(status)
}

pub(crate) fn schedule_app(
    target: ResolvedService,
    options: &RunOptions,
    schedule: &str,
) -> Result<()> {
    if schedule.trim().is_empty() {
        bail!("cix run --schedule must not be empty");
    }
    if options.detach {
        bail!("cix run --detach is not valid with --schedule; the timer is already asynchronous");
    }
    if !options.port.is_empty() {
        bail!("cix run -p/--port is not valid for manifest kind app (D47)");
    }
    let config = ResolvedConfig::resolve(&target.service, &options.env, &[])?;
    let mode = if options.user {
        UnitMode::UserFull
    } else {
        UnitMode::System
    };
    let definition = build_runtime_unit(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        options.closed_root,
        &format!("cix-run-{}-scheduled.service", target.name),
    )?;
    warn_degradations(&definition.degradations);
    start_scheduled_app(
        &target.output,
        &target.name,
        options.user,
        schedule,
        &definition,
    )
}

fn failed_app_attempt(
    app_name: &str,
    user: bool,
    output: &Path,
    definition: &UnitDefinition,
) -> Result<(ExitStatus, anyhow::Error)> {
    let name = format!("cix-run-{app_name}-{}.service", nonce());
    let result = run_transient_app(&name, user, output, definition)?;
    let error = with_unit_diagnostics(
        anyhow::anyhow!("app unit {name} failed: {}", result.stderr.trim()),
        &name,
        user,
    );
    Ok((result.status, error))
}

fn run_app_degraded(
    target: &ResolvedService,
    config: &ResolvedConfig,
    error: anyhow::Error,
) -> Result<()> {
    eprintln!("warning: the user manager rejected mount-namespace sandboxing ({error:#})");
    eprintln!(
        "warning: retrying without PrivateUsers, PrivatePIDs, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; this is the D13 degraded development path"
    );
    let degraded = build_unit(
        &target.output,
        &target.name,
        &target.service,
        config,
        UnitMode::UserDegraded,
    )?;
    let name = format!("cix-run-{}-{}.service", target.name, nonce());
    finish_app(run_transient_app(&name, true, &target.output, &degraded)?.status)
}

fn finish_app(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    process::exit(status.code().unwrap_or(1));
}
