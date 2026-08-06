use std::collections::BTreeMap;
use std::env;
use std::io::Write;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};

pub const PRIVATE_PIDS_PROBE_OVERRIDE: &str = "CIX_PRIVATE_PIDS_PROBE";
pub const PRIVATE_DEVICES_PROBE_OVERRIDE: &str = "CIX_PRIVATE_DEVICES_PROBE";

// This process-local cache avoids repeat manager probes; the key includes the
// manager identity and version, so no capability result crosses managers.
static USER_VERIFY_CACHE: OnceLock<Mutex<BTreeMap<String, BTreeMap<String, String>>>> =
    OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Capability {
    Supported,
    Unsupported { reason: String },
}

impl Capability {
    pub fn unsupported_reason(&self) -> Option<&str> {
        match self {
            Self::Supported => None,
            Self::Unsupported { reason } => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCapabilities {
    pub private_pids_with_persistent_directories: Capability,
    pub user_private_devices: Capability,
    pub unsupported_directives: BTreeMap<String, String>,
    pub systemd_version: u32,
}

impl HostCapabilities {
    pub fn all_supported() -> Self {
        Self {
            private_pids_with_persistent_directories: Capability::Supported,
            user_private_devices: Capability::Supported,
            unsupported_directives: BTreeMap::new(),
            systemd_version: 257,
        }
    }

    pub fn private_pids_with_persistent_directories_unsupported(reason: impl Into<String>) -> Self {
        Self {
            private_pids_with_persistent_directories: Capability::Unsupported {
                reason: reason.into(),
            },
            user_private_devices: Capability::Supported,
            unsupported_directives: BTreeMap::new(),
            systemd_version: 257,
        }
    }

    pub fn user_private_devices_unsupported(reason: impl Into<String>) -> Self {
        Self {
            private_pids_with_persistent_directories: Capability::Supported,
            user_private_devices: Capability::Unsupported {
                reason: reason.into(),
            },
            unsupported_directives: BTreeMap::new(),
            systemd_version: 257,
        }
    }

    pub fn for_systemd_version(version: u32) -> Self {
        Self {
            systemd_version: version,
            ..Self::all_supported()
        }
    }

    pub fn probe() -> Result<Self> {
        if let Some(capabilities) =
            capabilities_from_override(env::var(PRIVATE_PIDS_PROBE_OVERRIDE).ok().as_deref())?
        {
            return with_systemd_version(capabilities);
        }

        let systemctl = Command::new("systemctl")
            .arg("--version")
            .output()
            .context("failed to run systemctl --version for the PrivatePIDs capability probe")?;
        if !systemctl.status.success() {
            bail!(
                "systemctl --version failed while probing PrivatePIDs support: {}",
                String::from_utf8_lossy(&systemctl.stderr).trim()
            );
        }
        let version_output = String::from_utf8(systemctl.stdout)
            .context("systemctl --version returned non-UTF-8 output")?;
        let version = parse_systemd_version(&version_output)?;
        if version < 257 {
            return Ok(Self {
                systemd_version: version,
                ..Self::private_pids_with_persistent_directories_unsupported(
                format!(
                    "systemd {version} predates PrivatePIDs= support; capability probe was not realized"
                ),
                )
            });
        }

        let mut capabilities = realize_private_pids_probe(version)?;
        capabilities.systemd_version = version;
        Ok(capabilities)
    }

    pub fn probe_user() -> Result<Self> {
        if let Some(capabilities) = user_capabilities_from_override(
            env::var(PRIVATE_DEVICES_PROBE_OVERRIDE).ok().as_deref(),
        )? {
            return with_systemd_version(capabilities);
        }

        realize_user_private_devices_probe()
    }

    pub fn probe_user_directives(properties: &[(String, String)]) -> Result<Self> {
        let mut capabilities = with_systemd_version(Self::probe_user()?)?;
        let key = format!(
            "uid:{}:systemd:{}",
            unsafe { libc::geteuid() },
            capabilities.systemd_version
        );
        let cache = USER_VERIFY_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
        let mut cache = cache
            .lock()
            .expect("user verify cache mutex is not poisoned");
        let rejected = match cache.get(&key) {
            Some(rejected) => rejected.clone(),
            None => {
                let rejected = verify_directives(properties)?;
                cache.insert(key, rejected.clone());
                rejected
            }
        };
        capabilities.unsupported_directives.extend(rejected);
        Ok(capabilities)
    }

    pub fn unsupported_directive_reason(&self, name: &str) -> Option<&str> {
        self.unsupported_directives.get(name).map(String::as_str)
    }
}

fn with_systemd_version(mut capabilities: HostCapabilities) -> Result<HostCapabilities> {
    let output = Command::new("systemctl")
        .arg("--version")
        .output()
        .context("failed to run systemctl --version")?;
    if !output.status.success() {
        bail!(
            "systemctl --version failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    capabilities.systemd_version = parse_systemd_version(
        &String::from_utf8(output.stdout)
            .context("systemctl --version returned non-UTF-8 output")?,
    )?;
    Ok(capabilities)
}

fn capabilities_from_override(value: Option<&str>) -> Result<Option<HostCapabilities>> {
    match value {
        None | Some("") | Some("auto") => Ok(None),
        Some("supported") => Ok(Some(HostCapabilities::all_supported())),
        Some("unsupported") => Ok(Some(
            HostCapabilities::private_pids_with_persistent_directories_unsupported(format!(
                "forced unsupported by {PRIVATE_PIDS_PROBE_OVERRIDE}"
            )),
        )),
        Some(value) => bail!(
            "{PRIVATE_PIDS_PROBE_OVERRIDE} must be auto, supported, or unsupported, not {value:?}"
        ),
    }
}

fn user_capabilities_from_override(value: Option<&str>) -> Result<Option<HostCapabilities>> {
    match value {
        None | Some("") | Some("auto") => Ok(None),
        Some("supported") => Ok(Some(HostCapabilities::all_supported())),
        Some("unsupported") => Ok(Some(HostCapabilities::user_private_devices_unsupported(
            format!("forced unsupported by {PRIVATE_DEVICES_PROBE_OVERRIDE}"),
        ))),
        Some(value) => bail!(
            "{PRIVATE_DEVICES_PROBE_OVERRIDE} must be auto, supported, or unsupported, not {value:?}"
        ),
    }
}

fn parse_systemd_version(output: &str) -> Result<u32> {
    let first_line = output
        .lines()
        .next()
        .context("systemctl --version was empty")?;
    let version = first_line
        .strip_prefix("systemd ")
        .and_then(|line| line.split_whitespace().next())
        .context("systemctl --version did not start with `systemd VERSION`")?;
    version
        .parse()
        .with_context(|| format!("invalid systemd version {version:?}"))
}

fn realize_private_pids_probe(version: u32) -> Result<HostCapabilities> {
    let shell = probe_shell("PrivatePIDs")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let unit = format!(
        "cix-private-pids-probe-{}-{nonce}.service",
        std::process::id()
    );
    let output = Command::new("systemd-run")
        .args([
            "--quiet",
            "--collect",
            "--wait",
            "--pipe",
            "--service-type=exec",
            "--unit",
            &unit,
            "--property",
            "DynamicUser=yes",
            "--property",
            "PrivatePIDs=yes",
            "--property",
            "StateDirectory=cix-private-pids-probe",
            "--",
            shell,
            "-c",
            "true",
        ])
        .output()
        .context("failed to realize the PrivatePIDs capability probe with systemd-run")?;

    if output.status.success() {
        return Ok(HostCapabilities::all_supported());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe_failure = format!("{stdout}\n{stderr}");
    if output.status.code() == Some(226)
        || probe_failure.contains("NAMESPACE")
        || probe_failure.contains("Failed to allocate user namespace")
        || probe_failure.contains("Failed to set up mount namespacing")
    {
        return Ok(
            HostCapabilities::private_pids_with_persistent_directories_unsupported(format!(
                "systemd {version} failed the DynamicUser=yes + PrivatePIDs=yes + StateDirectory= realization probe"
            )),
        );
    }

    bail!(
        "PrivatePIDs capability probe unit {unit} failed unexpectedly (status {}): {}",
        output.status,
        probe_failure.trim()
    )
}

fn realize_user_private_devices_probe() -> Result<HostCapabilities> {
    let shell = probe_shell("user-manager PrivateDevices")?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let unit = format!(
        "cix-private-devices-probe-{}-{nonce}.service",
        std::process::id()
    );
    let output = Command::new("systemd-run")
        .args([
            "--user",
            "--quiet",
            "--collect",
            "--wait",
            "--pipe",
            "--service-type=exec",
            "--unit",
            &unit,
            "--property",
            "PrivateDevices=yes",
            "--",
            shell,
            "-c",
            "true",
        ])
        .output()
        .context(
            "failed to realize the user-manager PrivateDevices capability probe with systemd-run",
        )?;

    if output.status.success() {
        return Ok(HostCapabilities::all_supported());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let probe_failure = format!("{stdout}\n{stderr}");
    let lowered = probe_failure.to_ascii_lowercase();
    if output.status.code() == Some(218)
        || lowered.contains("privatedevices")
        || lowered.contains("capabilities")
        || lowered.contains("operation not permitted")
        || lowered.contains("not supported")
    {
        return Ok(HostCapabilities::user_private_devices_unsupported(
            "user manager failed the PrivateDevices=yes realization probe".to_owned(),
        ));
    }

    bail!(
        "user-manager PrivateDevices capability probe unit {unit} failed unexpectedly (status {}): {}",
        output.status,
        probe_failure.trim()
    )
}

fn verify_directives(properties: &[(String, String)]) -> Result<BTreeMap<String, String>> {
    let mut unit = tempfile::Builder::new()
        .suffix(".service")
        .tempfile()
        .context("creating systemd-analyze verify capability probe unit")?;
    writeln!(unit, "[Service]")?;
    let offered = properties
        .iter()
        .cloned()
        .chain(
            ALL_USER_DIRECTIVES
                .iter()
                .map(|name| ((*name).into(), "yes".into())),
        )
        .collect::<BTreeMap<_, _>>();
    for (name, value) in offered {
        writeln!(unit, "{name}={value}")?;
    }
    unit.flush()?;
    let output = Command::new("systemd-analyze")
        .args(["--user", "verify"])
        .arg(unit.path())
        .output()
        .context("running systemd-analyze --user verify for capability probe")?;
    let diagnostics = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let rejected = rejected_directives(&diagnostics);
    if output.status.success() || !rejected.is_empty() {
        return Ok(rejected);
    }
    bail!(
        "systemd-analyze --user verify failed while probing generated directives: {}",
        diagnostics.trim()
    )
}

const ALL_USER_DIRECTIVES: &[&str] = &[
    "AmbientCapabilities",
    "BindPaths",
    "BindReadOnlyPaths",
    "CacheDirectory",
    "CacheDirectoryMode",
    "CapabilityBoundingSet",
    "ConfigurationDirectory",
    "ConfigurationDirectoryMode",
    "DeviceAllow",
    "DevicePolicy",
    "Environment",
    "ExecStart",
    "ExecStartPre",
    "KillSignal",
    "LockPersonality",
    "LogsDirectory",
    "LogsDirectoryMode",
    "MemoryDenyWriteExecute",
    "NoNewPrivileges",
    "PrivateDevices",
    "PrivateNetwork",
    "PrivatePIDs",
    "PrivateTmp",
    "PrivateUsers",
    "ProtectControlGroups",
    "ProtectHome",
    "ProtectKernelLogs",
    "ProtectKernelModules",
    "ProtectKernelTunables",
    "ProtectSystem",
    "RestrictAddressFamilies",
    "RestrictSUIDSGID",
    "RuntimeDirectory",
    "RuntimeDirectoryMode",
    "Slice",
    "SocketBindAllow",
    "SocketBindDeny",
    "Sockets",
    "StateDirectory",
    "StateDirectoryMode",
    "SystemCallFilter",
    "TemporaryFileSystem",
    "Type",
];

fn rejected_directives(diagnostics: &str) -> BTreeMap<String, String> {
    diagnostics
        .lines()
        .filter_map(|line| {
            let name = line
                .split_once("Unknown assignment: ")
                .and_then(|(_, assignment)| assignment.split_once('=').map(|(name, _)| name))
                .or_else(|| {
                    line.split_once("Unknown lvalue '")
                        .and_then(|(_, rest)| rest.split_once('\'').map(|(name, _)| name))
                })
                .or_else(|| {
                    line.split_once("Unknown key name '")
                        .and_then(|(_, rest)| rest.split_once('\'').map(|(name, _)| name))
                })?;
            // Quote only the analyzer's message: the line prefixes it with the
            // ephemeral probe unit path (/tmp/.tmpXXXXXX.service:NN:), which is
            // meaningless to users and made every degraded render differ (the
            // CI tour-determinism failure of 2026-08-05).
            let message = line.find("Unknown").map_or(line, |start| &line[start..]);
            Some((
                name.to_owned(),
                format!("user manager rejected {name}= (systemd-analyze --user verify: {message})"),
            ))
        })
        .collect()
}

fn probe_shell(probe: &str) -> Result<&'static str> {
    ["/bin/sh", "/run/current-system/sw/bin/sh"]
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .with_context(|| format!("could not find /bin/sh for the {probe} capability probe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_systemd_version() {
        assert_eq!(
            parse_systemd_version("systemd 261 (261.1)\n+PAM").unwrap(),
            261
        );
    }

    #[test]
    fn probe_override_injects_both_paths() {
        assert_eq!(
            capabilities_from_override(Some("supported")).unwrap(),
            Some(HostCapabilities::all_supported())
        );
        let unsupported = user_capabilities_from_override(Some("unsupported"))
            .unwrap()
            .unwrap();
        assert_eq!(
            unsupported.user_private_devices.unsupported_reason(),
            Some("forced unsupported by CIX_PRIVATE_DEVICES_PROBE")
        );
        let unsupported = capabilities_from_override(Some("unsupported"))
            .unwrap()
            .unwrap();
        assert_eq!(
            unsupported
                .private_pids_with_persistent_directories
                .unsupported_reason(),
            Some("forced unsupported by CIX_PRIVATE_PIDS_PROBE")
        );
    }

    #[test]
    fn probe_override_rejects_unknown_values() {
        assert!(capabilities_from_override(Some("maybe"))
            .unwrap_err()
            .to_string()
            .contains("must be auto, supported, or unsupported"));
        assert!(user_capabilities_from_override(Some("maybe"))
            .unwrap_err()
            .to_string()
            .contains("must be auto, supported, or unsupported"));
    }

    #[test]
    fn extracts_each_rejected_directive_from_verify_output() {
        let rejected = rejected_directives(
            "Unknown assignment: PrivatePIDs=yes\n/tmp/probe:4: Unknown key name 'ProtectHome' in section 'Service'\n",
        );
        assert_eq!(rejected.len(), 2);
        assert!(rejected.contains_key("PrivatePIDs"));
        assert!(rejected.contains_key("ProtectHome"));
    }
}
