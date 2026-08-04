//! Executed local-index scenarios that generate `docs/tour/`.
//!
//! Run `cargo test --test tour -- --ignored generate_tour` to update the documents.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use cix_common::Ref;
use regex::Regex;

const TOUR_LISTEN: &str = "127.0.0.1:8420";
const TOUR_CIXFILE_LOCK: &str = r#"{
  "inputs": {
    "pkgs": {
      "url": "github:NixOS/nixpkgs/nixos-unstable",
      "rev": "624af665418d3c65d544145b4d34ad696439570e",
      "narHash": "sha256-m0pDuRJG7EDo9ri+4Ksu83VsI+PlxNC9lNBfydejce4="
    }
  }
}
"#;
const PROJ1_FILES: &[&str] = &[
    "Cixfile",
    "Cixfile.lock",
    "rust/Cargo.toml",
    "rust/Cargo.lock",
    "rust/common/Cargo.toml",
    "rust/common/src/lib.rs",
    "rust/api/Cargo.toml",
    "rust/api/src/main.rs",
    "rust/worker/Cargo.toml",
    "rust/worker/src/main.rs",
];
static NEXT_TOUR_PORT: AtomicU16 = AtomicU16::new(10_000);
// The proj1 chapter runs its documented fixed-port service in the shared user manager and host
// network namespace. Rendering concurrently would make one chapter observe another's listener.
static TOUR_RENDER_LOCK: Mutex<()> = Mutex::new(());

struct Server {
    child: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct UserUnit {
    name: String,
}

impl Drop for UserUnit {
    fn drop(&mut self) {
        let _ = cix_run::runtime::stop_service(&self.name, true);
    }
}

struct Doc {
    text: String,
    _temp: tempfile::TempDir,
    base: PathBuf,
    state_dir: PathBuf,
    bin_dir: PathBuf,
}

impl Doc {
    fn new(name: &str) -> Self {
        let temp = tempfile::Builder::new()
            .prefix(&format!("cix-tour-{name}-"))
            .tempdir_in(test_tmp_dir())
            .expect("creating scenario directory");
        let base = temp.path().to_owned();
        let state_dir = base.join("state");
        let bin_dir = base.join("bin");
        fs::create_dir_all(&bin_dir).expect("creating scenario bin directory");
        std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_cix"), bin_dir.join("cix"))
            .expect("linking test binary");

        Self {
            text: String::new(),
            _temp: temp,
            base,
            state_dir,
            bin_dir,
        }
    }

    fn para(&mut self, text: &str) {
        writeln!(self.text, "{text}\n").expect("writing paragraph");
    }

    fn sh(&mut self, command: &str, expect_success: bool) -> String {
        let state_dir = self.state_dir.clone();
        self.sh_in("$", &state_dir, command, expect_success)
    }

    fn sh_after_warming(&mut self, command: &str, expect_success: bool) -> String {
        let state_dir = self.state_dir.clone();
        // Nix may emit first-use progress while constructing a generation. Warm that work
        // unrecorded so the following, displayed invocation remains one real command whose
        // transcript is independent of the local Nix cache.
        self.run(&state_dir, command, expect_success);
        self.sh_in("$", &state_dir, command, expect_success)
    }

    fn sh_ps_units(&mut self, expect_success: bool, unit_names: &[String]) -> String {
        let state_dir = self.state_dir.clone();
        let output = self.run(&state_dir, "cix ps --json", expect_success);
        let raw = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let rows: Vec<cix_compose::PsRow> = serde_json::from_str(&raw)
            .unwrap_or_else(|error| panic!("cix ps --json emitted invalid JSON: {error}\n{raw}"));
        let rows = rows
            .into_iter()
            .filter(|row| unit_names.iter().any(|name| name == &row.unit))
            .collect::<Vec<_>>();
        let displayed = cix_compose::render_ps_table(&rows);
        self.record("$", "cix ps", &displayed);
        displayed
    }

    fn sh_in(
        &mut self,
        prompt: &str,
        state_dir: &Path,
        command: &str,
        expect_success: bool,
    ) -> String {
        let output = self.run(state_dir, command, expect_success);
        let raw = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        self.record(prompt, command, &raw);
        raw
    }

    fn record(&mut self, prompt: &str, command: &str, output: &str) {
        let displayed_command = normalize(command, &self.base);
        writeln!(self.text, "```sh\n{prompt} {displayed_command}").expect("writing command");
        let normalized = normalize(output, &self.base);
        if !normalized.is_empty() {
            self.text.push_str(&normalized);
            if !normalized.ends_with('\n') {
                self.text.push('\n');
            }
        }
        writeln!(self.text, "```\n").expect("writing transcript");
    }

    fn run(&self, state_dir: &Path, command: &str, expect_success: bool) -> std::process::Output {
        let mut path = self.bin_dir.display().to_string();
        if let Some(existing) = std::env::var_os("PATH") {
            path.push(':');
            path.push_str(&existing.to_string_lossy());
        }
        let output = Command::new("sh")
            .args(["-c", command])
            .current_dir(&self.base)
            .env("CIX_STATE_DIR", state_dir)
            .env("PATH", path)
            .output()
            .unwrap_or_else(|error| panic!("running `{command}`: {error}"));
        let raw = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            output.status.success(),
            expect_success,
            "`{command}` produced:\n{raw}"
        );
        output
    }

    fn background(&mut self, prompt: &str, command: &str) {
        let command = normalize(command, &self.base);
        writeln!(self.text, "```sh\n{prompt} {command} &\n```\n").expect("writing command");
    }

    fn finish(self) -> String {
        self.text
    }
}

fn test_tmp_dir() -> PathBuf {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("target/test-tmp");
    fs::create_dir_all(&path).expect("creating target/test-tmp");
    path
}

fn normalize(raw: &str, base: &Path) -> String {
    let store_hash = Regex::new(r"/nix/store/[0123456789abcdfghijklmnpqrsvwxyz]{32}-")
        .expect("valid store hash regex");
    let port = Regex::new(r"127\.0\.0\.1:\d+").expect("valid port regex");
    let created_at =
        Regex::new(r#"(\"createdAt\"\s*:\s*\")\d{10}(\")"#).expect("valid createdAt regex");
    let age = Regex::new(r"\b\d+s\b").expect("valid age regex");
    let build_wall_time = Regex::new(r" \(\d+ ms\)").expect("valid build wall-time regex");
    let builder_workspace = Regex::new(r"(?m)^BUILDER ([^ ]+) workspace [^\n]+$")
        .expect("valid builder workspace regex");
    let cargo_progress =
        Regex::new(r"(?m)^\s*(?:Compiling [^\n]+|Finished `release` profile[^\n]*)\n?")
            .expect("valid cargo progress regex");
    let unit_name =
        Regex::new(r"cix-run-([a-z][a-z0-9-]*)-[0-9a-f]+\.service").expect("valid unit name regex");
    let stale_failed_unit =
        Regex::new(r"(?m)^user\s+cix-run-[a-z][a-z0-9-]*-NONCE\.service\s+failed/failed.*\n?")
            .expect("valid stale unit regex");
    // The user manager determines both the rejected controls and the error text — and on
    // permissive kernels (unrestricted userns) the manager accepts everything and the pair
    // never appears at all. Presence is host-specific, so the pair is removed entirely.
    let degraded_fallback = Regex::new(
        r"(?ms)^warning: (?:the )?user manager rejected .*?^warning: retrying [^\n]*\n?",
    )
    .expect("valid degraded fallback regex");
    // systemd before version 257 rejects newer unit properties while parsing them. The property
    // name is host-version-specific and is captured by cix's following fallback warning.
    let unknown_assignment = Regex::new(r"(?m)^Unknown assignment: [^\r\n]*(?:\r?\n|$)")
        .expect("valid unknown assignment regex");

    let normalized = store_hash.replace_all(raw, "/nix/store/…-");
    let normalized = port.replace_all(&normalized, TOUR_LISTEN);
    let normalized = created_at.replace_all(&normalized, "${1}1700000000${2}");
    let normalized = age.replace_all(&normalized, "0s");
    let normalized = build_wall_time.replace_all(&normalized, "");
    let normalized =
        builder_workspace.replace_all(&normalized, "BUILDER ${1} workspace <persistent>");
    let normalized = cargo_progress.replace_all(&normalized, "");
    let normalized = unit_name.replace_all(&normalized, "cix-run-${1}-NONCE.service");
    let normalized = unknown_assignment.replace_all(&normalized, "");
    let normalized = degraded_fallback.replace_all(&normalized, "");
    let normalized = stale_failed_unit.replace_all(&normalized, "");
    // Nix emits fetch/build progress on cold stores (CI runners, fresh machines); those
    // lines are environment noise, not command output.
    let nix_progress = Regex::new(
        r"(?m)^(unpacking '|copying path '|building '/nix/store/|querying info about|downloading '|these \d+ (?:derivations|paths) will be (?:built|fetched).*|this (?:derivation|path) will be (?:built|fetched).*)[^\n]*\n?",
    )
    .expect("valid nix progress regex");
    let normalized = nix_progress.replace_all(&normalized, "");
    // Older systemd prefixes transient unit descriptions with "[systemd-run] "; newer does
    // not. The prefix is environment noise.
    let normalized = normalized.replace("[systemd-run] ", "");
    normalized
        .replace(base.to_string_lossy().as_ref(), "~")
        .trim_end()
        .to_owned()
}

fn fixture(doc: &mut Doc, name: &str, contents: &str) -> String {
    let state_dir = doc.state_dir.clone();
    fixture_in(doc, "$", &state_dir, name, contents)
}

fn fixture_in(doc: &mut Doc, prompt: &str, state_dir: &Path, name: &str, contents: &str) -> String {
    doc.sh_in(
        prompt,
        state_dir,
        &format!(
            "mkdir {name} && printf '%s\\n' '{contents}' > {name}/message && printf '%s\\n' '{{\"cixManifest\":0,\"start\":[\"message\"]}}' > {name}/cix-manifest.json"
        ),
        true,
    );
    doc.sh_in(prompt, state_dir, &format!("ls -1 {name}"), true);
    doc.sh_in(
        prompt,
        state_dir,
        &format!("cat {name}/message {name}/cix-manifest.json"),
        true,
    );
    doc.sh_in(
        prompt,
        state_dir,
        &format!("cix tag \"$(nix store add {name})\" my-app:v1"),
        true,
    );
    let table_root = state_dir
        .join("roots/names")
        .join(root_filename())
        .join("table");
    let table_path = fs::read_link(table_root).expect("reading fixture table root");
    let table: serde_json::Value = serde_json::from_slice(
        &fs::read(table_path.join("table.json")).expect("reading fixture table"),
    )
    .expect("parsing fixture table");
    let path = table["tags"]["v1"]["storePath"]
        .as_str()
        .expect("reading fixture store path")
        .to_owned();
    assert!(
        path.starts_with("/nix/store/"),
        "unexpected store path: {path}"
    );
    path
}

fn listener_fixture(doc: &Doc) -> String {
    let fixture = doc.base.join("listener-fixture");
    let executable = fixture.join("bin/listenfds");
    fs::create_dir_all(executable.parent().expect("listener executable parent"))
        .expect("creating listener fixture directory");
    fs::write(
        &executable,
        r#"#!/usr/bin/python3
import os
import socket

listen_fds = int(os.environ.get("LISTEN_FDS", "0"))
listen_pid = int(os.environ.get("LISTEN_PID", "0"))
if listen_fds != 1 or listen_pid != os.getpid():
    raise SystemExit("expected one named systemd listener")

listener = socket.fromfd(3, socket.AF_INET, socket.SOCK_STREAM)
while True:
    connection, _ = listener.accept()
    with connection:
        connection.recv(4096)
        body = b"LISTEN_FDS=1; no socket() authority\n"
        connection.sendall(
            b"HTTP/1.1 200 OK\r\n"
            + b"Content-Type: text/plain\r\n"
            + b"Content-Length: " + str(len(body)).encode() + b"\r\n"
            + b"Connection: close\r\n\r\n" + body
        )
"#,
    )
    .expect("writing listener executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&executable)
            .expect("reading listener executable permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("making listener executable");
    }
    fs::write(
        fixture.join("cix-manifest.json"),
        r#"{
  "cixManifest": 0,
  "start": ["bin/listenfds"],
  "listeners": {"http": {"type": "stream"}}
}
"#,
    )
    .expect("writing listener manifest");

    let output = Command::new("nix")
        .args(["store", "add-path"])
        .arg(&fixture)
        .output()
        .expect("adding listener fixture to the Nix store");
    assert!(
        output.status.success(),
        "nix store add-path failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout)
        .expect("Nix store path is UTF-8")
        .trim()
        .to_owned()
}

fn write_resolved_compose_lock(doc: &Doc, compose_path: &Path, reference: &str) {
    let reference = Ref::parse(reference).expect("parsing compose reference");
    let reference_text = reference.display();
    let pointer: serde_json::Value = serde_json::from_slice(
        &fs::read(
            doc.state_dir
                .join("names")
                .join(cix_index::Store::encode_name(&reference.name)),
        )
        .expect("reading compose name pointer"),
    )
    .expect("parsing compose name pointer");
    let table: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(pointer["storePath"].as_str().expect("pointer store path"))
                .join("table.json"),
        )
        .expect("reading compose tag table"),
    )
    .expect("parsing compose tag table");
    let record = &table["tags"][&reference.tag];
    let store_path = record["storePath"]
        .as_str()
        .expect("finding compose store path")
        .to_owned();
    let nar_hash = record["narHash"]
        .as_str()
        .expect("finding compose nar hash")
        .to_owned();
    fs::write(
        cix_compose::Compose::lock_path(compose_path),
        format!(
            "{{\n  \"paths\": {{\n    \"web\": {{\n      \"ref\": \"{reference_text}\",\n      \"storePath\": \"{store_path}\",\n      \"narHash\": \"{nar_hash}\"\n    }}\n  }}\n}}\n"
        ),
    )
    .expect("writing resolved compose lock");
}

fn next_listen() -> String {
    let port = NEXT_TOUR_PORT.fetch_add(1, Ordering::Relaxed);
    format!("127.0.0.1:{port}")
}

fn start_server(doc: &Doc, state_dir: &Path, listen: &str) -> Server {
    let child = Command::new(doc.bin_dir.join("cix"))
        .args(["serve", "--with-store", "--listen", listen])
        .current_dir(&doc.base)
        .env("CIX_STATE_DIR", state_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("starting cix serve");
    let mut server = Server { child };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = server.child.try_wait().expect("checking cix serve") {
            panic!("cix serve exited before becoming ready: {status}");
        }
        let ready = Command::new("curl")
            .args([
                "-fsS",
                "--max-time",
                "1",
                "-H",
                "Accept: application/vnd.cix+json;version=1",
                &format!("http://{listen}/my-app:v1"),
            ])
            .output()
            .is_ok_and(|output| output.status.success());
        if ready {
            return server;
        }
        assert!(Instant::now() < deadline, "timed out waiting for cix serve");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_for_http(listen: &str, expected: &str) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let output = Command::new("curl")
            .args(["-fsS", "--max-time", "1", &format!("http://{listen}")])
            .output();
        if output.as_ref().is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == expected
        }) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for listener on {listen}: {}",
            output
                .as_ref()
                .map(|output| String::from_utf8_lossy(&output.stderr).trim().to_owned())
                .unwrap_or_else(|error| error.to_string())
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn root_filename() -> &'static str {
    "bXktYXBw"
}

fn chapter_hello() -> String {
    let mut doc = Doc::new("hello");
    fs::write(
        doc.base.join("index.html"),
        "<h1>hello from your first composix service</h1>\n",
    )
    .expect("writing hello page");
    fs::write(
        doc.base.join("nginx.conf"),
        r#"daemon off;
pid /tmp/cix-tour-nginx.pid;
error_log stderr info;
events { }
http {
  access_log off;
  client_body_temp_path /tmp/cix-tour-nginx-body;
  server { listen 18085; root .; }
}
"#,
    )
    .expect("writing hello nginx config");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE hello
IMPORT ${pkgs.nginx}
COPY index.html /srv/www/index.html
COPY nginx.conf /etc/nginx/nginx.conf
START nginx -p ${src}/ -c nginx.conf -e stderr
PORT http = 18085
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
"#,
    )
    .expect("writing hello Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing hello lock");

    doc.para("You will build and run a small nginx service from a Cixfile. Afterwards, you will understand the shortest path from checked-in files to a supervised process.");
    doc.para("Composix is a nix-native Docker analogue. Images become immutable Nix store items, and containers become hardened systemd units. Dockerfiles become Cixfiles that say exactly what enters an item and what its process may use.");

    doc.para("## Before you start");
    doc.para("You need Nix with flakes enabled, `cix`, a running systemd user manager for this rootless walkthrough, and `curl`. Production uses the system manager; `--user` is the deliberately degraded development path and says so when you invoke it.");
    doc.para("Because a restricted user manager cannot project item mounts on this host, the development probe reads its checked-in page through the locked source path and uses private `/tmp` files. The same item still declares nginx's production cache and runtime paths; Chapter 5 returns to that runtime boundary explicitly.");

    doc.para("## Build the item");
    doc.para("Your first Cixfile imports nginx, copies two ordinary project files, names its entrypoint and port, and declares nginx's cache- and runtime-lifetime writable directories.");
    let source = doc.sh("cat Cixfile index.html nginx.conf", true);
    assert!(source.contains("IMPORT ${pkgs.nginx}"));
    assert!(source.contains("START nginx"));
    assert!(source.contains("CACHEDIR /var/cache/nginx"));
    assert!(source.contains("RUNDIR /run/nginx"));

    let built = doc.sh("cix build .", true);
    let store_path = built_store_path(&built, "-cix-item-hello");
    let manifest = doc.sh(&format!("cat {store_path}/cix-manifest.json"), true);
    assert!(manifest.contains("/bin/nginx"));
    assert!(manifest.contains("\"/var/cache/nginx\""));
    assert!(manifest.contains("\"/run/nginx\""));

    doc.para("## Run, probe, stop");
    let started = doc.sh(&format!("cix run {store_path} --user --detach"), true);
    let unit_name = started
        .lines()
        .find(|line| line.starts_with("cix-run-hello-") && line.ends_with(".service"))
        .expect("cix run printed the hello unit")
        .to_owned();
    let _unit = UserUnit {
        name: unit_name.clone(),
    };
    wait_for_http(
        "127.0.0.1:18085",
        "<h1>hello from your first composix service</h1>",
    );
    let response = doc.sh("curl -fsS http://127.0.0.1:18085", true);
    assert_eq!(
        response.trim(),
        "<h1>hello from your first composix service</h1>"
    );
    doc.sh(&format!("systemctl --user stop {unit_name}"), true);
    doc.para("You have now built an immutable item, run its declared service, reached it on its declared port, and stopped the transient unit. The next chapters unpack the language and operational model behind those five minutes.");
    doc.finish()
}

fn chapter_distribution_old() -> String {
    let mut doc = Doc::new("distribution");
    let publisher = doc.state_dir.clone();
    let consumer = doc.base.join("consumer-state");

    doc.para("A served index and a standard Nix binary cache are enough to move the same immutable artifact between machines. Separate state directories stand in for the publisher and consumer here.");
    doc.para("## Serve");
    let first = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "my-app-v1",
        "hello from my app v1",
    );
    let listen = next_listen();
    doc.background(
        "publisher $",
        &format!("cix serve --with-store --listen {listen}"),
    );
    let server = start_server(&doc, &publisher, &listen);
    let entry = doc.sh_in(
        "publisher $",
        &publisher,
        &format!(
            "curl -s -H 'Accept: application/vnd.cix+json;version=1' http://{listen}/my-app:v1"
        ),
        true,
    );
    assert!(entry.contains("\"outputs\":"));
    assert!(entry.contains("\"substituters\":"));
    assert!(entry.contains(&first));

    doc.para("The same URL serves an informative HTML representation to a browser; content negotiation keeps one public name instead of a separate API URL.");
    let html = doc.sh_in(
        "publisher $",
        &publisher,
        &format!("curl -s http://{listen}/my-app:v1 | head -c 120"),
        true,
    );
    assert!(html.contains("<!doctype html>"));

    doc.para("## Pull");
    doc.para("The qualified ref names both its origin and tag. `--as` adopts it under a bare local name while retaining the upstream needed for later refreshes.");
    let pulled = doc.sh_in(
        "consumer $",
        &consumer,
        &format!("cix pull {listen}/my-app:v1 --as my-app:v1"),
        true,
    );
    assert!(pulled.contains("updated 1 tag(s)"));
    let local = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(local.contains("my-app:v1"));
    assert!(local.contains(&first));
    assert!(local.contains(&listen));

    doc.para("## Follow a moving tag");
    doc.para("The publisher can move `my-app:v1` to a new immutable path. A bare `cix pull` refreshes every local tag that remembers an upstream.");
    let second = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "my-app-v2",
        "hello from my app v2",
    );
    let refreshed = doc.sh_in("consumer $", &consumer, "cix pull", true);
    assert!(refreshed.contains("updated 1 tag(s)"));
    let updated = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(updated.contains(&second));
    assert!(!updated.contains(&first));
    doc.para("GC follows those pins: after the refresh, the consumer roots the new path rather than the old one.");
    drop(server);
    doc.finish()
}

fn chapter_cixfile_language() -> String {
    let mut doc = Doc::new("cixfile-language");
    fs::write(doc.base.join("index.html"), "language guide\n")
        .expect("writing language fixture page");
    fs::write(
        doc.base.join("service.conf"),
        "root=/srv/language\nstate=/var/lib/language\n",
    )
    .expect("writing language fixture config");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE language
IMPORT ${pkgs.coreutils} ${pkgs.busybox} ${pkgs.bash}
COPY index.html /srv/language/index.html
COPY ${pkgs.coreutils}/bin/printf /opt/tools/printf
COPY ${pkgs.nginx}/conf /opt/nginx
COPY service.conf /etc/language/service.conf
FILE /etc/language/build-origin <<ORIGIN
packages=${pkgs.coreutils}
ORIGIN
START sleep 60
ENV SITE_NAME = guide
ENV API_TOKEN required
STATEDIR /var/lib/language
STATEDIR /opt/nginx/state
CACHEDIR /var/cache/language
LOGDIR /var/log/language
CONFIGDIR /etc/language
RUNDIR /run/language
PORT web = 8088
PORT dns = udp:5353
LISTENER admin
CLAIM egress
CLAIM jit
"#,
    )
    .expect("writing language Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing language lock");

    doc.para("You will grow the first service into an example of every everyday Cixfile declaration. Afterwards, you will understand the backward-only graph, explicit binders, and capability vocabulary well enough to read a Cixfile without hidden Docker assumptions.");

    doc.para("## A graph you can read from top to bottom");
    doc.para("A Cixfile is a backward-only graph. `FROM` binds inputs, a block binds the artifact it creates, and `${name}` can refer only to something declared earlier; there are no ambient package names or inherited filesystem layers. The local directory is the one deliberate convenience: a bare `COPY index.html …` is the same source as `${src}/index.html` after `FROM . AS src`.");
    let source = doc.sh("cat Cixfile index.html service.conf", true);
    assert!(source.contains("FROM . AS src"));
    assert!(source.contains("COPY index.html /srv/language/index.html"));
    assert!(source.contains("ENV API_TOKEN required"));
    assert!(source.contains("PORT dns = udp:5353"));
    assert!(source.contains("CLAIM egress"));

    doc.para("## IMPORT and COPY");
    doc.para("`IMPORT` unions each package's `bin`, `etc`, and `share` trees into the item. It accepts ordinary package references, bare commands such as `sleep` resolve through that union, and earlier imports win a collision—so coreutils supplies `ls` even though busybox comes later.");
    doc.para("`COPY` makes its storage choice from provenance. Local source bytes are materialized; package, FETCH, builder, and cix-item sources normally remain links into immutable store trees. A later write or a runtime directory mount beneath a link forces that branch to materialize, which is why the package tree containing `STATEDIR /opt/nginx/state` becomes a real directory while `/opt/tools/printf` remains a link.");
    let built = doc.sh("cix build .", true);
    let store_path = built_store_path(&built, "-cix-item-language");
    let layout = doc.sh(
        &format!(
            "test -f {store_path}/srv/language/index.html && test -L {store_path}/opt/tools/printf && test ! -L {store_path}/opt/nginx && printf 'local: materialized\\npackage: linked\\nmount ancestor: materialized\\n'"
        ),
        true,
    );
    assert_eq!(
        layout.trim(),
        "local: materialized\npackage: linked\nmount ancestor: materialized"
    );

    doc.para("`FILE` creates the small interpolated `build-origin` file below. It is useful when the content genuinely needs a binder value; for ordinary configuration it is a smell, because a checked-in file plus `COPY` stays easier to lint, edit, and test.");
    let generated = doc.sh(&format!("cat {store_path}/etc/language/build-origin"), true);
    assert!(generated.contains("packages=/nix/store/") && generated.contains("-coreutils-"));

    doc.para("## Runtime declarations are grants");
    doc.para("`ENV SITE_NAME = guide` supplies a default, while `ENV API_TOKEN required` requires an operator value without baking one into the item. Role directories use the application's native absolute paths and state their lifecycle: state persists, cache is disposable, logs are retained, config is operator-managed content, and run data disappears on stop.");
    doc.para("A bare port is TCP; the `udp:` prefix is the single UDP spelling. `LISTENER admin` is different: systemd owns a TCP socket and passes its file descriptor to the service, which is useful for socket activation and privileged binds. Chapter 6 executes that boundary in a compose setting.");
    doc.para("`CLAIM egress` admits outbound networking, and `CLAIM jit` allows writable executable memory. Without those explicit declarations, the corresponding sandbox authority stays denied.");
    let manifest = doc.sh(
        &format!("jq '{{env, ports, listeners, dirs, claims}}' {store_path}/cix-manifest.json"),
        true,
    );
    assert!(manifest.contains("\"udp\""));
    assert!(manifest.contains("\"admin\""));
    assert!(manifest.contains("\"required\": true"));
    assert!(manifest.contains("\"egress\""));
    assert!(manifest.contains("\"jit\""));

    doc.para("## Directive reference");
    doc.para("| Declaration | What it adds |\n| --- | --- |\n| `FROM … AS name` | A locked package/source/item binder; `FROM .` names local context. |\n| `FETCH name command … EXPECT hash` | A pinned network step and reusable source binder. |\n| `BUILDER name` | A persistent build workspace for `COPY`, `ENV`, `FETCH`, and offline `RUN`. |\n| `SERVICE name` / `APP name` / `ITEM name` | A long-running runtime contract / run-to-completion contract / manifest-less tree. |\n| `IMPORT package…` | An earlier-wins read-only package union with bare command lookup. |\n| `COPY source /destination` | Provenance-aware item assembly; builder destinations are workdir-relative. |\n| `FILE /destination <<EOF` | An inline interpolated file; prefer checked-in files when possible. |\n| `START` / `START_PRE` | The argv entrypoint / idempotent service pre-start argv. |\n| `ENV` / `SECRET` | Declared runtime configuration / credential-file need. |\n| `PORT` / `LISTENER` | A direct TCP/UDP bind / systemd-owned TCP socket. |\n| role dirs / `DIR` | Cix-managed lifecycle storage / operator-supplied data. |\n| `READINESS` / `LIVENESS` | Startup gating / watchdog restart probes. |\n| `CLAIM` / `SHM` | A narrow sandbox exception / bounded private shared memory. |");
    doc.finish()
}

fn chapter_build_run_debug() -> String {
    let mut doc = Doc::new("build-run-debug");
    fs::write(doc.base.join("greeting.txt"), "hello from Cixfile\n")
        .expect("writing Cixfile greeting fixture");
    fs::write(doc.base.join("tour-app"), "exec \"$@\"\n").expect("writing Cixfile script fixture");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

ITEM tour-assets
COPY ${src}/greeting.txt /share/greeting

SERVICE tour-app
COPY ${src}/greeting.txt /share/greeting
COPY ${src}/tour-app /bin/tour-app
START ${pkgs.bash}/bin/sh ${src}/tour-app ${pkgs.coreutils}/bin/sleep 300
"#,
    )
    .expect("writing Cixfile fixture");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing Cixfile lock fixture");

    doc.para("A Cixfile turns source material plus pinned package inputs into a store item with a runtime manifest. We start by looking at every input this tiny build will use.");
    doc.para("## Build");
    doc.sh("ls -1 Cixfile Cixfile.lock greeting.txt tour-app", true);
    let cixfile = doc.sh("cat Cixfile greeting.txt tour-app", true);
    assert!(cixfile.contains("ITEM tour-assets"));
    assert!(cixfile.contains("SERVICE tour-app"));
    assert!(cixfile.contains("COPY ${src}/tour-app /bin/tour-app"));
    assert!(cixfile.contains("START ${pkgs.bash}/bin/sh"));
    let lock = doc.sh("cat Cixfile.lock", true);
    assert!(lock.contains("\"narHash\""));

    doc.para("The package universe is pinned by revision and content hash. These ITEM and SERVICE blocks perform only assembly, so they need no BUILDER: builders exist only when FETCH or RUN has work to do.");
    doc.para("A directory can carry a translation twin beside its default Cixfile: `cix build --file Cixfile.dissolved .` selects that named file and writes its independent sibling lock, `Cixfile.dissolved.lock`.");
    let built = doc.sh("cix build . --namespace tour -t v1", true);
    let store_path = built_store_path(&built, "-cix-item-tour-app");
    let assets_path = built_store_path(&built, "-cix-item-tour-assets");

    doc.para("The ITEM is a pure store tree. It deliberately has no runtime manifest, so it can be tagged and copied from but cannot become a systemd unit.");
    let assets = doc.sh(&format!("find {assets_path} -type f | sort"), true);
    assert!(assets.contains("share/greeting"));
    assert!(!assets.contains("cix-manifest.json"));
    let item_run_error = doc.sh("cix run tour/tour-assets:v1 --user", false);
    assert!(item_run_error.contains("manifest-less ITEM (D68)"));

    doc.para("## Copy from a tagged item");
    doc.para("A tagged cix item is a third FROM input kind. It is a source tree—not a package namespace or inherited root filesystem—so a second Cixfile can copy one declared path from it.");
    let prebuilt = doc.base.join("prebuilt");
    fs::create_dir(&prebuilt).expect("creating tagged-item consumer directory");
    fs::write(
        prebuilt.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM tour/tour-assets:v1 AS prior

APP copied-greeting
COPY ${prior}/share/greeting /share/greeting
START /bin/true
"#,
    )
    .expect("writing tagged-item consumer Cixfile");
    fs::write(prebuilt.join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing tagged-item consumer lock");
    let consumer = doc.sh("cat prebuilt/Cixfile", true);
    assert!(consumer.contains("FROM tour/tour-assets:v1 AS prior"));
    assert!(consumer.contains("COPY ${prior}/share/greeting"));
    let copied = doc.sh("cix build prebuilt", true);
    let copied_path = built_store_path(&copied, "-cix-item-copied-greeting");
    let greeting = doc.sh(&format!("cat {copied_path}/share/greeting"), true);
    assert_eq!(greeting.trim(), "hello from Cixfile");
    doc.para("The generated lock pins the tag's selected store path and NAR hash. A later tag move does not affect this consumer until `cix build --update-lock prior prebuilt` deliberately refreshes that binder.");
    let consumer_lock = doc.sh("cat prebuilt/Cixfile.lock", true);
    assert!(consumer_lock.contains("\"artifacts\""));
    assert!(consumer_lock.contains("tour/tour-assets:v1"));
    assert!(consumer_lock.contains("\"outputs\""));

    doc.para("Before running anything, inspect the generated manifest. It is the hash-covered runtime contract baked into the item: one version-0 service definition, its executable, and any capabilities or writable directories it declares.");
    let manifest = doc.sh(&format!("cat {store_path}/cix-manifest.json"), true);
    assert!(manifest.contains("\"cixManifest\":0"));
    assert!(!manifest.contains("\"children\""));
    assert!(manifest.contains("/bin/sh"));

    doc.para("## Run");
    doc.para("The tag is enough to start a transient service. `--user` is the explicitly degraded rootless development path; production uses the system manager with DynamicUser and the full hardening profile.");
    let started = doc.sh("cix run tour/tour-app:v1 --detach --user", true);
    let unit_name = started
        .lines()
        .find(|line| line.starts_with("cix-run-tour-app-") && line.ends_with(".service"))
        .expect("cix run printed a transient unit name")
        .to_owned();
    let _unit = UserUnit {
        name: unit_name.clone(),
    };
    let own_units = [unit_name.clone()];
    let running = doc.sh_ps_units(true, &own_units);
    assert!(running.contains(&unit_name));
    assert!(running.contains("active/running"));

    doc.para("## Debug");
    doc.para("`cix debug` resolves the same TAG and compiles the same fresh sandbox, but replaces the declared entrypoint with an operator command. Omitting `-- command` opens an interactive shell.");
    let debugged = doc.sh(
        "cix debug tour/tour-app:v1 --user -- /bin/sh -c 'test -n \"$CIX_APP\" && echo debug-command-ran'",
        true,
    );
    assert!(debugged.contains("debug-command-ran"));
    assert!(debugged.contains("cix debug --user is degraded"));

    doc.sh(&format!("systemctl --user stop {unit_name}"), true);
    let stopped = doc.sh_ps_units(true, &own_units);
    assert!(!stopped.contains(&unit_name));
    doc.finish()
}

fn chapter_building_with_run() -> String {
    let mut doc = Doc::new("building-with-run");
    fs::create_dir(doc.base.join("src")).expect("creating RUN fixture source directory");
    fs::write(
        doc.base.join("src/app"),
        "#!/bin/sh\necho hello-from-run-tour\n",
    )
    .expect("writing RUN fixture input");
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(doc.base.join("src/app"))
            .expect("reading RUN fixture mode")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(doc.base.join("src/app"), permissions)
            .expect("making RUN fixture executable");
    }
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/src/ .
RUN <<BUILD
if test -e .cix-warm; then
    printf 'workspace-state: warm\n'
else
    printf 'workspace-state: cold\n'
fi
mkdir -p result
tr '[:lower:]' '[:upper:]' < app > result/upper
touch .cix-warm
BUILD

SERVICE run-tour
COPY ${build}/app /bin/app
COPY ${build}/result/upper /result/upper
START app
"#,
    )
    .expect("writing RUN Cixfile fixture");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing RUN Cixfile lock fixture");

    doc.para("A BUILDER is the workshop side of a Cixfile: it exists because this example has RUN work to perform. First inspect the complete local working directory and the files the build consumes.");
    doc.sh("ls -R .", true);
    let input = doc.sh("cat src/app", true);
    assert!(input.contains("hello-from-run-tour"));
    let cixfile = doc.sh("cat Cixfile", true);
    assert!(cixfile.contains("RUN <<BUILD"));
    doc.sh("cat Cixfile.lock", true);

    doc.para("IMPORT makes bare tools available through the read-only `/bin` union. A command memo combines its directive, imports, environment, and sandbox version with the workspace paths it actually read; unrelated COPY bytes do not invalidate it. The SERVICE consumes only two narrow paths from `${build}`.");
    let first = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .",
        true,
    );
    assert!(first.contains("workspace-state: cold"), "{first}");
    assert!(first.contains("BUILDER build memo miss"), "{first}");
    let first_path = built_store_path(&first, "-cix-item-run-tour");
    let transformed = doc.sh(&format!("tail -n 1 {first_path}/result/upper"), true);
    assert_eq!(transformed.trim(), "ECHO HELLO-FROM-RUN-TOUR");

    doc.para("The lock records just those consumed paths. Repeating the unchanged build materializes them from the store without running the builder.");
    let second = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .",
        true,
    );
    assert!(second.contains("BUILDER build memo hit"), "{second}");
    let second_path = built_store_path(&second, "-cix-item-run-tour");
    assert_eq!(first_path, second_path);

    doc.para("Changing a declared input changes the chain key. The builder runs again in its persistent workspace, so its private marker is warm while the selected outputs still depend only on declared inputs.");
    doc.sh(
        "sed -i 's/hello-from-run-tour/hello-from-run-tour-edited/' src/app",
        true,
    );
    let warm = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .",
        true,
    );
    assert!(warm.contains("workspace-state: warm"), "{warm}");
    let warm_path = built_store_path(&warm, "-cix-item-run-tour");

    doc.para("`--cold` samples the same chain with an empty workspace and now audits the command's read set as well as its selected outputs. This RUN deliberately reads the warm-only marker, so the audit identifies that path-dependent input and fails at the source line.");
    let cold = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build --cold .",
        false,
    );
    assert!(cold.contains("workspace-state: cold"), "{cold}");
    assert!(
        cold.contains("recorded read set differs between warm and cold"),
        "{cold}"
    );

    doc.para("A workspace is only an acceleration structure. Removing it is always safe: the unchanged chain still replays the recorded paths and returns the same item.");
    doc.sh("rm -rf ../.workspaces-run", true);
    let wiped = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-run cix build .",
        true,
    );
    assert!(wiped.contains("BUILDER build memo hit"), "{wiped}");
    assert_eq!(built_store_path(&wiped, "-cix-item-run-tour"), warm_path);
    doc.finish()
}

fn chapter_advanced() -> String {
    let mut doc = Doc::new("advanced");

    doc.para("The basic chapters use ordinary port declarations and single services. This chapter shows two places where composix deliberately exposes the underlying systemd and Nix shapes instead of hiding them.");
    doc.para("## Socket activation");
    let listener_path = listener_fixture(&doc);
    doc.para("The fixture is not opaque: it contains an executable that consumes systemd file descriptor 3 and a version-0 manifest declaring the named `http` listener.");
    doc.sh("ls -R listener-fixture", true);
    let fixture = doc.sh(
        "cat listener-fixture/bin/listenfds listener-fixture/cix-manifest.json",
        true,
    );
    assert!(fixture.contains("socket.fromfd(3"));
    assert!(fixture.contains("\"listeners\""));

    let listen = next_listen();
    let started = doc.sh(
        &format!("cix run {listener_path} --user -p http={listen} --detach"),
        true,
    );
    let unit_name = started
        .lines()
        .find(|line| line.starts_with("cix-run-") && line.ends_with(".service"))
        .expect("cix run printed a listener unit name")
        .to_owned();
    let _unit = UserUnit {
        name: unit_name.clone(),
    };
    wait_for_http(&listen, "LISTEN_FDS=1; no socket() authority");
    let response = doc.sh(&format!("curl -fsS http://{listen}"), true);
    assert_eq!(response.trim(), "LISTEN_FDS=1; no socket() authority");
    doc.sh(&format!("systemctl --user stop {unit_name}"), true);
    doc.para("Stopping the transient service also removes its companion `.socket` unit.");

    doc.para("## Compose");
    let compose_app = doc.base.join("compose-app");
    fs::create_dir(&compose_app).expect("creating compose Cixfile directory");
    write_compose_cixfile(&compose_app, "v1");
    fs::write(compose_app.join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing compose Cixfile lock");
    doc.para("Compose now starts from a real Cixfile-built service rather than a harness-created store path. Its complete build input is visible before use.");
    doc.sh("ls -1 compose-app", true);
    let first_cixfile = doc.sh("cat compose-app/Cixfile compose-app/web", true);
    assert!(first_cixfile.contains("COPY ${src}/web /bin/web"));
    assert!(first_cixfile.contains("START ${pkgs.bash}/bin/sh"));
    assert!(first_cixfile.contains("compose fixture v1"));
    doc.sh("cat compose-app/Cixfile.lock", true);
    let first_build = doc.sh("cix build compose-app -t current", true);
    let first = built_store_path(&first_build, "-cix-item-web");

    fs::write(
        doc.base.join("compose.json"),
        r#"{
  "cixCompose": 1,
  "name": "tour-compose",
  "children": {
    "web": {
      "item": "web:current",
      "update": "track"
    }
  }
}
"#,
    )
    .expect("writing compose declaration");
    let compose = doc.sh("cat compose.json", true);
    assert!(compose.contains("\"update\": \"track\""));
    let checked = doc.sh("cix compose check compose.json", true);
    assert_eq!(
        checked.trim(),
        "compose tour-compose: 1 services, 0 edges, valid"
    );

    write_resolved_compose_lock(&doc, &doc.base.join("compose.json"), "web:current");
    doc.para("`check` resolves and validates without activation. Root `cix up` owns the persistent lock write, so this rootless chapter records the checked tag's actual values before showing the lock and dry diff.");
    let lock = doc.sh("cat cix.lock", true);
    assert!(lock.contains(&first));
    let initial_diff = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(initial_diff.contains(&first));

    write_compose_cixfile(&compose_app, "v2");
    doc.para("Changing the copied script makes a new immutable item; rebuilding with the same tracked tag moves only the name.");
    let second_cixfile = doc.sh("cat compose-app/Cixfile compose-app/web", true);
    assert!(second_cixfile.contains("compose fixture v2"));
    let second_build = doc.sh("cix build compose-app -t current", true);
    let second = built_store_path(&second_build, "-cix-item-web");
    assert_ne!(first, second);
    let changed_diff = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(changed_diff.contains(&second));
    assert!(!changed_diff.contains(&first));

    doc.para("`cix up`, `cix rollback`, and `cix down` use the system manager and therefore require root. The [stack example](../../examples/compose/stack/) VM check covers activation, selective update, rollback, and cleanup.");
    doc.finish()
}

fn write_compose_cixfile(directory: &Path, version: &str) {
    fs::write(
        directory.join("web"),
        format!("echo compose fixture {version}\n"),
    )
    .expect("writing compose script");
    fs::write(
        directory.join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\n\nSERVICE web\nCOPY ${src}/web /bin/web\nSTART ${pkgs.bash}/bin/sh ${src}/web\n",
    )
    .expect("writing compose Cixfile");
}

fn built_store_path(output: &str, suffix: &str) -> String {
    build_member_map(output)
        .into_values()
        .find(|path| path.ends_with(suffix))
        .unwrap_or_else(|| panic!("build did not print an item ending in {suffix:?}:\n{output}"))
}

fn chapter_proj1() -> String {
    let mut doc = Doc::new("proj1");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/build/proj1");
    for relative in PROJ1_FILES {
        let destination = doc.base.join(relative);
        fs::create_dir_all(destination.parent().expect("project file has a parent"))
            .expect("creating proj1 directory");
        fs::copy(source.join(relative), destination).expect("copying proj1 fixture");
    }

    doc.para("This small Rust workspace makes persistent workspaces and narrow output records concrete. First inspect its complete tree, then read the Cixfile that turns it into two independent service artifacts.");
    doc.sh("ls -R .", true);
    let cixfile = doc.sh("cat Cixfile", true);
    assert!(cixfile.contains("BUILDER build"));
    assert!(cixfile.contains("IMPORT ${pkgs.bash}"));
    assert!(!cixfile.contains("\nCACHE "));
    assert!(cixfile.contains("COPY ${src}/rust/ ."));
    assert!(cixfile.contains("COPY ${build}/target/release/proj1-api"));
    assert!(cixfile.contains("RUN <<BUILD"));

    doc.para("One directory COPY stages the declared Rust sources. Cargo's `target/` tree and the marker written by RUN remain in the persistent workspace automatically, while the two SERVICE blocks consume only their own release binaries. The first build is cold.");
    let first = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build . --namespace proj1 -t v1",
        true,
    );
    assert!(first.contains("BUILDER build memo miss"), "{first}");
    assert!(first.contains("workspace-state: cold"), "{first}");
    let first_api = proj1_item_path(&first, "proj1-api");
    let first_worker = proj1_item_path(&first, "proj1-worker");

    doc.para("Changing only worker source changes the chain key and runs the builder in its warm workspace. Cargo rebuilds what changed. Because the lock records each consumed binary separately, the API item does not move.");
    let worker_source = doc.sh("cat rust/worker/src/main.rs", true);
    assert!(worker_source.contains("proj1-worker"));
    doc.sh(
        "sed -i 's/proj1-worker/proj1-worker-edited/' rust/worker/src/main.rs",
        true,
    );
    let edited = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build .",
        true,
    );
    assert!(edited.contains("BUILDER build memo miss"), "{edited}");
    assert!(edited.contains("workspace-state: warm"), "{edited}");
    let edited_api = proj1_item_path(&edited, "proj1-api");
    let edited_worker = proj1_item_path(&edited, "proj1-worker");
    assert_eq!(edited_api, first_api);
    assert_ne!(edited_worker, first_worker);
    let unchanged = doc.sh(
        &format!("test {first_api} = {edited_api} && echo 'api item unchanged: yes'"),
        true,
    );
    assert_eq!(unchanged.trim(), "api item unchanged: yes");

    doc.para("A sampled `--cold` rebuild uses an empty workspace and checks the RUN's traced inputs before accepting its outputs. This fixture deliberately reads warm Cargo state and its marker, so the audit reports the first warm/cold read-set difference instead of silently accepting output coincidence.");
    let clean = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build --cold .",
        false,
    );
    assert!(clean.contains("workspace-state: cold"), "{clean}");
    assert!(
        clean.contains("recorded read set differs between warm and cold"),
        "{clean}"
    );

    doc.para("The warm workspace remains disposable. Delete it and the unchanged chain replays the two recorded binaries without changing either item.");
    doc.sh("rm -rf ../.workspaces-proj1", true);
    let wiped = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/../.workspaces-proj1 cix build .",
        true,
    );
    assert!(wiped.contains("BUILDER build memo hit"), "{wiped}");
    assert_eq!(proj1_item_path(&wiped, "proj1-api"), edited_api);
    assert_eq!(proj1_item_path(&wiped, "proj1-worker"), edited_worker);

    let started = doc.sh("cix run proj1/proj1-api:v1 --user --detach", true);
    let unit_name = started
        .lines()
        .find(|line| line.starts_with("cix-run-proj1-api-") && line.ends_with(".service"))
        .expect("cix run printed the proj1 api unit")
        .to_owned();
    let _unit = UserUnit {
        name: unit_name.clone(),
    };
    wait_for_http("127.0.0.1:18084", "hello from proj1-api");
    let response = doc.sh("curl -fsS http://127.0.0.1:18084", true);
    assert_eq!(response.trim(), "hello from proj1-api");
    doc.sh(&format!("systemctl --user stop {unit_name}"), true);
    doc.finish()
}

fn proj1_item_path(output: &str, name: &str) -> String {
    build_member_map(output)
        .remove(name)
        .unwrap_or_else(|| panic!("proj1 build did not print the {name} item:\n{output}"))
}

fn build_member_map(output: &str) -> std::collections::BTreeMap<String, String> {
    let json = output
        .lines()
        .find(|line| line.starts_with('{'))
        .unwrap_or_else(|| panic!("build did not print a JSON member map:\n{output}"));
    serde_json::from_str(json)
        .unwrap_or_else(|error| panic!("build printed invalid member JSON: {error}\n{output}"))
}

#[derive(Debug, PartialEq, Eq)]
struct GeneratedFile {
    name: &'static str,
    content: String,
}

struct Scenario {
    filename: &'static str,
    title: &'static str,
    description: &'static str,
    body: String,
}

fn auto_generated_notice() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit = option_env!("GIT_COMMIT_HASH").unwrap_or("unknown");
    format!(
        "> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.\n> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.\n> Version **{version}**, commit `{commit}`.\n> **Do not edit** — re-run the test to regenerate.\n"
    )
}

fn render_index(scenarios: &[Scenario]) -> String {
    let mut index = format!(
        "# cix — tour\n\n{}\nThis executable tour follows composix from naming and distribution through building, running, debugging, and composing. Each chapter is one continuous story: inputs are shown before use, commands are real, and assertions keep the prose honest.\n\n## Chapters\n",
        auto_generated_notice()
    );
    for scenario in scenarios {
        writeln!(
            index,
            "\n- [{}]({}) — {}",
            scenario.title,
            scenario.filename.replace(".md", ".html"),
            scenario.description
        )
        .expect("writing scenario index");
    }
    index
}

fn render_scenario(scenarios: &[Scenario], position: usize) -> String {
    let scenario = &scenarios[position];
    let mut page = format!("# {}\n\n{}\n", scenario.title, auto_generated_notice());
    page.push_str(&scenario.body);
    page.push_str("\n---\n\n");
    if let Some(previous) = position.checked_sub(1) {
        write!(
            page,
            "[← Previous]({}) · ",
            scenarios[previous].filename.replace(".md", ".html")
        )
        .expect("writing previous link");
    }
    page.push_str("[Tour index](index.html)");
    if let Some(next) = scenarios.get(position + 1) {
        write!(
            page,
            " · [Next →]({})",
            next.filename.replace(".md", ".html")
        )
        .expect("writing next link");
    }
    page.push('\n');
    page
}

fn render_tour() -> Vec<GeneratedFile> {
    let _lock = TOUR_RENDER_LOCK.lock().expect("locking tour renderer");
    let scenarios = vec![
        Scenario {
            filename: "01-hello-composix.md",
            title: "Chapter 1: Hello, composix",
            description: "Build, run, probe, and stop your first Cixfile service.",
            body: chapter_hello(),
        },
        Scenario {
            filename: "02-cixfile-language.md",
            title: "Chapter 2: The Cixfile language",
            description:
                "Learn binders, assembly, runtime declarations, and the directive vocabulary.",
            body: chapter_cixfile_language(),
        },
        Scenario {
            filename: "03-build-run-debug.md",
            title: "Chapter 3: Build, run, debug",
            description: "Read a Cixfile, build its manifest, run by tag, and debug the same tag.",
            body: chapter_build_run_debug(),
        },
        Scenario {
            filename: "04-building-with-run.md",
            title: "Chapter 4: Building with RUN",
            description: "Build through a persistent workspace and replay only consumed paths.",
            body: chapter_building_with_run(),
        },
        Scenario {
            filename: "05-proj1.md",
            title: "Chapter 5: proj1",
            description: "Build two services from one Rust workspace and run the API.",
            body: chapter_proj1(),
        },
        Scenario {
            filename: "06-advanced.md",
            title: "Chapter 6: Advanced",
            description: "Inspect socket activation, then compose a real Cixfile-built service.",
            body: chapter_advanced(),
        },
    ];
    let mut files = Vec::with_capacity(scenarios.len() + 1);
    files.push(GeneratedFile {
        name: "index.md",
        content: render_index(&scenarios),
    });
    files.extend(
        scenarios
            .iter()
            .enumerate()
            .map(|(position, _)| GeneratedFile {
                name: scenarios[position].filename,
                content: render_scenario(&scenarios, position),
            }),
    );
    files
}

fn tour_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/tour")
}

#[test]
#[ignore = "run explicitly to update docs/tour/"]
fn generate_tour() {
    let directory = tour_dir();
    if directory.exists() {
        fs::remove_dir_all(&directory).expect("removing stale tour pages");
    }
    fs::create_dir_all(&directory).expect("creating tour directory");
    for file in render_tour() {
        let path = directory.join(file.name);
        fs::write(&path, file.content).unwrap_or_else(|error| {
            panic!("writing {}: {error}", path.display());
        });
        eprintln!("wrote {}", path.display());
    }
}

#[test]
fn generated_tour_is_deterministic() {
    assert_eq!(render_tour(), render_tour());
}

#[test]
fn normalize_swallows_every_host_specific_degraded_fallback_detail() {
    let base = Path::new("/tour");
    let namespace = "warning: the user manager rejected mount-namespace sandboxing (Operation not supported\ncaused by: host policy)\nwarning: retrying without PrivateUsers, PrivatePIDs, ProtectSystem, ProtectHome, PrivateTmp, and BindPaths; managed *Directory persistence remains";
    let capability = "warning: user manager rejected capability controls (Failed to set capabilities)\nwarning: retrying after dropping AmbientCapabilities, CapabilityBoundingSet, ProtectKernelModules, and ProtectKernelLogs";
    let private_devices = "warning: user manager rejected PrivateDevices isolation (Operation not permitted)\nwarning: retrying without PrivateDevices; this --user service can access the host device namespace (D13 degraded fallback)";
    let old_systemd = "Unknown assignment: PrivatePIDs=yes\nwarning: the user manager rejected mount-namespace sandboxing (Operation not supported)\nwarning: retrying without PrivateUsers";
    // Presence of the pair is itself host-specific (permissive kernels emit nothing), so
    // normalization removes it entirely: degraded and non-degraded hosts must render alike.
    assert_eq!(normalize(namespace, base), "");
    assert_eq!(normalize(capability, base), "");
    assert_eq!(normalize(private_devices, base), "");
    assert_eq!(normalize(old_systemd, base), "");
}

#[test]
fn tour_ignores_a_foreign_user_unit() {
    let output = Command::new("systemd-run")
        .args(["--user", "--unit=cix-run-decoy-x", "sleep", "60"])
        .output()
        .expect("starting foreign user unit");
    assert!(
        output.status.success(),
        "starting foreign user unit failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let _decoy = UserUnit {
        name: "cix-run-decoy-x.service".into(),
    };

    let rendered = render_tour();
    assert!(
        rendered
            .iter()
            .all(|file| !file.content.contains("cix-run-decoy-x")),
        "the foreign user unit leaked into the tour"
    );
}

#[test]
fn tour_matches_committed_document() {
    let expected = render_tour();
    let mut expected_names = expected
        .iter()
        .map(|file| file.name.to_owned())
        .collect::<Vec<_>>();
    expected_names.sort();
    let mut actual_names = fs::read_dir(tour_dir())
        .expect("reading docs/tour")
        .map(|entry| {
            entry
                .expect("reading docs/tour entry")
                .file_name()
                .into_string()
                .expect("tour filename is UTF-8")
        })
        .collect::<Vec<_>>();
    actual_names.sort();
    assert_eq!(
        actual_names, expected_names,
        "docs/tour has added, removed, or renamed pages; run `cargo test --test tour -- --ignored generate_tour`"
    );
    for file in expected {
        let path = tour_dir().join(file.name);
        let actual = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
        assert_eq!(
            actual,
            file.content,
            "{} has drifted; run `cargo test --test tour -- --ignored generate_tour`",
            path.display()
        );
    }
}
