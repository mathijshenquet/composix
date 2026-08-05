use std::fs;
use std::path::Path;
use std::process::{Child, Command, Output};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

const NIXPKGS_LOCK: &str = include_str!("../../../examples/pack/nginx/Cixfile.lock");
const SCRATCH_PREFIXES: &[&str] = &[
    "cix-build-cold-",
    "cix-build-view-",
    "cix-fetch-probe-",
    "cix-fetch-work-",
    "cix-import-loaders-",
    "cix-import-union-",
    "cix-read-trace-",
    "cix-step-delta-",
];

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
    let scratch = wait_for_scratch(&temp_root)?;
    age_to_stale(&scratch)?;
    let concurrent = sweep_with_cix(&temp_root)?;
    assert!(
        !concurrent.status.success(),
        "the intentionally missing inspect target unexpectedly succeeded"
    );
    assert!(
        scratch.exists(),
        "a concurrent cix startup sweep removed live scratch {}",
        scratch.display()
    );
    let killed = unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
    assert_eq!(killed, 0, "sending SIGTERM to cix failed");
    let status = child.wait().context("waiting for signalled cix")?;
    assert!(!status.success(), "SIGTERM build unexpectedly succeeded");
    assert_no_scratch(&temp_root)
}

#[test]
fn startup_sweep_removes_unlocked_stale_scratch() -> Result<()> {
    let root = tempfile::tempdir()?;
    let temp_root = root.path().join("temp");
    fs::create_dir(&temp_root)?;
    let dead = temp_root.join("cix-build-cold-dead");
    fs::create_dir(&dead)?;
    let dead_lock = temp_root
        .join(".cix-scratch-locks")
        .join("cix-build-cold-dead");
    fs::create_dir_all(dead_lock.parent().context("finding dead lock parent")?)?;
    fs::write(&dead_lock, [])?;
    age_to_stale(&dead)?;

    let result = sweep_with_cix(&temp_root)?;
    assert!(
        !result.status.success(),
        "the intentionally missing inspect target unexpectedly succeeded"
    );
    assert!(
        !dead.exists(),
        "startup sweep retained unlocked stale scratch {}",
        dead.display()
    );
    assert!(
        !dead_lock.exists(),
        "startup sweep retained dead scratch owner lock {}",
        dead_lock.display()
    );
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

fn sweep_with_cix(temp_root: &Path) -> Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_cix"))
        .args(["inspect", "does-not-exist"])
        .env("TMPDIR", temp_root)
        .output()
        .context("invoking cix startup sweep")
}

fn wait_for_scratch(temp_root: &Path) -> Result<std::path::PathBuf> {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        for entry in fs::read_dir(temp_root)? {
            let entry = entry?;
            if SCRATCH_PREFIXES
                .iter()
                .any(|prefix| entry.file_name().to_string_lossy().starts_with(prefix))
            {
                return Ok(entry.path());
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    anyhow::bail!("cix did not create scratch under {}", temp_root.display())
}

fn age_to_stale(path: &Path) -> Result<()> {
    let seconds = 7 * 60 * 60;
    let timestamp = libc::timespec {
        tv_sec: std::time::SystemTime::now()
            .checked_sub(Duration::from_secs(seconds))
            .context("calculating stale scratch timestamp")?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as libc::time_t,
        tv_nsec: 0,
    };
    let path = std::ffi::CString::new(path.as_os_str().as_encoded_bytes())?;
    let result = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            path.as_ptr(),
            [timestamp, timestamp].as_ptr(),
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("aging scratch directory")
    }
}

fn assert_no_scratch(temp_root: &Path) -> Result<()> {
    let snapshots = fs::read_dir(temp_root)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            let name = entry.file_name();
            SCRATCH_PREFIXES
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
