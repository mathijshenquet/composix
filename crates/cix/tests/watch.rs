use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

#[test]
fn scripted_edit_rebuilds_once_and_ignores_its_own_outputs() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::write(
        root.join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\n\nSERVICE app\n  COPY ${src}/start /bin/start\n  START /bin/start\n",
    )
    .unwrap();
    fs::write(
        root.join("Cixfile.lock"),
        include_str!("../../../examples/pack/nginx/Cixfile.lock"),
    )
    .unwrap();
    fs::write(root.join("start"), "#!/bin/sh\necho first\n").unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_cix"))
        .arg("watch")
        .arg(root)
        .env("CIX_WATCH_DEBOUNCE_MS", "30")
        .env("CIX_BUILD_WORKSPACE_DIR", root.join("workspaces"))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let (lines_sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let _ = lines_sender.send(line.unwrap());
        }
    });
    let stderr = child.stderr.take().unwrap();
    let (errors_sender, errors) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = errors_sender.send(line.unwrap());
        }
    });

    assert_eq!(
        errors.recv_timeout(Duration::from_secs(5)).unwrap(),
        format!("watching {}", root.display())
    );
    std::thread::sleep(Duration::from_millis(100));
    fs::write(root.join("start"), "#!/bin/sh\necho changed\n").unwrap();
    let first = match lines.recv_timeout(Duration::from_secs(60)) {
        Ok(line) => line,
        Err(error) => {
            unsafe { libc::kill(child.id() as i32, libc::SIGTERM) };
            let _ = child.wait();
            panic!(
                "edited Cixfile context should rebuild ({error}); watcher stderr: {:?}",
                errors.try_iter().collect::<Vec<_>>()
            );
        }
    };
    assert!(first.starts_with("/nix/store/"), "{first}");

    fs::write(root.join("Cixfile.lock"), "{}\n").unwrap();
    fs::create_dir_all(root.join("workspaces/build/work")).unwrap();
    fs::write(root.join("workspaces/build/work/state"), "generated\n").unwrap();
    assert!(lines.recv_timeout(Duration::from_millis(900)).is_err());
    assert!(errors.try_iter().next().is_none());

    unsafe { libc::kill(child.id() as i32, libc::SIGINT) };
    assert!(child.wait().unwrap().success());
}
