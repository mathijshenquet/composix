use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use anyhow::{Context, Result};

const NIXPKGS_LOCK: &str = include_str!("../../../examples/pack/nginx/Cixfile.lock");

#[test]
fn update_lock_fetch_probes_remove_readonly_snapshots_after_success_and_failure() -> Result<()> {
    let root = tempfile::tempdir()?;
    let temp_root = root.path().join("temp");
    fs::create_dir(&temp_root)?;

    let success = root.path().join("success");
    write_project(
        &success,
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
RUN mkdir readonly; chmod 0555 readonly
FETCH chmod 0755 readonly; printf payload > readonly/payload; chmod 0555 readonly
SERVICE result
COPY ${build}/readonly/payload /payload
EXEC /bin/true
"#,
    )?;
    let output = cix(&success, &temp_root, &["build", "--update-lock", "build"])?;
    assert!(output.status.success(), "{}", command_failure(&output));
    assert_no_probe_snapshots(&temp_root)?;

    let failure = root.path().join("failure");
    write_project(
        &failure,
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
RUN mkdir readonly; chmod 0555 readonly
FETCH false
SERVICE result
COPY ${build}/readonly /readonly
EXEC /bin/true
"#,
    )?;
    let output = cix(&failure, &temp_root, &["build", "--update-lock", "build"])?;
    assert!(
        !output.status.success(),
        "the failing FETCH unexpectedly succeeded"
    );
    assert_no_probe_snapshots(&temp_root)?;

    Ok(())
}

fn write_project(directory: &Path, cixfile: &str) -> Result<()> {
    fs::create_dir(directory)?;
    fs::write(directory.join("Cixfile"), cixfile)?;
    fs::write(directory.join("Cixfile.lock"), NIXPKGS_LOCK)?;
    Ok(())
}

fn cix(directory: &Path, temp_root: &Path, args: &[&str]) -> Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cix"))
        .args(args)
        .arg(directory)
        .env("TMPDIR", temp_root)
        .env("CIX_BUILD_WORKSPACE_DIR", directory.join("workspaces"))
        .env("XDG_CACHE_HOME", directory.join("cache"))
        .env("CIX_STATE_DIR", directory.join("state"))
        .output()
        .context("invoking cix")
}

fn assert_no_probe_snapshots(temp_root: &Path) -> Result<()> {
    let snapshots = fs::read_dir(temp_root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("cix-fetch-probe-")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert!(
        snapshots.is_empty(),
        "FETCH probe snapshots remain in {}: {snapshots:?}",
        temp_root.display()
    );
    Ok(())
}

fn command_failure(output: &Output) -> String {
    format!(
        "cix exited with {}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    )
}
