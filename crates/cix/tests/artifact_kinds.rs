use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{bail, Context, Result};

#[test]
fn app_propagates_exit_status() -> Result<()> {
    if !command_succeeds("systemctl", &["--user", "show-environment"])
        || !command_succeeds("nix", &["--version"])
    {
        eprintln!("skipping: requires Nix and a running systemd user manager");
        return Ok(());
    }

    let temporary = tempfile::tempdir()?;
    let app = temporary.path().join("app-fixture");
    fs::create_dir(&app)?;
    let shell = find_program("sh")?;
    write_manifest(
        &app,
        serde_json::json!({
            "cixManifest": 4,
            "kind": "app",
            "exec": [shell, "-c", "echo d47-app-output; exit 23"]
        }),
    )?;
    let app_store_path = add_to_store(&app)?;

    let result = cix(&["run", path_str(&app_store_path)?, "--user"])?;
    assert_eq!(result.status.code(), Some(23), "{result:?}");
    assert!(
        String::from_utf8_lossy(&result.stdout).contains("d47-app-output"),
        "app output was not streamed: {result:?}"
    );

    Ok(())
}

fn cix(args: &[&str]) -> Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cix"))
        .args(args)
        .output()
        .context("failed to invoke cix")
}

fn write_manifest(directory: &Path, value: serde_json::Value) -> Result<()> {
    fs::write(
        directory.join("cix-manifest.json"),
        serde_json::to_vec_pretty(&value)?,
    )?;
    Ok(())
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

fn command_succeeds(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str().context("store path was not UTF-8")
}
