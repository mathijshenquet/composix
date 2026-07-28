use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use cix_run::config::ResolvedConfig;
use cix_run::runtime::{start_service, stop_service};
use cix_run::spec::Spec;

#[test]
fn user_run_persists_in_the_managed_state_directory() -> Result<()> {
    if !command_succeeds("systemctl", &["--user", "show-environment"])
        || !command_succeeds("nix", &["--version"])
    {
        eprintln!("skipping: requires Nix and a running systemd user manager");
        return Ok(());
    }

    let nonce = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let temporary = std::env::temp_dir().join(format!("cix-run-integration-{nonce}"));
    let fixture = temporary.join("fixture");
    let host_timestamp = user_state_root()?.join("cix-run-integration-test/timestamp");
    let app_state = host_timestamp.parent().unwrap().to_owned();
    fs::create_dir_all(fixture.join("bin"))?;

    let shell = find_program("sh")?;
    let date = find_program("date")?;
    let sleep = find_program("sleep")?;
    let script = format!(
        "#!{shell}\nset -eu\n{date} --iso-8601=seconds > {}/timestamp\nexec {sleep} 300\n",
        app_state.display()
    );
    let executable = fixture.join("bin/service");
    fs::write(&executable, script)?;
    make_executable(&executable)?;

    let json = serde_json::json!({
        "cixSpec": 1,
        "services": {
            "integration-test": {
                "exec": ["bin/service"],
                "dirs": {"state": [app_state]}
            }
        }
    });
    fs::write(
        fixture.join("cix-spec.json"),
        serde_json::to_vec_pretty(&json)?,
    )?;

    let store_path = add_to_store(&fixture)?;
    let spec = Spec::load(&store_path)?;
    let service = &spec.services["integration-test"];
    let config = ResolvedConfig::resolve(service, &[], &[])?;
    if host_timestamp.exists() {
        fs::remove_file(&host_timestamp)?;
    }

    let started = start_service(&store_path, "integration-test", service, &config, true)?;
    let guard = UnitGuard {
        name: started.name.clone(),
    };
    wait_for(&host_timestamp, Duration::from_secs(10))?;
    let active = Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", &started.name])
        .status()?;
    if !active.success() {
        bail!("{} did not remain active", started.name);
    }
    stop_service(&started.name, true)?;
    std::mem::forget(guard);
    fs::remove_dir_all(&temporary)?;
    Ok(())
}

struct UnitGuard {
    name: String,
}

impl Drop for UnitGuard {
    fn drop(&mut self) {
        let _ = stop_service(&self.name, true);
    }
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn find_program(name: &str) -> Result<String> {
    let output = Command::new("sh")
        .args(["-c", "command -v \"$1\"", "sh", name])
        .output()
        .with_context(|| format!("failed to find {name}"))?;
    if !output.status.success() {
        bail!("could not find {name}");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn add_to_store(path: &Path) -> Result<PathBuf> {
    let output = Command::new("nix")
        .args(["store", "add-path"])
        .arg(path)
        .output()
        .context("failed to invoke nix store add-path")?;
    if !output.status.success() {
        bail!(
            "nix store add-path failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(PathBuf::from(
        String::from_utf8(output.stdout)?.trim().to_owned(),
    ))
}

fn user_state_root() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("XDG_STATE_HOME") {
        return Ok(path.into());
    }
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".local/state"))
}

fn wait_for(path: &Path, timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if path.is_file() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("timed out waiting for {}", path.display())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}
