use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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
        "cixManifest": 0,
        "exec": ["bin/service"],
        "mounts": mounts,
    });
    fs::write(
        fixture.join("cix-manifest.json"),
        serde_json::to_vec_pretty(&json)?,
    )?;

    let store_path = add_to_store(&fixture)?;
    let spec = Spec::load(&store_path)?;
    let service = spec.select_service(None)?.1;
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
    let gc_root = Path::new("/run/cix/gcroots").join(format!("{}.root", started.name));
    assert_eq!(fs::read_link(&gc_root)?, store_path);
    assert!(
        auto_root_exists(&gc_root)?,
        "missing auto root for {}",
        gc_root.display()
    );
    let unit = unit_file(&started.name)?;
    assert!(unit.contains("ExecStopPost=+"), "{unit}");
    assert!(unit.contains(gc_root.to_string_lossy().as_ref()), "{unit}");
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
    assert!(
        fs::symlink_metadata(&gc_root)
            .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "GC root {} remained after stop",
        gc_root.display()
    );
    drop(slice_guard);
    fs::remove_dir_all(&temporary)?;
    Ok(())
}

#[test]
fn system_v3_listeners_inherit_fds_and_socket_bind_rules_are_kernel_enforced() -> Result<()> {
    if !is_root() || !command_succeeds("systemctl", &["show-environment"]) {
        eprintln!("skipping: requires root and a system manager");
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
    let temporary = std::env::temp_dir().join(format!("cix-manifestv3-{nonce}"));
    let python = find_program("python3")?;
    let listen_port = free_port()?;
    let declared_port = free_port()?;
    let denied_port = free_port()?;

    let listener_fixture = temporary.join("listener");
    fs::create_dir_all(listener_fixture.join("bin"))?;
    let listener_script = format!(
        "#!{python}\nimport os\nimport socket\n\nif os.environ.get('LISTEN_FDS') != '1' or os.environ.get('LISTEN_FDNAMES') != 'http':\n    raise SystemExit('missing named listener')\nlistener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)\nwhile True:\n    connection, _ = listener.accept()\n    with connection:\n        connection.recv(4096)\n        connection.sendall(b'HTTP/1.1 200 OK\\r\\nContent-Length: 3\\r\\nConnection: close\\r\\n\\r\\nok\\n')\n"
    );
    let listener_executable = listener_fixture.join("bin/service");
    fs::write(&listener_executable, listener_script)?;
    make_executable(&listener_executable)?;
    fs::write(
        listener_fixture.join("cix-manifest.json"),
        br#"{
            "cixManifest": 0,
            "exec": ["bin/service"],
            "listeners": {"http": {"type": "stream"}}
        }"#,
    )?;
    let listener_store = add_to_store(&listener_fixture)?;
    let listener_spec = Spec::load(&listener_store)?;
    let listener_service = listener_spec.select_service(None)?.1;
    let listener_config = ResolvedConfig::resolve(
        listener_service,
        &[],
        &[format!("http=127.0.0.1:{listen_port}")],
    )?;
    let slice_guard = SystemSliceGuard;
    let listener_started = start_service(
        &listener_store,
        "listener-test",
        listener_service,
        &listener_config,
        false,
    )?;
    let listener_guard = UnitGuard {
        name: listener_started.name.clone(),
    };
    wait_for_http(listen_port)?;
    let socket = format!(
        "{}-http.socket",
        listener_started.name.trim_end_matches(".service")
    );
    assert_property(&listener_started.name, "PrivateNetwork", "yes")?;
    assert_property(&listener_started.name, "PrivatePIDs", "yes")?;
    assert_property(&listener_started.name, "RestrictAddressFamilies", "AF_UNIX")?;
    assert_property(&listener_started.name, "CapabilityBoundingSet", "")?;
    assert_property(&listener_started.name, "SocketBindDeny", "any")?;
    assert_property(&socket, "ActiveState", "active")?;
    let socket_text = unit_file(&socket)?;
    assert!(socket_text.contains(&format!("ListenStream=127.0.0.1:{listen_port}")));
    assert!(socket_text.contains("FileDescriptorName=http"));
    assert!(socket_text.contains(&format!("Service={}", listener_started.name)));
    let service_text = unit_file(&listener_started.name)?;
    for property in ["Requires", "After", "Sockets"] {
        assert!(
            service_text.contains(&format!("{property}={socket}")),
            "{service_text}"
        );
    }
    stop_service(&listener_started.name, false)?;
    std::mem::forget(listener_guard);
    assert_eq!(systemctl_property(&socket, "LoadState")?, "not-found");

    let ports_fixture = temporary.join("ports");
    fs::create_dir_all(ports_fixture.join("bin"))?;
    let ports_script = format!(
        "#!{python}\nimport socket\nimport time\n\nallowed = socket.socket(socket.AF_INET, socket.SOCK_STREAM)\nallowed.bind(('127.0.0.1', {declared_port}))\ndenied = socket.socket(socket.AF_INET, socket.SOCK_STREAM)\ntry:\n    denied.bind(('127.0.0.1', {denied_port}))\nexcept PermissionError:\n    pass\nelse:\n    raise SystemExit('undeclared bind was not denied')\ntime.sleep(300)\n"
    );
    let ports_executable = ports_fixture.join("bin/service");
    fs::write(&ports_executable, ports_script)?;
    make_executable(&ports_executable)?;
    fs::write(
        ports_fixture.join("cix-manifest.json"),
        format!(
            r#"{{
                "cixManifest": 0,
                "exec": ["bin/service"],
                "ports": {{"declared": {{"value": {declared_port}, "protocol": "tcp"}}}}
            }}"#
        ),
    )?;
    let ports_store = add_to_store(&ports_fixture)?;
    let ports_spec = Spec::load(&ports_store)?;
    let ports_service = ports_spec.select_service(None)?.1;
    let ports_config = ResolvedConfig::resolve(ports_service, &[], &[])?;
    let ports_started = start_service(
        &ports_store,
        "port-test",
        ports_service,
        &ports_config,
        false,
    )?;
    let ports_guard = UnitGuard {
        name: ports_started.name.clone(),
    };
    thread::sleep(Duration::from_millis(500));
    if !command_succeeds("systemctl", &["is-active", "--quiet", &ports_started.name]) {
        bail!("{} did not remain active", ports_started.name);
    }
    assert!(
        unit_file(&ports_started.name)?.contains(&format!("SocketBindAllow=tcp:{declared_port}"))
    );
    assert_property(&ports_started.name, "SocketBindDeny", "any")?;
    stop_service(&ports_started.name, false)?;
    std::mem::forget(ports_guard);
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

fn make_executable(path: &Path) -> Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn wait_for_http(port: u16) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
            let mut response = String::new();
            stream.read_to_string(&mut response)?;
            if response.ends_with("\r\n\r\nok\n") {
                return Ok(());
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    bail!("timed out waiting for listener on 127.0.0.1:{port}")
}

fn systemctl_property(unit: &str, property: &str) -> Result<String> {
    let output = Command::new("systemctl")
        .args(["show", unit, "--property", property, "--value"])
        .output()?;
    if !output.status.success() {
        bail!("failed to read {property} for {unit}");
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn assert_property(unit: &str, property: &str, expected: &str) -> Result<()> {
    let actual = systemctl_property(unit, property)?;
    if actual != expected {
        bail!("{unit} has {property}={actual:?}, expected {expected:?}");
    }
    Ok(())
}

fn unit_file(unit: &str) -> Result<String> {
    let path = systemctl_property(unit, "FragmentPath")?;
    Ok(fs::read_to_string(path)?)
}

fn auto_root_exists(root: &Path) -> Result<bool> {
    let roots = Path::new("/nix/var/nix/gcroots/auto");
    for entry in fs::read_dir(roots).with_context(|| format!("reading {}", roots.display()))? {
        let entry = entry?;
        if fs::read_link(entry.path()).is_ok_and(|target| target == root) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn add_to_store(path: &Path) -> Result<PathBuf> {
    let nix = if Command::new("nix").arg("--version").output().is_ok() {
        "nix"
    } else {
        "/nix/var/nix/profiles/default/bin/nix"
    };
    let output = Command::new(nix)
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
