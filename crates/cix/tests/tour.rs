//! Executed local-index scenarios that generate `docs/tour/`.
//!
//! Run `cargo test --test tour -- --ignored generate_tour` to update the documents.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc;
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
        let _ = wait_for_user_units_gone([self.name.as_str()]);
    }
}

fn user_cix_units() -> Result<BTreeSet<String>, String> {
    let output = Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "cix-*",
            "--all",
            "--output=json",
            "--no-pager",
            "--no-legend",
        ])
        .output()
        .map_err(|error| format!("listing user cix units: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "listing user cix units failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let units: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("parsing user cix units: {error}"))?;
    units
        .into_iter()
        .map(|unit| {
            unit.get("unit")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "systemctl user unit has no string unit field".to_owned())
        })
        .collect()
}

fn wait_for_user_units_gone<'a>(units: impl IntoIterator<Item = &'a str>) -> Result<(), String> {
    let units = units.into_iter().map(str::to_owned).collect::<Vec<_>>();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let present = user_cix_units()?;
        let remaining = units
            .iter()
            .filter(|unit| present.contains(*unit))
            .cloned()
            .collect::<Vec<_>>();
        if remaining.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!(
                "timed out waiting for user units to unload: {}",
                remaining.join(", ")
            ));
        }
        // `systemd-run --collect` unloads asynchronously after stop. Waiting here keeps the
        // next tour receipt from observing a unit created by the preceding receipt.
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn stop_user_units_created_since(before: &BTreeSet<String>, prefix: &str, receipt: &str) {
    let after =
        user_cix_units().unwrap_or_else(|error| panic!("listing units after {receipt}: {error}"));
    let mut created = after
        .difference(before)
        .filter(|unit| unit.starts_with(prefix))
        .cloned()
        .collect::<Vec<_>>();
    created.sort_by_key(|unit| unit.ends_with(".slice"));
    for unit in &created {
        let output = Command::new("systemctl")
            .args(["--user", "stop", unit])
            .output()
            .unwrap_or_else(|error| panic!("stopping {unit} after {receipt}: {error}"));
        if !output.status.success()
            && user_cix_units()
                .unwrap_or_else(|error| panic!("checking {unit} after {receipt}: {error}"))
                .contains(unit)
        {
            panic!(
                "stopping {unit} after {receipt} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
    }
    wait_for_user_units_gone(created.iter().map(String::as_str))
        .unwrap_or_else(|error| panic!("tearing down units after {receipt}: {error}"));
}

fn stop_user_unit(unit: &str, receipt: &str) {
    cix_run::runtime::stop_service(unit, true)
        .unwrap_or_else(|error| panic!("stopping {unit} after {receipt}: {error:#}"));
    wait_for_user_units_gone([unit])
        .unwrap_or_else(|error| panic!("tearing down {unit} after {receipt}: {error}"));
}

fn stop_empty_cix_run_slice(receipt: &str) {
    let active = Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "cix-run-*.service",
            "cix-debug-*.service",
            "cix-run-*.socket",
            "cix-run-*.timer",
            "--state=active",
            "--no-pager",
            "--no-legend",
        ])
        .output()
        .unwrap_or_else(|error| panic!("checking active cix units after {receipt}: {error}"));
    assert!(
        active.status.success(),
        "checking active cix units after {receipt} failed: {}",
        String::from_utf8_lossy(&active.stderr).trim()
    );
    if !active.stdout.is_empty() {
        return;
    }
    let units = user_cix_units()
        .unwrap_or_else(|error| panic!("checking cix-run.slice after {receipt}: {error}"));
    if !units.contains("cix-run.slice") {
        return;
    }
    let output = Command::new("systemctl")
        .args(["--user", "stop", "cix-run.slice"])
        .output()
        .unwrap_or_else(|error| panic!("stopping cix-run.slice after {receipt}: {error}"));
    assert!(
        output.status.success(),
        "stopping cix-run.slice after {receipt} failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    wait_for_user_units_gone(["cix-run.slice"])
        .unwrap_or_else(|error| panic!("tearing down cix-run.slice after {receipt}: {error}"));
}

struct ScheduledUserUnit {
    timer: String,
}

impl Drop for ScheduledUserUnit {
    fn drop(&mut self) {
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &self.timer])
            .output();
        let Some(stem) = self.timer.strip_suffix(".timer") else {
            return;
        };
        let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") else {
            return;
        };
        let directory = PathBuf::from(runtime).join("systemd/user");
        for suffix in [".service", ".timer", "-root.service"] {
            let _ = fs::remove_file(directory.join(format!("{stem}{suffix}")));
        }
        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .output();
        let service = format!("{stem}.service");
        let root_service = format!("{stem}-root.service");
        let _ = wait_for_user_units_gone([
            self.timer.as_str(),
            service.as_str(),
            root_service.as_str(),
        ]);
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

    fn sh_with_env(
        &mut self,
        command: &str,
        variables: &[(&str, &str)],
        expect_success: bool,
    ) -> String {
        let output = self.run_with_env(&self.state_dir, command, variables, expect_success);
        let raw = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        self.record("$", command, &raw);
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
        self.run_with_env(state_dir, command, &[], expect_success)
    }

    fn run_with_env(
        &self,
        state_dir: &Path,
        command: &str,
        variables: &[(&str, &str)],
        expect_success: bool,
    ) -> std::process::Output {
        let mut path = self.bin_dir.display().to_string();
        if let Some(existing) = std::env::var_os("PATH") {
            path.push(':');
            path.push_str(&existing.to_string_lossy());
        }
        let mut process = Command::new("sh");
        process
            .args(["-c", command])
            .current_dir(&self.base)
            .env("CIX_STATE_DIR", state_dir)
            .env("PATH", path);
        for (name, value) in variables {
            process.env(name, value);
        }
        let output = process
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

    fn output(&mut self, output: &str) {
        let normalized = normalize(output, &self.base);
        writeln!(self.text, "```text\n{normalized}\n```\n").expect("writing command output");
    }

    fn show_file(&mut self, path: impl AsRef<Path>) -> String {
        let path = path.as_ref();
        let actual = if path.is_absolute() {
            path.to_owned()
        } else {
            self.base.join(path)
        };
        let contents = fs::read_to_string(&actual)
            .unwrap_or_else(|error| panic!("reading displayed file {}: {error}", actual.display()));
        let label = relative_file_label(path, &self.base);
        let language = file_language(path);
        writeln!(self.text, "#### `{}`\n", label.display()).expect("writing file label");
        writeln!(self.text, "```{language}").expect("writing file fence");
        let normalized = normalize(&contents, &self.base);
        self.text.push_str(&normalized);
        if !normalized.ends_with('\n') {
            self.text.push('\n');
        }
        writeln!(self.text, "```\n").expect("writing file content");
        contents
    }

    fn finish(self) -> String {
        self.text
    }
}

fn relative_file_label(path: &Path, base: &Path) -> PathBuf {
    if !path.is_absolute() {
        return path.to_owned();
    }
    if let Ok(relative) = path.strip_prefix(base) {
        return relative.to_owned();
    }
    if let Ok(store_relative) = path.strip_prefix("/nix/store") {
        let mut components = store_relative.components();
        if components.next().is_some() {
            let relative = components.as_path();
            if !relative.as_os_str().is_empty() {
                return relative.to_owned();
            }
        }
    }
    path.file_name().map(PathBuf::from).unwrap_or_default()
}

fn file_language(path: &Path) -> &'static str {
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if filename.ends_with(".lock") {
        return "json";
    }
    if filename.starts_with("Cixfile") {
        return "dockerfile";
    }
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("nix") => "nix",
        Some("conf") => "nginx",
        Some("py") => "python",
        Some("json") => "json",
        Some("html") => "html",
        _ => "",
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
    let age = Regex::new(r"(?m)(\s{2,})\d+s(\s*)$").expect("valid age regex");
    let build_wall_time = Regex::new(r" \(\d+ ms\)").expect("valid build wall-time regex");
    let nginx_diagnostic =
        Regex::new(r"(?m)^\d{4}/\d{2}/\d{2} \d{2}:\d{2}:\d{2} \[emerg\] \d+#\d+:")
            .expect("valid nginx diagnostic regex");
    let local_fetch_memo = Regex::new(r"(?m)^(FETCH native memo (?:miss|hit)) [0-9a-f]{12}")
        .expect("valid local FETCH memo regex");
    let builder_workspace = Regex::new(r"(?m)^BUILDER ([^ ]+) workspace [^\n]+$")
        .expect("valid builder workspace regex");
    let cargo_progress =
        Regex::new(r"(?m)^\s*(?:Compiling [^\n]+|Finished `release` profile[^\n]*)\n?")
            .expect("valid cargo progress regex");
    let unit_name = Regex::new(r"cix-run-([a-z][a-z0-9-]*)-[0-9a-f]+\.(service|timer)")
        .expect("valid unit name regex");
    let observer_stats = Regex::new(r"(?m)^(user  run  cix-run-observer-[0-9a-f]+\.service)  .+$")
        .expect("valid observer stats regex");
    let stale_failed_unit =
        Regex::new(r"(?m)^user\s+cix-run-[a-z][a-z0-9-]*-NONCE\.service\s+failed/failed.*\n?")
            .expect("valid stale unit regex");
    let degraded_user =
        Regex::new(r"(?m)^warning: --user is degraded development mode;[^\r\n]*(?:\r?\n|$)")
            .expect("valid --user warning regex");
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
    let normalized = age.replace_all(&normalized, "${1}0s${2}");
    let normalized = build_wall_time.replace_all(&normalized, "");
    let normalized = nginx_diagnostic.replace_all(&normalized, "nginx: [emerg]");
    let normalized = local_fetch_memo.replace_all(&normalized, "${1} <command-key>");
    let normalized =
        builder_workspace.replace_all(&normalized, "BUILDER ${1} workspace <persistent>");
    let normalized = cargo_progress.replace_all(&normalized, "");
    // Accounting values are live by definition; retain the asserted row identity and schema.
    let normalized =
        observer_stats.replace_all(&normalized, "${1}  <live>  <live>  <live>  <live>  <live>");
    let normalized = unit_name.replace_all(&normalized, "cix-run-${1}-NONCE.${2}");
    let normalized = degraded_user.replace_all(
        &normalized,
        "[manager degradation warnings vary by host — elided]\n",
    );
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
    doc.show_file(format!("{name}/message"));
    doc.show_file(format!("{name}/cix-manifest.json"));
    let added = doc.sh_in(
        prompt,
        state_dir,
        &format!("item_v1=$(nix store add {name}); printf '%s\\n' \"$item_v1\""),
        true,
    );
    let path = added
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("nix store add printed the v1 item")
        .to_owned();
    let output = doc.run_with_env(
        state_dir,
        "cix tag \"$item_v1\" my-app:v1",
        &[("item_v1", &path)],
        true,
    );
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    doc.record(prompt, "cix tag \"$item_v1\" my-app:v1", &raw);
    let table_root = state_dir
        .join("roots/names")
        .join(root_filename())
        .join("table");
    let table_path = fs::read_link(table_root).expect("reading fixture table root");
    let table: serde_json::Value = serde_json::from_slice(
        &fs::read(table_path.join("table.json")).expect("reading fixture table"),
    )
    .expect("parsing fixture table");
    let recorded_path = table["tags"]["v1"]["storePath"]
        .as_str()
        .expect("reading fixture store path")
        .to_owned();
    assert!(
        recorded_path.starts_with("/nix/store/"),
        "unexpected store path: {recorded_path}"
    );
    assert_eq!(recorded_path, path);
    path
}

fn listener_fixture(doc: &Doc) {
    let fixture = doc.base.join("listener-fixture");
    fs::create_dir(&fixture).expect("creating listener fixture directory");
    let executable = fixture.join("listenfds.py");
    fs::write(
        &executable,
        r#"#!/usr/bin/env python3
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
        fixture.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE listener-demo
IMPORT ${pkgs.coreutils} ${pkgs.python3}
COPY listenfds.py /bin/listenfds
START listenfds
LISTENER http
"#,
    )
    .expect("writing listener Cixfile");
    fs::write(fixture.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing listener lock");
}

fn write_resolved_compose_lock_entries(doc: &Doc, compose_path: &Path, entries: &[(&str, &str)]) {
    let mut paths = serde_json::Map::new();
    for (path, reference) in entries {
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
        paths.insert(
            (*path).to_owned(),
            serde_json::json!({
                "ref": reference_text,
                "storePath": store_path,
                "narHash": nar_hash,
            }),
        );
    }
    fs::write(
        cix_compose::Compose::lock_path(compose_path),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({ "paths": paths }))
                .expect("serializing compose lock")
        ),
    )
    .expect("writing resolved compose lock");
}

fn next_listen() -> String {
    let port = NEXT_TOUR_PORT.fetch_add(1, Ordering::Relaxed);
    format!("127.0.0.1:{port}")
}

fn tree_hash(name: &str, contents: &[u8]) -> String {
    let directory = tempfile::Builder::new()
        .prefix("cix-tour-hash-")
        .tempdir_in(test_tmp_dir())
        .expect("creating hash fixture");
    fs::write(directory.path().join(name), contents).expect("writing hash fixture");
    let output = Command::new("nix")
        .args(["hash", "path", "--sri"])
        .arg(directory.path())
        .output()
        .expect("hashing fixture tree");
    assert!(
        output.status.success(),
        "nix hash path failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    String::from_utf8(output.stdout)
        .expect("Nix hash is UTF-8")
        .trim()
        .to_owned()
}

fn fhs_elf_fixture() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalizing repository root");
    let expression = format!(
        r#"let pkgs = import (builtins.getFlake "path:{}").inputs.nixpkgs {{ system = "x86_64-linux"; }}; in
pkgs.runCommand "cix-tour-fhs-elf" {{ nativeBuildInputs = [ pkgs.gcc pkgs.patchelf ]; }} ''
  printf '#include <stdio.h>\nint main(void) {{ puts("fhs-tour-ok"); return 0; }}\n' > probe.c
  cc probe.c -o fhs-probe
  patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 --remove-rpath fhs-probe
  mkdir -p "$out"
  cp fhs-probe "$out/fhs-probe"
''"#,
        root.display()
    );
    let output = Command::new("nix")
        .args([
            "build",
            "--impure",
            "--no-link",
            "--print-out-paths",
            "--expr",
            &expression,
        ])
        .output()
        .expect("building FHS ELF fixture");
    assert!(
        output.status.success(),
        "building FHS ELF failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("FHS fixture path is UTF-8")
            .trim(),
    )
}

fn start_file_server(doc: &Doc, directory: &Path, listen: &str) -> Server {
    let (host, port) = listen.split_once(':').expect("host:port listen address");
    let child = Command::new("python3")
        .args(["-m", "http.server", "--bind", host, port, "--directory"])
        .arg(directory)
        .current_dir(&doc.base)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("starting fixture file server");
    let mut server = Server { child };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some(status) = server.child.try_wait().expect("checking file server") {
            panic!("fixture file server exited before becoming ready: {status}");
        }
        let ready = Command::new("curl")
            .args([
                "-fsS",
                "--max-time",
                "1",
                &format!("http://{listen}/fhs-probe"),
            ])
            .output()
            .is_ok_and(|output| output.status.success());
        if ready {
            return server;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for fixture file server"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
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
error_log stderr info;
events { }
http {
  access_log off;
  client_body_temp_path /tmp/cix-tour-nginx-client-body;
  server { listen 8420; root srv/www; }
}
"#,
    )
    .expect("writing hello nginx config");
    fs::write(
        doc.base.join("start-hello"),
        r#"#!/usr/bin/env bash
set -eu
prefix=${CIX_APP:-/}
exec nginx -p "$prefix" -c etc/nginx/nginx.conf -e stderr -g 'pid /tmp/cix-tour-nginx.pid;'
"#,
    )
    .expect("writing hello launcher");
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(doc.base.join("start-hello"))
            .expect("reading hello launcher permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(doc.base.join("start-hello"), permissions)
            .expect("making hello launcher executable");
    }
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE hello
IMPORT ${pkgs.nginx} ${pkgs.bash} ${pkgs.coreutils}
COPY index.html /srv/www/index.html
COPY nginx.conf /etc/nginx/nginx.conf
COPY start-hello /bin/start-hello
START start-hello
PORT http = 8420
CACHEDIR /var/cache/nginx
RUNDIR /run/nginx
"#,
    )
    .expect("writing hello Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing hello lock");

    doc.para("You will build and run a small nginx service from ordinary checked-in files. A build result is an **item**: one immutable directory in `/nix/store` containing the program, its files, and a machine-readable service manifest. A Cixfile is the declaration that assembles that directory and states the process's runtime needs.");

    doc.para("## Before you start");
    doc.para("Install the current alpha with `nix profile install github:mathijshenquet/composix#cix`, or use `cix` from this repository's `devenv` shell. The commands below require Linux, Nix, and a per-user systemd manager; macOS, non-systemd Linux, and containers or WSL sessions without user systemd can follow the build sections but cannot run the service lifecycle.");
    let nix_version = doc.sh("nix --version", true);
    assert!(nix_version.contains("nix"));
    let flakes = doc.sh(
        "nix flake metadata --no-write-lock-file github:NixOS/nixpkgs/624af665418d3c65d544145b4d34ad696439570e >/dev/null && printf 'flakes: available\\n'",
        true,
    );
    assert_eq!(flakes.trim(), "flakes: available");
    let manager = doc.sh(
        "systemctl --user is-system-running >/dev/null 2>&1 && printf 'user manager: running\\n' || { state=$(systemctl --user is-system-running 2>/dev/null); test \"$state\" = degraded && printf 'user manager: running (degraded)\\n'; }",
        true,
    );
    assert!(manager.contains("user manager: running"));
    doc.para("Here **rootless** means that `cix run --user` asks your per-user systemd manager to start the unit without root privileges. This development path lacks `DynamicUser=` and may lose mount-namespace, device, PID, and capability restrictions that the system manager provides; cix prints that degradation instead of implying production-equivalent isolation.");

    doc.para("## Build the item");
    doc.para("`FROM … AS pkgs` selects a Nix package collection; the adjacent `Cixfile.lock` records its immutable Git revision and NAR hash, a fingerprint of the serialized source tree. `IMPORT` adds selected packages' command and data trees to the item, so `nginx` and `bash` can be named without host-installed copies. `${pkgs.nginx}` means the `nginx` package from the earlier `pkgs` name; `${…}` is Cixfile build-time substitution, not a shell variable.");
    doc.para("The service copies its page, configuration, and launcher; names its entrypoint and inbound port; and declares two **role directories**, writable paths whose lifecycle systemd manages. `CACHEDIR` data may be cleaned and survives an ordinary restart, while `RUNDIR` is recreated for each service lifetime. The launcher uses the real item path exposed as `CIX_APP` only on the degraded user path, so this demo keeps working when that manager cannot project the copied `/etc` and `/srv` paths.");
    let source = ["Cixfile", "index.html", "nginx.conf", "start-hello"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("IMPORT ${pkgs.nginx}"));
    assert!(source.contains("START start-hello"));
    assert!(source.contains("CACHEDIR /var/cache/nginx"));
    assert!(source.contains("RUNDIR /run/nginx"));

    doc.para("Run from the directory containing these four files and `Cixfile.lock`. Capture a one-member build with a selector; this teaches the reusable shell idiom for every later command. The ellipsis in displayed output is normalization only: `$item` contains the complete path.");
    let built = doc.sh("item=$(cix build .#hello); printf '%s\\n' \"$item\"", true);
    let store_path = built
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected hello build printed an item")
        .to_owned();
    doc.para("`cix-manifest.json` is generated inside the item, not in the project. The build compiler derives its absolute command, read-only projections (mounts), port grant, and writable directory roles from the Cixfile; the runtime validates this manifest before compiling a unit.");
    let manifest = doc.sh_with_env(
        "jq '{start, mounts:[.mounts[] | select(. == \"/etc/nginx\" or . == \"/srv/www\")], ports, dirs}' \"$item/cix-manifest.json\"",
        &[("item", &store_path)],
        true,
    );
    assert!(manifest.contains("bin/start-hello"));
    assert!(manifest.contains("\"/var/cache/nginx\""));
    assert!(manifest.contains("\"/run/nginx\""));

    doc.para("## Run, probe, and stop it");
    doc.para("A **projection** is a read-only bind mount that makes an item path such as `$item/srv/www` appear at its declared service path such as `/srv/www`. The production system manager supplies those projections and stronger isolation; this rootless demo also has the `CIX_APP` fallback described above. The harness normalizes host-varying manager degradation warnings to the fixed marker line `[manager degradation warnings vary by host — elided]`; the service, HTTP probe, and stop command still really execute.");
    let started = doc.sh_with_env(
        "unit=$(cix run \"$item\" --user --detach); printf '%s\\n' \"$unit\"",
        &[("item", &store_path)],
        true,
    );
    let unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-hello-") && line.ends_with(".service"))
        .expect("hello run printed its unit")
        .to_owned();
    wait_for_http(
        TOUR_LISTEN,
        "<h1>hello from your first composix service</h1>",
    );
    let response = doc.sh("curl -fsS http://127.0.0.1:8420", true);
    assert_eq!(
        response.trim(),
        "<h1>hello from your first composix service</h1>"
    );
    doc.sh_with_env("systemctl --user stop \"$unit\"", &[("unit", &unit)], true);
    wait_for_user_units_gone([unit.as_str()]).expect("hello unit unloads after stop");
    stop_empty_cix_run_slice("the Chapter 1 lifecycle");

    doc.para("You now have the complete first loop: checked-in files became one immutable item, its manifest became a named systemd unit, an HTTP request reached the real process, and the exact printed unit was stopped.");
    doc.finish()
}

fn chapter_cixfile_language() -> String {
    let mut doc = Doc::new("cixfile-language");
    fs::write(doc.base.join("index.html"), "guide site\n").expect("writing language fixture page");
    fs::write(
        doc.base.join("service.conf"),
        "root=/srv/guide-site\nstate=/var/lib/guide-site\n",
    )
    .expect("writing language fixture config");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

SERVICE guide-site
IMPORT ${pkgs.coreutils} ${pkgs.busybox} ${pkgs.bash}
COPY index.html /srv/guide-site/index.html
COPY ${pkgs.coreutils}/bin/printf /opt/tools/printf
COPY ${pkgs.nginx}/conf /opt/nginx
COPY service.conf /etc/guide-site/service.conf
FILE /etc/guide-site/build-origin <<ORIGIN
packages=${pkgs.coreutils}
ORIGIN
START sleep 60
ENV SITE_NAME = guide
ENV API_TOKEN required
STATEDIR /var/lib/guide-site
STATEDIR /opt/nginx/state
CACHEDIR /var/cache/guide-site
LOGDIR /var/log/guide-site
CONFIGDIR /etc/guide-site
RUNDIR /run/guide-site
PORT web = 8088
PORT dns = udp:5353
LISTENER admin
CLAIM egress
CLAIM jit
"#,
    )
    .expect("writing language Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing language lock");

    doc.para("You will expand the first service into an example of the everyday Cixfile declarations. Each declaration either names an input, assembles the item's filesystem, or grants a narrowly described runtime capability.");

    doc.para("## A graph you can read from top to bottom");
    doc.para("A **binder** is a name introduced by `AS`, `FETCH`, or `BUILDER` and referenced later as `${name}`. A SERVICE, APP, or ITEM block produces an **artifact**—the final immutable store item, as distinct from a temporary builder workspace. References point only backward, so the file is a graph that can be understood from top to bottom without an implicit starting filesystem.");
    doc.para("Here is the rule in five lines on each side. The Dockerfile column repeatedly changes one implicit build filesystem; the Cixfile column names the temporary `make` tree and then copies one explicit result into `output`.");
    doc.para("| Dockerfile (five lines) | Cixfile (five lines) |\n| --- | --- |\n| `FROM alpine:3.22` | `BUILDER make` |\n| `WORKDIR /work` | `COPY message .` |\n| `COPY message .` | `RUN tr a-z A-Z < message > result` |\n| `RUN tr a-z A-Z < message > result` | `ITEM output` |\n| `RUN chmod 0444 result` | `COPY ${make}/result /result` |");
    doc.para("In the full example, `FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs` is resolved to the immutable revision recorded in `Cixfile.lock`; it supplies packages, not a mutable base filesystem. `FROM . AS src` names this Cixfile's directory, and `${src}/index.html` therefore means that checked-in file. Bare `COPY index.html …` is the deliberate shorthand for the same local source.");
    let source = ["Cixfile", "index.html", "service.conf"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("FROM . AS src"));
    assert!(source.contains("SERVICE guide-site"));
    assert!(source.contains("COPY index.html /srv/guide-site/index.html"));
    assert!(source.contains("ENV API_TOKEN required"));
    assert!(source.contains("PORT dns = udp:5353"));
    assert!(source.contains("CLAIM egress"));

    doc.para("## IMPORT and the store-aware copy rule (CIP-91)");
    doc.para("`IMPORT` unions each package's `bin`, `etc`, and `share` trees at those same destinations in the item; paths outside those trees are not imported. Earlier imports win a collision, so coreutils supplies `ls` even though busybox follows it.");
    doc.para("The **provenance** of a COPY source is simply its declared origin: local context, package, FETCH, builder, or another item. Local bytes are **materialized**, meaning an ordinary real copy like Docker's `COPY`. Store-backed sources normally become symbolic links whose targets are immutable `/nix/store` paths; the item's Nix closure records those targets so copying the closure to another machine brings every runtime dependency too.");
    doc.para("A writable runtime mount cannot be placed below a symlinked directory: the mount namespace needs a real ancestor directory. The store-aware copy rule (CIP-91) therefore copies the exact `/opt/nginx` subtree because `STATEDIR /opt/nginx/state` sits below it, while the unrelated `printf` file remains a link.");
    let built = doc.sh(
        "item=$(cix build .#guide-site); printf '%s\\n' \"$item\"",
        true,
    );
    let store_path = built
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected guide-site build printed an item")
        .to_owned();
    let linked = doc.sh_with_env(
        "ls -l \"$item/opt/tools/printf\"",
        &[("item", &store_path)],
        true,
    );
    assert!(linked.contains(" -> /nix/store/"), "{linked}");
    let materialized = doc.sh_with_env(
        "test ! -L \"$item/opt/nginx\" && printf 'opt/nginx is a materialized directory\\n'",
        &[("item", &store_path)],
        true,
    );
    assert_eq!(materialized.trim(), "opt/nginx is a materialized directory");

    doc.para("`FILE` creates the small interpolated `build-origin` file below. It is useful when the content genuinely needs a binder value; for ordinary configuration it is a smell, because a checked-in file plus `COPY` stays easier to lint, edit, and test.");
    let generated = doc.show_file(Path::new(&store_path).join("etc/guide-site/build-origin"));
    assert!(generated.contains("packages=/nix/store/") && generated.contains("-coreutils-"));

    doc.para("## Runtime declarations are grants");
    doc.para("`ENV SITE_NAME = guide` supplies a default. `ENV API_TOKEN required` names a required non-secret operator value: direct run supplies it as `cix run \"$item\" -e API_TOKEN=example`, while a compose child uses `\"env\": {\"API_TOKEN\": \"example\"}`. Secret values instead use `SECRET` and the credential-file mechanism described below.");
    doc.para("Role directories use the application's native absolute paths. Systemd creates unit-scoped backing below the host's state, cache, log, configuration, and runtime roots and binds it to the declared path: state survives until explicit purge, cache is expendable, logs are retained until cleaning policy removes them, writable config is operator-managed, and run data disappears on stop. An operator can replace a declared role with existing content using `--dir /etc/guide-site=host:/srv/guide-config --identity guide-site`; compose places the same `host:` materialization in the child's `dirs` map. For a compose named `stack`, `cix clean stack --what cache` removes only expendable cache, while `cix down stack --purge --yes` explicitly removes cix-owned state and shared data; host-backed `DIR` data is never deleted.");
    doc.para("A bare port is TCP; the `udp:` prefix is the single UDP spelling. `LISTENER admin` declares no address: the operator assigns one with `-p admin=127.0.0.1:8420`, systemd owns that TCP socket, and the process receives file descriptor 3 with `LISTEN_FDS=1` and `LISTEN_FDNAMES=admin`. Compose publishes a named listener in Chapter 6.");
    doc.para("Claims form a closed vocabulary: `egress` permits outbound networking, `jit` drops `MemoryDenyWriteExecute=`, `gpu` opens the `/dev/dri` class, and `device /dev/name` opens exactly one device. Without egress the compiler uses a private or deny-by-default network; without jit writable executable memory stays denied. These declarations still describe the intended unit under `--user`, but an incapable user manager may emit the degradation marker taught in Chapter 1.");
    let manifest = doc.sh_with_env(
        "jq '{env, ports, listeners, dirs, claims}' \"$item/cix-manifest.json\"",
        &[("item", &store_path)],
        true,
    );
    assert!(manifest.contains("\"udp\""));
    assert!(manifest.contains("\"admin\""));
    assert!(manifest.contains("\"required\": true"));
    assert!(manifest.contains("\"egress\""));
    assert!(manifest.contains("\"jit\""));

    doc.para("## The remaining runtime grammar");
    doc.para("`START` is the main argv. `START_PRE` is run before every initial start and restart, so it must be safe to repeat after a partial attempt. `SERVICE` stays running; `APP` is a systemd oneshot whose exit status is the result; `ITEM` is only a store tree with no manifest, so it can be copied from or tagged but not run.");
    doc.para("`SECRET db-password AS DB_PASSWORD_FILE` declares a credential need without a value. Compose supplies `\"secrets\": {\"db-password\": {\"file\": \"/etc/cix/db-password\"}}`; systemd mounts the root-owned source at `$CREDENTIALS_DIRECTORY/db-password` and sets `DB_PASSWORD_FILE` to that path. `DIR /media:ro` instead declares pre-existing operator data: cix neither creates nor deletes it, and the operator maps it with a `host:`, `shared:`, or role alias materialization.");
    doc.para("Health declarations have one complete shape: `READINESS http :8080/healthz IN 30s` waits up to 30 seconds for the first successful HTTP response before startup succeeds, while `LIVENESS tcp 127.0.0.1:8080 EVERY 10s` probes repeatedly and gives systemd a three-interval watchdog window before restart. `notify` replaces the protocol and target when the program speaks systemd notify itself. `SHM 64M` creates a private `/dev/shm` tmpfs with that size limit.");

    doc.para("## Directive reference");
    doc.para("| Declaration | What it adds |\n| --- | --- |\n| `FROM … AS name` | A package/source/item binder pinned in `Cixfile.lock`; `FROM .` names unpinned local context. |\n| `FETCH name command … EXPECT hash` | The only networked step; it binds pinned downloaded output. |\n| `BUILDER name` | A reusable workspace under `~/.cache/cix/workspaces` by default; delete that cache to reclaim it without changing correctness. |\n| `SERVICE` / `APP` / `ITEM` | A long-running unit / finite oneshot / non-runnable store tree. |\n| `IMPORT package…` | An earlier-wins read-only package union with bare command lookup. |\n| `COPY source /destination` | Store-aware item assembly; builder destinations are workspace-relative. |\n| `FILE /destination <<EOF` | An inline interpolated file; prefer checked-in files when possible. |\n| `START` / `START_PRE` | Main argv / repeat-safe service pre-start argv. |\n| `ENV` / `SECRET` | Non-secret runtime configuration / compose-supplied credential file. |\n| `PORT` / `LISTENER` | A direct TCP/UDP bind / systemd-owned TCP socket. |\n| role dirs / `DIR` | Cix-owned lifecycle storage / operator-owned data. |\n| `READINESS` / `LIVENESS` | Startup gate / watchdog restart probe. |\n| `CLAIM` / `SHM` | A named sandbox exception / size-bounded private tmpfs. |");
    doc.finish()
}

fn chapter_building() -> String {
    let mut doc = Doc::new("building");

    let fetch_demo = doc.base.join("fetch-demo");
    fs::create_dir(&fetch_demo).expect("creating FETCH demo");
    let expected_hash = tree_hash("expected", b"author-pinned");
    fs::write(
        fetch_demo.join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH expected ${{pkgs.coreutils}}/bin/printf author-pinned > expected EXPECT {expected_hash}
FETCH resolved ${{pkgs.coreutils}}/bin/printf lock-pinned > resolved

BUILDER assemble
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{expected}}/expected expected
COPY ${{resolved}}/resolved resolved
RUN cat expected resolved > result

ITEM fetched-result
COPY ${{assemble}}/result /result
"#
        ),
    )
    .expect("writing FETCH demo Cixfile");
    fs::write(fetch_demo.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing FETCH demo lock");

    doc.para("You will pin downloaded inputs, reuse checked build work, replay it from an empty workspace, repair a conventional Linux binary, and compile a two-service Rust project. A **lock** is the checked-in `Cixfile.lock` beside each Cixfile; it records immutable source revisions, download pins, and the evidence needed to validate reusable build steps.");

    doc.para("## FETCH, EXPECT, and deliberate lock movement");
    doc.para("Choose `EXPECT` when an author or upstream release gives you a trusted checksum. Its SRI value is a `sha256-…` integrity fingerprint over the complete serialized output directory; to calculate one for a download you have independently inspected, put the fetched files in one directory and run `nix hash path --sri that-directory`. Without `EXPECT`, `--update-lock` uses trust on first use: cix fetches twice to expose immediate volatility and pins only paths used downstream, but two matching responses do not authenticate a consistently malicious server.");
    doc.para("Read the first line as labelled grammar: in `FETCH expected ${pkgs.coreutils}/bin/printf author-pinned > expected EXPECT sha256-…`, the first `expected` is the new binder name, everything through `> expected` is the network-enabled command and its declared output file, and the final `EXPECT` clause is the author-supplied whole-tree hash. The second FETCH omits that clause, so only an explicit update may create or change its lock pin.");
    let fetch_source = doc.show_file("fetch-demo/Cixfile");
    assert!(fetch_source.contains("FETCH expected"));
    assert!(fetch_source.contains("EXPECT sha256-"));
    assert!(fetch_source.contains("FETCH resolved"));
    assert!(fetch_source.contains("RUN cat expected resolved > result"));

    doc.para("The tour sets `CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces` only to keep its disposable work below this fixture. You do not need to create it: cix does so automatically, and without the variable it uses the user cache at `~/.cache/cix/workspaces`.");
    doc.para("A **memo** is a recorded build-step result keyed by the command, imports, environment, and observed inputs. A **build view** is the immutable `/nix/store` snapshot of the memo's output that later COPY steps consume; a miss executes the step and writes a view, while a hit reuses the prior view after rechecking its inputs. `--update-lock resolved` permits the network for that named FETCH, rewrites `fetch-demo/Cixfile.lock`, and should be followed by committing that lock.");
    let updated = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --update-lock resolved fetch-demo",
        true,
    );
    assert!(updated.contains("BUILDER assemble memo miss"), "{updated}");
    let result_path = built_store_path(&updated, "-cix-item-fetched-result");
    let combined = doc.show_file(Path::new(&result_path).join("result"));
    assert_eq!(combined.trim(), "author-pinnedlock-pinned");

    doc.para("The lock is written beside the Cixfile, not in the workspace. It keeps the immutable nixpkgs revision, each FETCH pin, step memos, consumed output objects, and a **development-environment snapshot**: the complete set of environment variables derived together from one builder's imported package universe. That is why compiler-related values such as `PKG_CONFIG_PATH` arrive as one pinned set instead of hand-wired host paths; inspect the full records with `jq '.devEnvs' fetch-demo/Cixfile.lock`.");
    let lock = doc.sh(
        "jq '{fetches, devEnvCount:(.devEnvs | length)}' fetch-demo/Cixfile.lock",
        true,
    );
    assert!(lock.contains("\"expected\""));
    assert!(lock.contains("\"resolved\""));
    assert!(lock.contains("\"devEnvCount\": 1"));

    doc.para("FETCH alone has network authority. RUN executes in a bubblewrap sandbox: a temporary filesystem and namespace containing only the declared packages and workspace, with networking removed. Cix follows the command and all subprocesses with `strace` until they exit, recording file opens, metadata checks, directory listings, missing paths, and writes; the command, imports, environment, and that observed read set form the reusable step identity.");
    let warm = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --stats fetch-demo",
        true,
    );
    assert!(warm.contains("memo-hit"), "{warm}");
    assert!(warm.contains("\"nixSubprocesses\":0"), "{warm}");
    doc.para("That hit is not a timestamp promise. For example, `RUN cat expected resolved > result` records reads of those two files: changing either produces a miss, while adding an unread `notes` file does not. Directory enumeration, metadata-only probes, and an absent file are fingerprinted too; nondeterministic reads therefore cause a miss or a warm-versus-cold audit error instead of becoming invisible state. A persistent workspace is an acceleration structure, not hidden build input.");

    doc.para("`--update-lock` and `--cold` are the audit pair. The first permits the network, writes the pin to `Cixfile.lock`, stores fetched bytes as a Nix store snapshot, and records a receipt below `~/.cache/cix/fetch-snapshots`. `--cold` creates an empty builder workspace, never contacts the network, replays those pinned bytes from the local snapshot cache, and compares the new reads and outputs. A different machine must first perform an ordinary pin-verifying build or receive that cached store closure; if the receipt or store snapshot was garbage-collected, cold replay refuses and tells you to repopulate it rather than refetching silently.");
    let cold = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build --cold fetch-demo",
        true,
    );
    assert!(cold.contains("BUILDER assemble"), "{cold}");
    assert_eq!(
        built_store_path(&cold, "-cix-item-fetched-result"),
        result_path
    );

    doc.para("## The FHS diagnostic, then the one-line fix");
    let fhs_fixture = fhs_elf_fixture();
    let fhs_bytes = fs::read(fhs_fixture.join("fhs-probe")).expect("reading FHS probe");
    let fhs_hash = tree_hash("fhs-probe", &fhs_bytes);
    let listen = next_listen();
    let _server = start_file_server(&doc, &fhs_fixture, &listen);
    let fhs_demo = doc.base.join("fhs-demo");
    fs::create_dir(&fhs_demo).expect("creating FHS demo");
    fs::write(
        fhs_demo.join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

FETCH native ${{pkgs.curl}}/bin/curl -fsS http://{listen}/fhs-probe -o fhs-probe EXPECT {fhs_hash}

BUILDER native-build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{native}}/fhs-probe .
RUN chmod +x fhs-probe
RUN ./fhs-probe > result

ITEM native-result
COPY ${{native-build}}/result /result
"#
        ),
    )
    .expect("writing FHS demo Cixfile");
    fs::write(fhs_demo.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing FHS demo lock");

    doc.para("The tour harness serves the next fixture URL from a temporary local HTTP server; in your Cixfile substitute any real URL and its independently obtained EXPECT hash. The downloaded ELF is a conventional Linux executable whose header demands the fixed loader path `/lib64/ld-linux-x86-64.so.2`. Nix normally keeps that loader under glibc's unique store path, so merely having the executable bytes is insufficient. The builder imports a shell and core utilities but no libc, and the real trace produces the failure below.");
    let fhs_source = doc.show_file("fhs-demo/Cixfile");
    assert!(fhs_source.contains("http://"));
    assert!(!fhs_source.contains("pkgs.glibc"));
    let missing = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo",
        false,
    );
    assert!(
        missing.contains("fhs-probe requires the FHS loader"),
        "{missing}"
    );
    assert!(missing.contains("/lib64/ld-linux-x86-64.so.2"), "{missing}");
    assert!(missing.contains("IMPORT ${pkgs.glibc}"), "{missing}");

    doc.para("Add glibc to the ordered IMPORT union. In the builder sandbox, that import mounts glibc's loader at the conventional `/lib64/ld-linux-x86-64.so.2` alias and offers its library closure; the same bytes now run without mutation or a patchelf step.");
    doc.sh(
        "sed -i 's/${pkgs.coreutils}/${pkgs.coreutils} ${pkgs.glibc}/' fhs-demo/Cixfile",
        true,
    );
    let repaired_source = doc.sh("grep '^IMPORT' fhs-demo/Cixfile", true);
    assert!(repaired_source.contains("${pkgs.glibc}"));
    let repaired = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build fhs-demo",
        true,
    );
    let repaired_path = built_store_path(&repaired, "-cix-item-native-result");
    let fhs_result = doc.show_file(Path::new(&repaired_path).join("result"));
    assert_eq!(fhs_result.trim(), "fhs-tour-ok");

    doc.para("## Capstone: one Rust workspace, two services");
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/build/proj1");
    for relative in PROJ1_FILES {
        let destination = doc.base.join("proj1").join(relative);
        fs::create_dir_all(destination.parent().expect("project file has a parent"))
            .expect("creating proj1 directory");
        fs::copy(source.join(relative), destination).expect("copying proj1 fixture");
    }
    doc.para("The capstone is a complete small Cargo workspace copied from this repository's `examples/build/proj1`. `Cargo.toml` declares the three local members, `Cargo.lock` pins their dependency graph, and the source tree is shown below. There are no registry dependencies in this fixture, so `--offline` needs no unseen vendor directory; cargo, rustc, gcc, and coreutils come from the nixpkgs revision shown in `Cixfile.lock`.");
    let proj1_source = [
        "proj1/Cixfile",
        "proj1/Cixfile.lock",
        "proj1/rust/Cargo.toml",
        "proj1/rust/Cargo.lock",
    ]
    .map(|path| doc.show_file(path))
    .join("");
    assert!(proj1_source.contains("cargo build --release --locked --offline --workspace"));
    assert!(proj1_source.contains("SERVICE proj1-api"));
    assert!(proj1_source.contains("SERVICE proj1-worker"));
    let layout = doc.sh(
        "find proj1/rust -type f -not -path '*/target/*' | sort",
        true,
    );
    assert!(layout.contains("proj1/rust/api/src/main.rs"));
    assert!(layout.contains("proj1/rust/worker/src/main.rs"));
    assert!(layout.contains("proj1/rust/common/src/lib.rs"));

    doc.para("The positional `proj1` chooses that directory. `--namespace proj1` supplies the slash-grouped tag family, and `-t v1` tags both declared members, yielding `proj1/proj1-api:v1` and `proj1/proj1-worker:v1`. A `directory#member` selector instead builds one final member and its backward dependency slice; it cannot be combined with family tagging.");
    let first = doc.sh(
        "items=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1 --namespace proj1 -t v1); api_v1=$(printf '%s\\n' \"$items\" | jq -r '.\"proj1-api\"'); worker_v1=$(printf '%s\\n' \"$items\" | jq -r '.\"proj1-worker\"'); printf '%s\\n' \"$items\"",
        true,
    );
    let first_api = proj1_item_path(&first, "proj1-api");
    let first_worker = proj1_item_path(&first, "proj1-worker");
    let refs = doc.sh("cix ls proj1/", true);
    assert!(refs.contains("proj1/proj1-api:v1"), "{refs}");
    assert!(refs.contains("proj1/proj1-worker:v1"), "{refs}");
    let before_worker = doc.sh_with_env(
        "\"$worker_v1/bin/proj1-worker\"",
        &[("worker_v1", &first_worker)],
        true,
    );
    assert_eq!(before_worker.trim(), "hello from proj1-worker");

    doc.para("Now change only the worker and select that member. Cargo reruns because the shared builder staged the changed workspace, but each final SERVICE depends only on the release binary it copied from that workspace.");
    doc.sh(
        "sed -i 's/proj1-worker/proj1-worker-edited/' proj1/rust/worker/src/main.rs",
        true,
    );
    let edited_worker = doc.sh(
        "worker_v2=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-worker); printf '%s\\n' \"$worker_v2\"",
        true,
    );
    let edited_worker = edited_worker
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected worker build printed its item")
        .to_owned();
    assert_ne!(edited_worker, first_worker);
    let worker_receipt = doc.sh_with_env(
        "\"$worker_v2/bin/proj1-worker\"",
        &[("worker_v2", &edited_worker)],
        true,
    );
    assert_eq!(worker_receipt.trim(), "hello from proj1-worker-edited");
    let unchanged_api = doc.sh(
        "api_after=$(CIX_BUILD_WORKSPACE_DIR=$PWD/.workspaces cix build proj1#proj1-api); printf '%s\\n' \"$api_after\"",
        true,
    );
    let unchanged_api = unchanged_api
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("selected API build printed its item")
        .to_owned();
    assert_eq!(unchanged_api, first_api);
    let api_receipt = doc.sh_with_env(
        "cmp -s \"$api_v1\" \"$api_after\" && printf 'API item reused byte-for-byte\\n'",
        &[("api_v1", &first_api), ("api_after", &unchanged_api)],
        true,
    );
    assert_eq!(api_receipt.trim(), "API item reused byte-for-byte");
    doc.para("The worker receipt visibly changes while the API item is byte-for-byte identical. The warm workspace and memo records stay private to the builder—they are acceleration state, not runtime dependencies—while each final item's Nix closure contains only the immutable paths reached by what that member copied.");
    doc.finish()
}

fn chapter_naming_distribution() -> String {
    let mut doc = Doc::new("naming-distribution");
    let publisher = doc.state_dir.clone();
    let consumer = doc.base.join("consumer-state");

    doc.para("You will name immutable items, move and remove those names, then serve one local index and pull from it into another. A **family** is the slash-grouped prefix of related tag names, such as `guide/` in `guide/web:v1`. It is a naming convention except where `cix build --namespace` creates member names and `cix ls guide/` filters them; it does not create an access-control, storage, or distribution boundary.");

    doc.para("## One demystifying aside: an item is a tree");
    doc.para("Normally `cix build` writes this tree for you. At the boundary, however, an item is simply a Nix store tree with `cix-manifest.json`. This hand-written manifest intentionally makes a taggable inspection fixture, not a runnable service: `message` is data rather than an executable. `nix store add` recursively serializes the directory as a Nix archive, copies it to a content-addressed store path, and prints that path; it neither validates the cix manifest nor protects the result from garbage collection.");
    let first = fixture(&mut doc, "my-app-v1", "hello from my app v1");

    doc.para("## Names come after builds");
    doc.para("The store path already has its complete content identity. A tag is a mutable pointer added afterwards, and its source-then-destination syntax is `cix tag <item-or-existing-ref> <new-bare-ref>`. Each local tag is also a **GC root**, a durable reference that keeps the item from Nix garbage collection; cleanup can reclaim the item only after every cix tag and other root is gone. The explicit `:tag` suffix is mandatory—there is no implicit `latest`.");
    doc.sh("cix tag my-app:v1 guide/web:v1", true);
    doc.sh("cix tag my-app:v1 guide/web:stable", true);
    let family = doc.sh("cix ls -l guide/", true);
    assert!(family.contains("guide/web:v1"));
    assert!(family.contains("guide/web:stable"));
    assert!(family.contains(&first));

    let inspected = doc.sh(
        "cix inspect guide/web:v1 | jq '{kind, reference, storePath, systems:(.outputs | keys)}'",
        true,
    );
    assert!(inspected.contains("\"kind\": \"artifact\""));
    assert!(inspected.contains("\"reference\": \"guide/web:v1\""));
    assert!(inspected.contains(&first));
    assert!(inspected.contains(std::env::consts::ARCH));
    doc.para("The inspection word `artifact` means the item-facing side of `cix inspect`, not a fourth Cixfile block kind alongside SERVICE, APP, and ITEM. `systems` comes from the current Nix store platform that `cix tag` records in the index output slot; the hand-written manifest did not declare it.");

    doc.para("Names move without rewriting item bytes. Create the destination pointer and then remove the source with `cix untag`; the old store path remains as long as any other root still reaches it.");
    doc.sh(
        "cix tag guide/web:v1 guide/web:release && cix untag guide/web:stable",
        true,
    );
    let moved_names = doc.sh("cix ls guide/", true);
    assert!(moved_names.contains("guide/web:release"));
    assert!(!moved_names.contains("guide/web:stable"));

    doc.para("Build each new immutable tree before pointing a tag at it. Version 2 differs by one payload line:");
    doc.sh(
        "mkdir my-app-v2 && printf '%s\\n' 'hello from my app v2' > my-app-v2/message && printf '%s\\n' '{\"cixManifest\":0,\"start\":[\"message\"]}' > my-app-v2/cix-manifest.json",
        true,
    );
    doc.show_file("my-app-v2/message");
    let second_added = doc.sh(
        "item_v2=$(nix store add my-app-v2); printf '%s\\n' \"$item_v2\"",
        true,
    );
    let second = second_added
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("nix store add printed the v2 item")
        .to_owned();
    doc.para("Moving `guide/web:v1` to a new build changes only that pointer. The immutable v1 path still exists wherever another root retains it.");
    doc.sh_with_env(
        "cix tag \"$item_v2\" guide/web:v1",
        &[("item_v2", &second)],
        true,
    );
    let moved = doc.sh("cix ls -l guide/", true);
    assert!(moved.contains(&second));
    assert!(moved.contains(&first));

    doc.para("## Serve and pull");
    doc.para("This demo runs publisher and consumer prompts as two logical shells with separate `CIX_STATE_DIR` indexes on one host; they share only the host's Nix daemon/store. On two machines, install Nix and cix on the consumer as in Chapter 1. `cix serve --with-store` exposes the publisher's bare tag database and additionally materializes a standard Nix binary cache containing the referenced closures.");
    doc.para("The qualified-reference grammar is `host:port/family/name:tag`: the host and optional port before the first slash are the origin, the middle slash components are the name, and the final colon introduces the mandatory tag. Name components use lower-case letters, digits, `.`, `_`, and `-`; there is no path escaping or default registry. For the command below, that becomes `127.0.0.1:8420/guide/web:v1`.");
    doc.para("The same ordinary URL is content-negotiated so humans and tools need no parallel API: a browser receives HTML, while cix sends the shown `Accept` header and receives the exact JSON index entry. The index maps the name to a store path; Nix then downloads its **closure**, the item plus every store path it references at runtime.");
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
            "curl -s -H 'Accept: application/vnd.cix+json;version=1' http://{listen}/guide/web:v1 | jq '{{outputs, substituters}}'"
        ),
        true,
    );
    assert!(entry.contains("\"outputs\""));
    assert!(entry.contains(&second));
    assert!(entry.contains("/store"));

    doc.para("This localhost demo is deliberately unsigned: NAR hashes detect corruption, but they do not authenticate who published the bytes. Production adds TLS plus `cix serve --with-store --sign-key /etc/cix/cache.sec`; the corresponding public key must be trusted in the consumer's Nix `trusted-public-keys` configuration and may be advertised in the index entry. Do not infer production trust from the unsigned loopback receipt.");
    doc.para("`--as` adopts the qualified remote ref under a bare local name and stores its upstream origin in tag metadata. The pull copies the selected item closure from the advertised `/store` cache, verifies the recorded NAR hash and any configured signature policy, then creates the consumer's local GC-rooted tag. A later argument-free `cix pull` revisits every recorded upstream and downloads any closure whose tag moved.");
    let pulled = doc.sh_in(
        "consumer $",
        &consumer,
        &format!("cix pull {listen}/guide/web:v1 --as guide/web:v1"),
        true,
    );
    assert!(pulled.contains("updated 1 tag(s)"));
    let local = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(local.contains("guide/web:v1"));
    assert!(local.contains(&second));
    assert!(local.contains(&listen));

    doc.sh_in(
        "publisher $",
        &publisher,
        "mkdir my-app-v3 && printf '%s\\n' 'hello from my app v3' > my-app-v3/message && printf '%s\\n' '{\"cixManifest\":0,\"start\":[\"message\"]}' > my-app-v3/cix-manifest.json",
        true,
    );
    doc.show_file("my-app-v3/message");
    let third_added = doc.sh_in(
        "publisher $",
        &publisher,
        "item_v3=$(nix store add my-app-v3); printf '%s\\n' \"$item_v3\"",
        true,
    );
    let third = third_added
        .lines()
        .find(|line| line.starts_with("/nix/store/"))
        .expect("nix store add printed the v3 item")
        .to_owned();
    let output = doc.run_with_env(
        &publisher,
        "cix tag \"$item_v3\" guide/web:v1",
        &[("item_v3", &third)],
        true,
    );
    let raw = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    doc.record("publisher $", "cix tag \"$item_v3\" guide/web:v1", &raw);
    let refreshed = doc.sh_in("consumer $", &consumer, "cix pull", true);
    assert!(refreshed.contains("updated 1 tag(s)"));
    let updated = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(updated.contains(&third));
    assert!(!updated.contains(&second));
    drop(server);

    doc.para("The positive model is deliberately small: each local cix index stores GC-rooted name-to-path records and optional upstream origins; qualified names select another served index; Nix substitution transfers the complete immutable closure.");
    doc.finish()
}

fn chapter_runtime_contract() -> String {
    let mut doc = Doc::new("runtime-contract");
    fs::write(
        doc.base.join("server.py"),
        r#"from http.server import BaseHTTPRequestHandler, HTTPServer

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"runtime healthy\n"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, format, *args):
        pass

print("runtime service started", flush=True)
HTTPServer(("127.0.0.1", 18086), Handler).serve_forever()
"#,
    )
    .expect("writing runtime HTTP server");
    fs::write(
        doc.base.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

SERVICE web
IMPORT ${pkgs.python3}
COPY server.py /srv/app/server.py
START python3 /srv/app/server.py
PORT http = 18086
STATEDIR /var/lib/runtime-guide
SECRET db-password AS DB_PASSWORD_FILE
READINESS http :18086/healthz IN 10s
LIVENESS http :18086/livez EVERY 2s

APP cleanup
IMPORT ${pkgs.coreutils}
START true

SERVICE observer
IMPORT ${pkgs.coreutils}
START sleep 300
"#,
    )
    .expect("writing runtime Cixfile");
    fs::write(doc.base.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing runtime lock");

    doc.para("You will inspect a tagged HTTP service with health contracts at the honest rootless boundary, then debug and schedule its run-to-completion sibling. Afterwards, you will understand the runtime boundary—immutable world, declared writable state, credential files, health supervision, timers, and journald/accounting observability—including which guarantees require the system-manager VM gate.");

    doc.para("## The item owns needs; the operator owns values");
    doc.para("The web service declares a direct port, persistent application-native state, one credential-file need, and real HTTP readiness and liveness endpoints. The finite APP is eligible for timer scheduling, while the minimal observer service stays alive long enough for scoped observability receipts.");
    let source = ["Cixfile", "server.py"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(source.contains("STATEDIR /var/lib/runtime-guide"));
    assert!(source.contains("COPY server.py /srv/app/server.py"));
    assert!(source.contains("START python3 /srv/app/server.py"));
    assert!(!source.contains("${src}"));
    assert!(source.contains("SECRET db-password AS DB_PASSWORD_FILE"));
    assert!(source.contains("READINESS http :18086/healthz IN 10s"));
    assert!(source.contains("LIVENESS http :18086/livez EVERY 2s"));
    assert!(source.contains("APP cleanup"));
    assert!(source.contains("SERVICE observer"));
    assert!(source.contains("START sleep 300"));
    let built = doc.sh("cix build . --namespace runtime -t v1", true);
    assert!(built.contains("\"web\""));
    assert!(built.contains("\"cleanup\""));
    assert!(built.contains("\"observer\""));
    let web_path = proj1_item_path(&built, "web");

    doc.para("## Inspect the item, then cross the system-manager boundary");
    doc.para("`cix run` resolves the tag and compiles the manifest into a transient unit. Production projects `/srv/app/server.py` from the item before readiness and liveness supervision begins. Because D13 permits a user manager to reject that mount namespace, the rootless receipt parses the copied program through its physical item path instead of claiming a live HTTP service.");
    let parsed = doc.sh(
        &format!(
            "{web_path}/bin/python3 -c 'compile(open(\"{web_path}/srv/app/server.py\").read(), \"server.py\", \"exec\"); print(\"copied server parses\")'"
        ),
        true,
    );
    assert!(parsed.contains("copied server parses"));
    doc.para("`cix debug` still resolves an item by tag and replaces its entrypoint inside the service sandbox. The finite cleanup sibling has no mount or health dependency, so it is the honest rootless target for that receipt.");
    let before_debug = user_cix_units().expect("listing user units before cix debug");
    let debugged = doc.sh("cix debug runtime/cleanup:v1 --user -- true", true);
    assert!(debugged.contains("cix debug --user is degraded"));
    stop_user_units_created_since(&before_debug, "cix-debug-cleanup-", "the cix debug receipt");
    stop_empty_cix_run_slice("the cix debug receipt");

    doc.para("The observer sibling is deliberately small and long-running, so the observability receipts can assert one tour-owned unit. `ps --json` selects that exact unit instead of formatting an ambient table whose widths depend on unrelated units; the `stats` projection keeps the live counters live while asserting their stable manager, composite, and unit identity.");
    let started = doc.sh("cix run runtime/observer:v1 --user --detach", true);
    let observer_unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-observer-") && line.ends_with(".service"))
        .expect("cix run printed an observer unit")
        .to_owned();
    let active = doc.run(
        &doc.state_dir,
        &format!("systemctl --user is-active {observer_unit}"),
        true,
    );
    assert_eq!(String::from_utf8_lossy(&active.stdout).trim(), "active");
    let ps = doc.sh(
        &format!(
            "cix ps --json | jq --arg unit '{observer_unit}' '.[] | select(.unit == $unit) | {{manager, service, unit, state}}'"
        ),
        true,
    );
    assert!(ps.contains("\"manager\": \"user\""));
    assert!(ps.contains("\"service\": \"observer\""));
    assert!(ps.contains(&format!("\"unit\": \"{observer_unit}\"")));
    assert!(ps.contains("\"state\": \"active/running\""));
    stop_user_unit(&observer_unit, "the cix ps receipt");
    stop_empty_cix_run_slice("the cix ps receipt");

    let stats_started = doc.run(
        &doc.state_dir,
        "cix run runtime/observer:v1 --user --detach",
        true,
    );
    let stats_output = format!(
        "{}{}",
        String::from_utf8_lossy(&stats_started.stdout),
        String::from_utf8_lossy(&stats_started.stderr)
    );
    let stats_unit = stats_output
        .lines()
        .find(|line| line.starts_with("cix-run-observer-") && line.ends_with(".service"))
        .expect("cix run printed a stats observer unit");
    let active = doc.run(
        &doc.state_dir,
        &format!("systemctl --user is-active {stats_unit}"),
        true,
    );
    assert_eq!(String::from_utf8_lossy(&active.stdout).trim(), "active");
    let stats = doc.sh(
        &format!("cix stats 2>/dev/null | awk -v unit='{stats_unit}' 'NR == 1 || $3 == unit'"),
        true,
    );
    let mut stats_lines = stats.lines();
    assert_eq!(
        stats_lines.next(),
        Some("MANAGER  COMPOSITE  SERVICE  MEMORY  CPU  TASKS  IO  IP")
    );
    let stats_row = stats_lines
        .next()
        .expect("cix stats printed the observer row");
    assert!(
        stats_lines.next().is_none(),
        "unexpected cix stats rows: {stats}"
    );
    let stats_fields = stats_row.split_whitespace().collect::<Vec<_>>();
    assert!(
        stats_fields.len() >= 8,
        "unexpected cix stats row: {stats_row}"
    );
    assert_eq!(&stats_fields[..3], &["user", "run", stats_unit]);
    stop_user_unit(stats_unit, "the cix stats receipt");
    stop_empty_cix_run_slice("the cix stats receipt");

    let logs = doc.sh("cix logs run/observer --explain", true);
    assert!(logs.contains("journalctl CIX_COMPOSITE=run CIX_SERVICE=observer"));

    doc.para("## The system-manager guarantees");
    doc.para("The ordinary production path runs in a read-only world: in `--closed-root` audit mode even undeclared host paths are absent, while the whole Nix store and the item's projections remain read-only. Only declared role directories are writable. The rootless contract does not guarantee that mount namespace, so the [closed-root audit scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/closedroot-audit.nix) executes the failed undeclared access and sealed-root inventory under the system manager.");
    doc.para("`STATEDIR /var/lib/runtime-guide` survives service restarts and belongs to cix until an explicit purge; the item never chooses a host backing path. `SECRET db-password` similarly names no value: compose supplies a root-owned file, systemd projects it below `$CREDENTIALS_DIRECTORY`, and `DB_PASSWORD_FILE` receives only that path. The [directory lifecycle scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix), [secrets scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/secrets.nix), and [health scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/health.nix) execute persistence, credential rotation, readiness blocking, and liveness restart without faking host privileges here.");

    doc.para("## Schedule the APP");
    doc.para("An APP runs to completion instead of staying active. `--schedule` writes a transient service/timer pair using systemd's `OnCalendar` syntax and prints the timer name; no polling daemon is involved.");
    let scheduled = doc.sh(
        "cix run runtime/cleanup:v1 --user --schedule '*-*-* 00:00:00'",
        true,
    );
    let timer = scheduled
        .lines()
        .find(|line| line.starts_with("cix-run-cleanup-") && line.ends_with(".timer"))
        .expect("scheduled APP printed its timer")
        .to_owned();
    let _timer = ScheduledUserUnit {
        timer: timer.clone(),
    };
    let active = doc.sh(&format!("systemctl --user is-active {timer}"), true);
    assert_eq!(active.trim(), "active");

    doc.para("You now have the complete ownership split: artifacts declare their process needs, compose supplies host policy and secrets, and systemd owns lifecycle, health, logs, timers, and accounting.");
    doc.finish()
}

fn chapter_compose() -> String {
    let mut doc = Doc::new("compose");

    doc.para("You will connect two independently built services with a Unix edge and shared state, validate and diff their compose generation, and exercise the socket-activation primitive beneath named listeners. Afterwards, you will understand compose's resolve/build/activate lifecycle, unary `cix run`, rollback boundary, pod option, and journal namespace without mistaking rootless dry-runs for system activation.");

    doc.para("## Named listeners are systemd sockets");
    listener_fixture(&doc);
    doc.para("A `LISTENER` does not let the process call `socket()` for that port. This canonical Cixfile imports the probe's runtime, copies the checked-in Python script, and declares `LISTENER http`; systemd owns the socket and passes file descriptor 3 to the process.");
    let listener_source = ["listener-fixture/Cixfile", "listener-fixture/listenfds.py"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(listener_source.contains("socket.fromfd(3"));
    assert!(listener_source.contains("COPY listenfds.py /bin/listenfds"));
    assert!(listener_source.contains("LISTENER http"));
    let listener_build = doc.sh("cix build listener-fixture", true);
    let listener_path = built_store_path(&listener_build, "-cix-item-listener-demo");
    let manifest = fs::read_to_string(Path::new(&listener_path).join("cix-manifest.json"))
        .expect("reading built listener manifest");
    assert!(manifest.contains("\"listeners\""));
    let listen = next_listen();
    let started = doc.sh(
        &format!("cix run {listener_path} --user -p http={listen} --detach"),
        true,
    );
    let listener_unit = started
        .lines()
        .find(|line| line.starts_with("cix-run-") && line.ends_with(".service"))
        .expect("cix run printed a listener unit")
        .to_owned();
    let _listener = UserUnit {
        name: listener_unit.clone(),
    };
    wait_for_http(&listen, "LISTEN_FDS=1; no socket() authority");
    let response = doc.sh(&format!("curl -fsS http://{listen}"), true);
    assert_eq!(response.trim(), "LISTEN_FDS=1; no socket() authority");
    doc.sh(&format!("systemctl --user stop {listener_unit}"), true);

    doc.para("## Two items, one operator document");
    for (name, extra) in [("producer", "RUNDIR /run/producer\n"), ("consumer", "")] {
        let directory = doc.base.join(name);
        fs::create_dir(&directory).expect("creating compose member directory");
        fs::write(
            directory.join("Cixfile"),
            format!(
                "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\n\nSERVICE {name}\nIMPORT ${{pkgs.coreutils}}\nSTART sleep 300\nENV VERSION = v1\nSTATEDIR /var/lib/shared\n{extra}"
            ),
        )
        .expect("writing compose member Cixfile");
        fs::write(directory.join("Cixfile.lock"), TOUR_CIXFILE_LOCK)
            .expect("writing compose member lock");
    }
    let members = ["producer/Cixfile", "consumer/Cixfile"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(members.contains("RUNDIR /run/producer"));
    assert_eq!(members.matches("STATEDIR /var/lib/shared").count(), 2);
    let producer_build = doc.sh("cix build producer -t current", true);
    let first_producer = built_store_path(&producer_build, "-cix-item-producer");
    let consumer_build = doc.sh("cix build consumer -t v1", true);
    let consumer = built_store_path(&consumer_build, "-cix-item-consumer");

    fs::write(
        doc.base.join("compose.json"),
        r#"{
  "cixCompose": 1,
  "name": "tour-stack",
  "logNamespace": true,
  "children": {
    "producer": {
      "item": "producer:current",
      "update": "track",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    },
    "consumer": {
      "item": "consumer:v1",
      "dirs": {"/var/lib/shared": {"shared": "payload"}}
    }
  },
  "edges": {
    "producer-api": {
      "producer": {"child": "producer", "path": "/run/producer"},
      "consumers": {"consumer": {}}
    }
  }
}
"#,
    )
    .expect("writing compose fixture");
    doc.para("The compose file owns host policy rather than rebuilding either item. Both members opt the same declared STATEDIR into compose-local shared backing, while the edge projects the producer's `/run/producer` Unix surface into the consumer and orders startup structurally.");
    let compose = doc.show_file("compose.json");
    assert!(compose.contains("\"shared\": \"payload\""));
    assert!(compose.contains("\"producer-api\""));
    assert!(compose.contains("\"logNamespace\": true"));
    let checked = doc.sh("cix compose check compose.json", true);
    assert_eq!(
        checked.trim(),
        "compose tour-stack: 2 services, 1 edges, valid"
    );

    write_resolved_compose_lock_entries(
        &doc,
        &doc.base.join("compose.json"),
        &[
            ("producer", "producer:current"),
            ("consumer", "consumer:v1"),
        ],
    );
    let lock = doc.show_file("cix.lock");
    assert!(lock.contains(&first_producer));
    assert!(lock.contains(&consumer));
    let initial = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(
        initial.contains("cix-tour\x2dstack-producer.service"),
        "{initial}"
    );
    assert!(
        initial.contains("cix-tour\x2dstack-consumer.service"),
        "{initial}"
    );

    doc.para("`cix run` is the unary form of the same contract compiler. It gives one item a transient lifecycle; compose adds stable names, edges, shared backing, operator values, and retained generations.");
    let unary = doc.sh("cix run producer:current --user --detach", true);
    let unary_unit = unary
        .lines()
        .find(|line| line.starts_with("cix-run-producer-") && line.ends_with(".service"))
        .expect("unary run printed a producer unit")
        .to_owned();
    let _unary = UserUnit {
        name: unary_unit.clone(),
    };
    doc.sh(&format!("systemctl --user stop {unary_unit}"), true);

    doc.para("Change only the tracked producer item. The dry diff resolves its moved tag and builds a candidate generation without touching the active system manager.");
    doc.sh(
        "sed -i 's/ENV VERSION = v1/ENV VERSION = v2/' producer/Cixfile",
        true,
    );
    let producer_v2 = doc.sh("cix build producer -t current", true);
    let second_producer = built_store_path(&producer_v2, "-cix-item-producer");
    assert_ne!(first_producer, second_producer);
    let changed = doc.sh_after_warming("cix compose diff compose.json", true);
    assert!(changed.contains(&second_producer), "{changed}");

    doc.para("## Activation is the privileged receipt");
    doc.para("This harness intentionally stops at `check` and `diff`: `cix up compose.json`, `cix rollback tour-stack`, and `cix down tour-stack` manage `/etc/systemd/system`, a root profile, shared backing ownership, and the system manager. The [stack VM scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/lib.nix) executes that exact up → selective change → diff → rollback → down lifecycle, and [the dirs scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/dirs2.nix) asserts both writers see the same setgid shared directory.");
    doc.para("`network: \"pod\"` places a subtree in one private network namespace; named networks and service-DNS policy stay separate concerns. The [network-namespace scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/netns.nix) proves pod co-location, isolation, publication, and cleanup. `logNamespace: true` similarly asks systemd for one journal namespace for this compose tree; `cix logs tour-stack[/child]` selects its stamped fields, with the [observability scenario](https://github.com/mathijshenquet/composix/blob/main/nix/scenarios/observability.nix) carrying the privileged receipt.");
    doc.finish()
}

fn chapter_dev_loop() -> String {
    let mut doc = Doc::new("dev-loop");

    doc.para("You will keep one artifact rebuilding through an edit, then build faithful and dissolved Docker translations side by side with independent locks. Afterwards, you will understand where `cix watch` fits in the development loop, how `--file` preserves honest alternatives, and where to continue when migrating real Docker projects.");

    doc.para("## Watch the artifact, not a mutable container");
    let cache_root = std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME is set for the tour")
        .join(".cache/cix/tmp");
    fs::create_dir_all(&cache_root).expect("creating cix cache temp root");
    let watch_temp = tempfile::Builder::new()
        .prefix("tour-watch-")
        .tempdir_in(&cache_root)
        .expect("creating watch fixture outside ignored target paths");
    let watch_dir = watch_temp.path().to_owned();
    fs::create_dir(watch_dir.join(".git")).expect("marking watch fixture as its own repository");
    std::os::unix::fs::symlink(&watch_dir, doc.base.join("watch-app"))
        .expect("linking watch fixture into the tour story");
    fs::write(watch_dir.join("message"), "first\n").expect("writing watch message");
    fs::write(
        watch_dir.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

ITEM watched
COPY message /message
"#,
    )
    .expect("writing watch Cixfile");
    fs::write(watch_dir.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing watch lock");
    let watch_source = ["watch-app/Cixfile", "watch-app/message"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(watch_source.contains("ITEM watched"));
    assert!(watch_source.contains("first"));

    let mut path = doc.bin_dir.display().to_string();
    if let Some(existing) = std::env::var_os("PATH") {
        path.push(':');
        path.push_str(&existing.to_string_lossy());
    }
    let mut watcher = Command::new(doc.bin_dir.join("cix"))
        .args(["watch", "watch-app"])
        .current_dir(&doc.base)
        .env("CIX_STATE_DIR", &doc.state_dir)
        .env(
            "CIX_BUILD_WORKSPACE_DIR",
            doc.base.join(".watch-workspaces"),
        )
        .env("CIX_WATCH_DEBOUNCE_MS", "30")
        .env("PATH", path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("starting cix watch");
    let stdout = watcher.stdout.take().expect("watch stdout");
    let (lines_sender, lines) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let _ = lines_sender.send(line.expect("watch stdout line"));
        }
    });
    let stderr = watcher.stderr.take().expect("watch stderr");
    let (errors_sender, errors) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let _ = errors_sender.send(line.expect("watch stderr line"));
        }
    });
    let watching = errors
        .recv_timeout(Duration::from_secs(5))
        .expect("watcher announces its root");
    assert!(watching.starts_with("watching "));
    doc.background("$", "cix watch watch-app");
    doc.output(&watching.replace(
        watch_dir.to_string_lossy().as_ref(),
        doc.base.join("watch-app").to_string_lossy().as_ref(),
    ));
    std::thread::sleep(Duration::from_millis(100));
    doc.sh("printf 'changed\\n' > watch-app/message", true);
    let rebuilt = match lines.recv_timeout(Duration::from_secs(60)) {
        Ok(line) => line,
        Err(error) => {
            unsafe {
                libc::kill(watcher.id() as i32, libc::SIGINT);
            }
            let _ = watcher.wait();
            panic!(
                "edited watch context should rebuild ({error}); watcher stderr: {:?}",
                errors.try_iter().collect::<Vec<_>>()
            );
        }
    };
    assert!(rebuilt.starts_with("/nix/store/"), "{rebuilt}");
    doc.output(&rebuilt);
    unsafe {
        libc::kill(watcher.id() as i32, libc::SIGINT);
    }
    assert!(watcher.wait().expect("waiting for cix watch").success());
    assert!(errors.try_iter().next().is_none());
    drop(watch_temp);

    doc.para("The watcher coalesces edit bursts, warm-builds the affected Cixfile, and prints the new item. It ignores `.git`, `target`, Cixfile locks, its own workspaces, and gitignored paths, so its outputs do not trigger loops. In a directory with `compose.json`, the same outer loop selectively restarts only services whose rebuilt item changed; framework hot reload stays in `nix develop`.");

    doc.para("## Keep faithful and dissolved translations together");
    let twins = doc.base.join("twins");
    fs::create_dir(&twins).expect("creating translation twins");
    fs::write(twins.join("payload"), "same runtime payload\n").expect("writing twin payload");
    fs::write(
        twins.join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src

BUILDER faithful-build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/payload .
RUN cp payload result

ITEM translation
COPY ${faithful-build}/result /payload
"#,
    )
    .expect("writing faithful Cixfile");
    fs::write(twins.join("Cixfile.lock"), TOUR_CIXFILE_LOCK).expect("writing faithful lock");
    fs::write(
        twins.join("Cixfile.dissolved"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs

ITEM translation
COPY payload /payload
"#,
    )
    .expect("writing dissolved Cixfile");
    fs::write(twins.join("Cixfile.dissolved.lock"), TOUR_CIXFILE_LOCK)
        .expect("writing dissolved lock");

    doc.para("A faithful twin preserves upstream build choreography when that behavior matters; a dissolved twin selects the nix-native result directly when the ceremony adds no contract. `--file` chooses one without renaming files or mixing trust state, and each Cixfile writes its own sibling lock.");
    let twin_source = ["twins/Cixfile", "twins/Cixfile.dissolved"]
        .map(|path| doc.show_file(path))
        .join("");
    assert!(twin_source.contains("BUILDER faithful-build"));
    assert!(twin_source.contains("ITEM translation\nCOPY payload /payload"));
    let faithful = doc.sh(
        "CIX_BUILD_WORKSPACE_DIR=$PWD/.twin-workspaces cix build twins",
        true,
    );
    let faithful_path = built_store_path(&faithful, "-cix-item-translation");
    let dissolved = doc.sh("cix build --file Cixfile.dissolved twins", true);
    let dissolved_path = built_store_path(&dissolved, "-cix-item-translation");
    assert_eq!(
        fs::read_to_string(Path::new(&faithful_path).join("payload"))
            .expect("reading faithful payload"),
        fs::read_to_string(Path::new(&dissolved_path).join("payload"))
            .expect("reading dissolved payload")
    );
    let locks = doc.sh("ls -1 twins/Cixfile*.lock", true);
    assert!(locks.contains("twins/Cixfile.lock"));
    assert!(locks.contains("twins/Cixfile.dissolved.lock"));

    doc.para("## Continue with real migrations");
    doc.para("Use the [Docker-to-Cixfile translation guide](../migrate.html) for directive-by-directive choices and the faithful-versus-dissolved decision. Browse the [migration corpus gallery](../corpus/index.html) for worked pairs, source context, receipts, and explicit remaining gaps. Start with behavior you can probe, keep every FETCH pinned, and let the resulting Cixfile become the same artifact contract you have just watched and built here.");
    doc.finish()
}

fn built_store_path(output: &str, suffix: &str) -> String {
    build_member_map(output)
        .into_values()
        .find(|path| path.ends_with(suffix))
        .unwrap_or_else(|| panic!("build did not print an item ending in {suffix:?}:\n{output}"))
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
    "> Auto-generated by `cargo test --test tour -- --ignored generate_tour`.\n> Every command below really ran, in a throwaway cix index (the local\n> database mapping names to store paths) so nothing from the generating\n> machine leaks in. Outputs are asserted; store-path ellipses appear in\n> outputs only, never in commands you are meant to type.\n"
        .to_owned()
}

fn render_index(scenarios: &[Scenario]) -> String {
    let mut index = format!(
        "# composix — new-user guide\n\n{}\nComposix builds and runs services the way Nix builds software: a\nbuild produces an immutable directory in the Nix store (an \"item\"),\nand running it asks systemd to start a locked-down unit whose\nfilesystem is assembled from that item. If you know Docker: items\nplay the role of images, units of containers, the Cixfile of the\nDockerfile. If you have never used Docker: you need nothing from it —\na Cixfile is a small declaration of what goes into the item and what\nthe process may touch at runtime.\n\nThe local index is the per-user database that maps mutable names to immutable item paths. Each generated chapter uses a fresh index, so its names cannot read or alter your normal cix state. Start at [Chapter 1](01-hello-composix.html) and follow the guide in order.\n\n## Chapters\n",
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
            description: "Build, run, request, and stop one service without root privileges.",
            body: chapter_hello(),
        },
        Scenario {
            filename: "02-cixfile-language.md",
            title: "Chapter 2: The Cixfile language",
            description:
                "Learn how declarations name inputs, assemble files, and grant runtime capabilities.",
            body: chapter_cixfile_language(),
        },
        Scenario {
            filename: "03-building.md",
            title: "Chapter 3: Building: BUILDERs, FETCH, and the lock",
            description:
                "Pin downloaded bytes, reuse checked build work, repair a Linux binary, and build proj1.",
            body: chapter_building(),
        },
        Scenario {
            filename: "04-naming-distribution.md",
            title: "Chapter 4: Naming and distribution",
            description:
                "Group slash-prefixed names, serve their store contents, and refresh a name that moved.",
            body: chapter_naming_distribution(),
        },
        Scenario {
            filename: "05-runtime-contract.md",
            title: "Chapter 5: Running: the runtime contract",
            description:
                "Run and probe a service, inspect its systemd state, and schedule a finite command.",
            body: chapter_runtime_contract(),
        },
        Scenario {
            filename: "06-compose.md",
            title: "Chapter 6: Compose",
            description: "Connect producer and consumer paths, share writable data, and inspect deployment generations.",
            body: chapter_compose(),
        },
        Scenario {
            filename: "07-dev-loop-docker.md",
            title: "Chapter 7: The dev loop and coming from Docker",
            description: "Rebuild after edits, compare two migration strategies, and browse worked migrations.",
            body: chapter_dev_loop(),
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
fn displayed_files_use_their_source_language() {
    for (path, expected) in [
        ("Cixfile", "dockerfile"),
        ("Cixfile.dissolved", "dockerfile"),
        ("Cixfile.lock", "json"),
        ("overlay.nix", "nix"),
        ("nginx.conf", "nginx"),
        ("server.py", "python"),
        ("compose.json", "json"),
        ("index.html", "html"),
        ("message", ""),
    ] {
        assert_eq!(file_language(Path::new(path)), expected, "{path}");
    }
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
