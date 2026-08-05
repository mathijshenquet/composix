use std::process::{self, ExitStatus};

use anyhow::{anyhow, bail, Result};

use crate::capabilities::HostCapabilities;
use crate::config::ResolvedConfig;
use crate::degradation::{warn_degradations, without_properties, without_user_capability_controls};
use crate::manager::{
    capability_failure, current_uid, namespace_failure, nonce, run_transient_foreground,
    with_unit_diagnostics, ForegroundResult,
};
use crate::shell::{resolve_program, resolve_shell, ShellSource};
use crate::target::{resolve_service, ResolvedService};
use crate::unit::{build_unit_with_options, UnitCompileOptions, UnitDefinition, UnitMode};

pub struct DebugOptions {
    pub installable: String,
    pub env: Vec<String>,
    pub user: bool,
    pub command: Vec<String>,
    pub state_directory: std::path::PathBuf,
}

pub fn debug(options: DebugOptions) -> Result<()> {
    if !options.user && current_uid()? != 0 {
        bail!(
            "cix debug targets the system manager and must run as root; retry with sudo, or pass --user for explicitly degraded dev mode"
        );
    }
    if options.user {
        eprintln!(
            "warning: cix debug --user is degraded development mode; it does not provide the full system-manager sandbox or DynamicUser identity"
        );
    }

    let target = resolve_service(&options.state_directory, &options.installable)?;
    let mut config = ResolvedConfig::resolve_debug(&target.service, &options.env)?;
    config.env = config.item_environment(&target.output)?;
    if !target.service.listeners.is_empty() {
        eprintln!("note: declared listeners are not bound in cix debug; debug commands inherit no listener file descriptors");
    }

    let interactive = options.command.is_empty();
    let argv = if interactive {
        let shell = resolve_shell(&config.env)?;
        let source = match shell.source {
            ShellSource::ServicePath => "service PATH",
            ShellSource::OperatorPath => "operator fallback PATH",
        };
        eprintln!("cix debug: using shell {} ({source})", shell.path.display());
        vec![shell.path.to_string_lossy().into_owned()]
    } else {
        let mut command = options.command;
        command[0] = resolve_program(&command[0], &config.env)?
            .to_string_lossy()
            .into_owned();
        command
    };

    if options.user {
        eprintln!(
            "=== cix debug: degraded service sandbox; service={}; identity=caller (--user) ===",
            target.name
        );
    } else {
        eprintln!(
            "=== cix debug: full service sandbox; service={}; identity=service DynamicUser ===",
            target.name
        );
    }

    let mode = if options.user {
        UnitMode::UserFull
    } else {
        UnitMode::System
    };
    let baseline = debug_definition(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        argv.clone(),
        &HostCapabilities::all_supported(),
    )?;
    let capabilities = if options.user {
        HostCapabilities::probe_user_directives(&baseline.properties)?
    } else {
        HostCapabilities::all_supported()
    };
    let definition = debug_definition(
        &target.output,
        &target.name,
        &target.service,
        &config,
        mode,
        argv.clone(),
        &capabilities,
    )?;
    warn_degradations(&definition.degradations);
    if !options.user {
        let name = debug_name(&target.name);
        let result = run_attempt(name.clone(), false, &definition, interactive)?;
        if result.status.success()
            || !namespace_failure(&anyhow!(
                "debug unit {name} failed: {}",
                result.stderr.trim()
            ))
        {
            return finish(result.status);
        }
        eprintln!(
            "warning: the system manager rejected PrivatePIDs isolation (debug unit {name} failed: {})",
            result.stderr.trim()
        );
        eprintln!(
            "warning: retrying without PrivatePIDs; this debug sandbox shares the host PID namespace (D36 degraded fallback)"
        );
        let fallback = without_properties(&definition, &["PrivatePIDs"]);
        return finish(
            run_attempt(debug_name(&target.name), false, &fallback, interactive)?.status,
        );
    }

    let (status, error) = failed_attempt(debug_name(&target.name), true, &definition, interactive)?;
    if status.success() {
        return Ok(());
    }
    if capability_failure(&error) {
        eprintln!("warning: user manager rejected capability controls ({error:#})");
        eprintln!(
            "warning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, ProtectKernelLogs, and PrivateDevices"
        );
        let without_capabilities = without_user_capability_controls(&definition);
        let (retry_status, retry_error) = failed_attempt(
            debug_name(&target.name),
            true,
            &without_capabilities,
            interactive,
        )?;
        if retry_status.success() {
            return Ok(());
        }
        if !namespace_failure(&retry_error) {
            return finish(retry_status);
        }
        return run_degraded(&target, &config, argv, interactive, retry_error);
    }
    if namespace_failure(&error) {
        return run_degraded(&target, &config, argv, interactive, error);
    }
    finish(status)
}

fn run_degraded(
    target: &ResolvedService,
    config: &ResolvedConfig,
    argv: Vec<String>,
    interactive: bool,
    error: anyhow::Error,
) -> Result<()> {
    eprintln!("warning: the user manager rejected mount-namespace sandboxing ({error:#})");
    eprintln!(
        "warning: retrying without PrivateUsers, PrivatePIDs, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; this is the D13 degraded development path"
    );
    let degraded = debug_definition(
        &target.output,
        &target.name,
        &target.service,
        config,
        UnitMode::UserDegraded,
        argv,
        &HostCapabilities::all_supported(),
    )?;
    finish(run_attempt(debug_name(&target.name), true, &degraded, interactive)?.status)
}

fn debug_definition(
    output: &std::path::Path,
    service_name: &str,
    service: &crate::spec::Service,
    config: &ResolvedConfig,
    mode: UnitMode,
    argv: Vec<String>,
    capabilities: &HostCapabilities,
) -> Result<UnitDefinition> {
    let mut definition = build_unit_with_options(
        output,
        service_name,
        service,
        config,
        mode,
        &UnitCompileOptions::cix_run(service_name),
        capabilities,
    )?;
    definition.override_argv(argv);
    Ok(definition)
}

fn debug_name(service: &str) -> String {
    format!("cix-debug-{service}-{}.service", nonce())
}

fn run_attempt(
    name: String,
    user: bool,
    definition: &UnitDefinition,
    interactive: bool,
) -> Result<ForegroundResult> {
    run_transient_foreground(&name, user, definition, interactive)
}

fn failed_attempt(
    name: String,
    user: bool,
    definition: &UnitDefinition,
    interactive: bool,
) -> Result<(ExitStatus, anyhow::Error)> {
    let result = run_attempt(name.clone(), user, definition, interactive)?;
    let error = with_unit_diagnostics(
        anyhow!("debug unit {name} failed: {}", result.stderr.trim()),
        &name,
        user,
    );
    Ok((result.status, error))
}

fn finish(status: ExitStatus) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    process::exit(status.code().unwrap_or(1));
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::spec::Spec;
    use crate::unit::build_unit;

    use super::*;

    #[test]
    fn debug_overrides_only_the_generated_entrypoint() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["/nix/store/00000000000000000000000000000000-web/bin/server", "--serve"],
                "env": {"PATH": {"default": "/nix/store/11111111111111111111111111111111-shell/bin"}},
                "dirs": {"state": ["/var/lib/web"]}
            }"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let config = ResolvedConfig::resolve_debug(service, &[]).unwrap();
        let output = Path::new("/nix/store/00000000000000000000000000000000-web");
        let normal = build_unit(output, "web", service, &config, UnitMode::System).unwrap();
        let debug = debug_definition(
            output,
            "web",
            service,
            &config,
            UnitMode::System,
            vec!["/bin/sh".into(), "-c".into(), "id".into()],
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        assert_eq!(debug.properties, normal.properties);
        assert_eq!(debug.environment, normal.environment);
        assert_eq!(
            debug.argv,
            ["/bin/sh".to_owned(), "-c".to_owned(), "id".to_owned()]
        );
        let expected = normal.text.replace(
            "ExecStart=\"/nix/store/00000000000000000000000000000000-web/bin/server\" \"--serve\"",
            "ExecStart=\"/bin/sh\" \"-c\" \"id\"",
        );
        assert_eq!(debug.text, expected);
    }

    #[test]
    fn debug_projects_the_item_bin_default() {
        let spec = Spec::from_slice(
            br#"{
                "cixManifest": 0,
                "start": ["bin/app"],
                "env": {"PATH": {"default": "bin"}}
            }"#,
        )
        .unwrap();
        let service = spec.select_service(None).unwrap().1;
        let output = Path::new("/nix/store/00000000000000000000000000000000-app");
        let mut config = ResolvedConfig::resolve_debug(service, &[]).unwrap();
        config.env = config.item_environment(output).unwrap();
        let definition = debug_definition(
            output,
            "app",
            service,
            &config,
            UnitMode::System,
            vec![output.join("bin/app").to_string_lossy().into_owned()],
            &HostCapabilities::all_supported(),
        )
        .unwrap();

        assert!(definition.environment.contains(&(
            "PATH".into(),
            output.join("bin").to_string_lossy().into_owned(),
        )));
    }
}
