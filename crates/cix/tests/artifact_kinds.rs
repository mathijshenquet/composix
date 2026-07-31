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
    let shell = if Path::new(&shell).starts_with("/nix/store") {
        shell
    } else {
        match store_shell() {
            Ok(shell) => shell,
            Err(error) => {
                eprintln!("skipping: requires a store-backed shell for the app fixture: {error:#}");
                return Ok(());
            }
        }
    };
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

fn store_shell() -> Result<String> {
    // Tests may use the ambient registry only to replace a host PATH shell with a store path.
    let output = Command::new("nix")
        .args(["build", "--print-out-paths", "--no-link", "nixpkgs#bash"])
        .output()
        .context("failed to obtain a store-backed shell")?;
    if !output.status.success() {
        bail!(
            "nix build nixpkgs#bash failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let output = String::from_utf8(output.stdout)?;
    output
        .lines()
        .map(|store_path| Path::new(store_path).join("bin/sh"))
        .find(|shell| shell.starts_with("/nix/store") && shell.is_file())
        .map(|shell| shell.to_string_lossy().into_owned())
        .context("nix build nixpkgs#bash did not provide /nix/store/.../bin/sh")
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
