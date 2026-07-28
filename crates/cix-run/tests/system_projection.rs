use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context, Result};
use cix_run::config::ResolvedConfig;
use cix_run::runtime::{start_service, stop_service};
use cix_run::spec::Spec;

#[test]
fn system_projection_shadows_host_dirs_blocks_symlink_escape_and_handles_volume() -> Result<()> {
    if !is_root()
        || !command_succeeds("systemctl", &["show-environment"])
        || !Path::new("/etc/ssl/openssl.cnf").exists()
    {
        eprintln!("skipping: requires root, a system manager, and /etc/ssl/openssl.cnf");
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
    let temporary = std::env::temp_dir().join(format!("cix-system-projection-{nonce}"));
    let fixture = temporary.join("fixture");
    fs::create_dir_all(fixture.join("bin"))?;
    fs::create_dir_all(fixture.join("etc/ssl"))?;
    fs::create_dir_all(fixture.join("opt"))?;
    fs::write(fixture.join("etc/ssl/cix-fsproj-marker"), "projected\n")?;
    symlink("/etc/shadow", fixture.join("opt/cix-symlink"))?;

    let mut mounts = vec!["/etc/ssl".to_owned(), "/opt/cix-symlink".to_owned()];
    for index in 0..25 {
        let mount = format!("/cix-volume-{index}");
        fs::write(fixture.join(mount.trim_start_matches('/')), "visible\n")?;
        mounts.push(mount);
    }

    let shell = find_program("sh")?;
    let sleep = find_program("sleep")?;
    let volume_checks = (0..25)
        .map(|index| format!("test -f /cix-volume-{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let script = format!(
        "#!{shell}\nset -eu\ntest \"$(cat /etc/ssl/cix-fsproj-marker)\" = projected\ntest ! -e /etc/ssl/openssl.cnf\nif cat /opt/cix-symlink >/dev/null 2>&1; then exit 1; fi\n{volume_checks}\nexec {sleep} 300\n"
    );
    let executable = fixture.join("bin/service");
    fs::write(&executable, script)?;
    let mut permissions = fs::metadata(&executable)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions)?;

    let json = serde_json::json!({
        "cixSpec": 2,
        "services": {
            "projection-test": {
                "exec": ["bin/service"],
                "mounts": mounts,
            }
        }
    });
    fs::write(
        fixture.join("cix-spec.json"),
        serde_json::to_vec_pretty(&json)?,
    )?;

    let store_path = add_to_store(&fixture)?;
    let spec = Spec::load(&store_path)?;
    let service = &spec.services["projection-test"];
    let config = ResolvedConfig::resolve(service, &[], &[])?;
    let slice_guard = SystemSliceGuard;
    let started = start_service(&store_path, "projection-test", service, &config, false)?;
    let guard = UnitGuard {
        name: started.name.clone(),
    };

    thread::sleep(Duration::from_millis(300));
    if !command_succeeds("systemctl", &["is-active", "--quiet", &started.name]) {
        bail!("{} did not remain active", started.name);
    }
    let binds = Command::new("systemctl")
        .args([
            "show",
            &started.name,
            "--property=BindReadOnlyPaths",
            "--value",
        ])
        .output()?;
    let binds = String::from_utf8_lossy(&binds.stdout);
    for mount in &service.mounts.clone().unwrap_or_default() {
        assert!(
            binds.contains(&format!(":{}:rbind", mount.display())),
            "{binds}"
        );
    }

    stop_service(&started.name, false)?;
    std::mem::forget(guard);
    drop(slice_guard);
    fs::remove_dir_all(&temporary)?;
    Ok(())
}

struct UnitGuard {
    name: String,
}

impl Drop for UnitGuard {
    fn drop(&mut self) {
        let _ = stop_service(&self.name, false);
    }
}

struct SystemSliceGuard;

impl Drop for SystemSliceGuard {
    fn drop(&mut self) {
        let _ = Command::new("systemctl")
            .args(["stop", "cix-run.slice"])
            .status();
    }
}

fn is_root() -> bool {
    Command::new("id")
        .arg("-u")
        .output()
        .is_ok_and(|output| output.status.success() && output.stdout == b"0\n")
}

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
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
