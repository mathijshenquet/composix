use std::fs;
use std::path::Path;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

const NIXPKGS_LOCK: &str = include_str!("../../../examples/pack/nginx/Cixfile.lock");

#[test]
fn update_lock_removes_all_build_scratch_after_success_and_failure() -> Result<()> {
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
START /bin/true
"#,
    )?;
    let output = cix(&success, &temp_root, &["build", "--update-lock", "build"])?;
    assert!(output.status.success(), "{}", command_failure(&output));
    assert_no_scratch(&temp_root)?;

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
START /bin/true
"#,
    )?;
    let output = cix(&failure, &temp_root, &["build", "--update-lock", "build"])?;
    assert!(
        !output.status.success(),
        "the failing FETCH unexpectedly succeeded"
    );
    assert_no_scratch(&temp_root)?;

    Ok(())
}

#[test]
fn sigterm_removes_live_build_scratch() -> Result<()> {
    let root = tempfile::tempdir()?;
    let temp_root = root.path().join("temp");
    fs::create_dir(&temp_root)?;
    let project = root.path().join("project");
    write_project(
        &project,
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
FETCH sleep 30
SERVICE result
COPY ${build}/. /result
START /bin/true
"#,
    )?;

    let mut child = cix_child(&project, &temp_root, &["build", "--update-lock", "build"])?;
    wait_for_scratch(&temp_root)?;
    let killed = unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    assert_eq!(killed, 0, "sending SIGTERM to cix failed");
    let status = child.wait().context("waiting for signalled cix")?;
    assert!(!status.success(), "SIGTERM build unexpectedly succeeded");
    assert_no_scratch(&temp_root)
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

fn cix_child(directory: &Path, temp_root: &Path, args: &[&str]) -> Result<Child> {
    Command::new(env!("CARGO_BIN_EXE_cix"))
        .args(args)
        .arg(directory)
        .env("TMPDIR", temp_root)
        .env("CIX_BUILD_WORKSPACE_DIR", directory.join("workspaces"))
        .env("XDG_CACHE_HOME", directory.join("cache"))
        .env("CIX_STATE_DIR", directory.join("state"))
        .spawn()
        .context("starting cix")
}

fn wait_for_scratch(temp_root: &Path) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if fs::read_dir(temp_root)?.next().is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    anyhow::bail!("cix did not create scratch under {}", temp_root.display())
}

fn assert_no_scratch(temp_root: &Path) -> Result<()> {
    const PREFIXES: &[&str] = &[
        "cix-build-cold-",
        "cix-build-view-",
        "cix-fetch-probe-",
        "cix-fetch-work-",
        "cix-import-loaders-",
        "cix-import-union-",
        "cix-read-trace-",
        "cix-step-delta-",
    ];
    let snapshots = fs::read_dir(temp_root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            PREFIXES
                .iter()
                .any(|prefix| name.to_string_lossy().starts_with(prefix))
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert!(
        snapshots.is_empty(),
        "cix build scratch remains in {}: {snapshots:?}",
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
