//! Executed local-index scenarios that generate `docs/tour/`.
//!
//! Run `cargo test --test tour -- --ignored generate_tour` to update the documents.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use cix_common::Ref;
use cix_test_support::{assert_generated_matches, write_generated_atomically, GeneratedFile};
use regex::Regex;

mod tour_scenarios;

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
// Every renderer gets a distinct ephemeral port because parallel test processes
// can otherwise bind the same listener while producing independent documents.
static NEXT_TOUR_PORT: AtomicU16 = AtomicU16::new(10_000);
// The fixed-port chapters share a user manager inside this test process.
static TOUR_RENDER_LOCK: Mutex<()> = Mutex::new(());
// All renderers need the same immutable helper: probe subprocesses run inside
// ProtectHome and therefore cannot execute the workspace-linked test binary.
static TOUR_RUNTIME_HELPER: OnceLock<PathBuf> = OnceLock::new();

fn tour_runtime_helper() -> &'static PathBuf {
    TOUR_RUNTIME_HELPER.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .expect("cix crate has a workspace root");
        let output = Command::new("nix")
            .args(["build", "--no-link", "--print-out-paths", ".#cix"])
            .current_dir(root)
            .output()
            .expect("building the store-backed cix probe helper");
        assert!(
            output.status.success(),
            "building the store-backed cix probe helper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let output = String::from_utf8(output.stdout)
            .expect("store-backed cix helper path is UTF-8")
            .trim()
            .to_owned();
        PathBuf::from(output).join("bin/cix")
    })
}

fn lock_tour_host_resources() -> fs::File {
    let lock_path = std::env::temp_dir().join("cix-tour-render.lock");
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .unwrap_or_else(|error| panic!("opening {}: {error}", lock_path.display()));
    // Independent cargo test processes share the user manager and fixed listener ports.
    let result = unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) };
    assert_eq!(
        result,
        0,
        "locking {}: {}",
        lock_path.display(),
        std::io::Error::last_os_error()
    );
    lock
}

struct Server {
    child: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
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
    let deadline = Instant::now() + Duration::from_secs(45);
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
        // A unit that exited failed (seen on older CI managers) stays loaded until
        // reset-failed; issue it idempotently so failed and clean exits both unload.
        for unit in &remaining {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "reset-failed", unit])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
        }
        // `systemd-run --collect` unloads asynchronously after stop. Waiting here keeps the
        // next tour receipt from observing a unit created by the preceding receipt.
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn stop_user_units_created_since(before: &BTreeSet<String>, prefix: &str, receipt: &str) {
    let after =
        user_cix_units().unwrap_or_else(|error| panic!("listing units after {receipt}: {error}"));
    let created = created_cix_units(before, &after, prefix);
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

fn created_cix_units(
    before: &BTreeSet<String>,
    after: &BTreeSet<String>,
    prefix: &str,
) -> Vec<String> {
    let mut created = after
        .difference(before)
        .filter(|unit| unit.starts_with(prefix))
        .cloned()
        .collect::<Vec<_>>();
    created.sort_by_key(|unit| unit.ends_with(".slice"));
    created
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
            .env("CIX_RUNTIME_HELPER", tour_runtime_helper())
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
    if filename.starts_with("Cixfile") && filename.ends_with(".lock")
        || matches!(filename, "cix.lock" | "flake.lock")
    {
        return "json";
    }
    if filename == "Cargo.lock" {
        return "toml";
    }
    if filename.starts_with("Cixfile") || filename == "Dockerfile" {
        return "dockerfile";
    }
    if filename == "start" {
        return "sh";
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
    let edge_temp = Regex::new(r"cix-tour-edge-[A-Za-z0-9]{6}/")
        .expect("valid compose edge temporary-directory regex");
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
    let normalized = edge_temp.replace_all(&normalized, "cix-tour-edge-TMP/");
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
            "mkdir {name} && printf '%s\\n' '{contents}' > {name}/message && jq -n '{{ cixManifest: 0, start: [\"message\"] }}' > {name}/cix-manifest.json"
        ),
        true,
    );
    doc.sh_in(prompt, state_dir, &format!("ls -1 {name}"), true);
    doc.show_file(format!("{name}/message"));
    let manifest = doc.show_file(format!("{name}/cix-manifest.json"));
    assert!(manifest.contains("\"cixManifest\": 0"));
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
    let json_start = output
        .find('{')
        .unwrap_or_else(|| panic!("build did not print a JSON member map:\n{output}"));
    serde_json::Deserializer::from_str(&output[json_start..])
        .into_iter::<std::collections::BTreeMap<String, String>>()
        .next()
        .expect("a JSON member map starts with an object")
        .unwrap_or_else(|error| panic!("build printed invalid member JSON: {error}\n{output}"))
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
        "# composix — new-user guide\n\n{}\nComposix builds and runs services the way Nix builds software: a\nbuild produces an immutable directory in the Nix store (an \"item\"),\nand running it asks systemd to start a locked-down unit whose\nfilesystem is assembled from that item. If you know Docker: items\nplay the role of images, units of containers, the Cixfile of the\nDockerfile. If you have never used Docker: you need nothing from it —\na Cixfile is a small declaration of what goes into the item and what\nthe process may touch at runtime.\n\nThe local index is the per-user database that maps mutable names to immutable item paths. Each generated chapter uses a fresh index, so its names cannot read or alter your normal cix state. To follow every receipt you need Linux, Nix with flakes, `cix`, and a running systemd user manager; other hosts can still follow the build-only chapters. The rootless receipts ask that user manager to own process lifetime and accounting without root, but they explicitly lose some production sandbox controls; Chapter 1 names the exact boundary and checks every prerequisite. Start at [Chapter 1](01-hello-composix.html) and follow the guide in order.\n\n## Chapters\n",
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
    let _lock = TOUR_RENDER_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _host_lock = lock_tour_host_resources();
    let scenarios = vec![
        Scenario {
            filename: "01-hello-composix.md",
            title: "Chapter 1: Hello, composix",
            description: "Build, run, request, and stop one service without root privileges.",
            body: tour_scenarios::hello::chapter_hello(),
        },
        Scenario {
            filename: "02-cixfile-language.md",
            title: "Chapter 2: The Cixfile language",
            description:
                "Learn how declarations name inputs, assemble files, and grant runtime capabilities.",
            body: tour_scenarios::cixfile_language::chapter_cixfile_language(),
        },
        Scenario {
            filename: "03-building.md",
            title: "Chapter 3: Building: BUILDERs, FETCH, and the lock",
            description:
                "Pin downloaded bytes, reuse checked build work, repair a Linux binary, and build proj1.",
            body: tour_scenarios::building::chapter_building(),
        },
        Scenario {
            filename: "04-naming-distribution.md",
            title: "Chapter 4: Naming and distribution",
            description:
                "Group slash-prefixed names, serve their store contents, and refresh a name that moved.",
            body: tour_scenarios::naming_distribution::chapter_naming_distribution(),
        },
        Scenario {
            filename: "05-runtime-contract.md",
            title: "Chapter 5: Running: the runtime contract",
            description:
                "Run and probe a service, inspect its systemd state, and schedule a finite command.",
            body: tour_scenarios::runtime_contract::chapter_runtime_contract(),
        },
        Scenario {
            filename: "06-compose.md",
            title: "Chapter 6: Compose",
            description: "Connect producer and consumer paths, share writable data, and inspect deployment generations.",
            body: tour_scenarios::compose::chapter_compose(),
        },
        Scenario {
            filename: "07-dev-loop-docker.md",
            title: "Chapter 7: The dev loop and coming from Docker",
            description: "Rebuild after edits, compare two migration strategies, and browse worked migrations.",
            body: tour_scenarios::dev_loop::chapter_dev_loop(),
        },
    ];
    let mut files = Vec::with_capacity(scenarios.len() + 1);
    files.push(GeneratedFile {
        name: "index.md".to_owned(),
        content: render_index(&scenarios),
    });
    files.extend(
        scenarios
            .iter()
            .enumerate()
            .map(|(position, _)| GeneratedFile {
                name: scenarios[position].filename.to_owned(),
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
    write_generated_atomically(&directory, &render_tour())
        .unwrap_or_else(|error| panic!("publishing {}: {error:#}", directory.display()));
}

#[test]
fn generated_tour_is_deterministic() {
    let first = render_tour();
    let second = render_tour();
    if first == second {
        return;
    }
    // CI strips giant assert_eq payloads; print a bounded per-file diff so
    // the failing environment names its own leak.
    let mut report = String::new();
    for (a, b) in first.iter().zip(second.iter()) {
        if a == b {
            continue;
        }
        report.push_str(&format!("== {} differs between renders ==\n", a.name));
        let (left, right): (Vec<_>, Vec<_>) =
            (a.content.lines().collect(), b.content.lines().collect());
        let mut shown = 0;
        for i in 0..left.len().max(right.len()) {
            let l = left.get(i).copied().unwrap_or("<absent>");
            let r = right.get(i).copied().unwrap_or("<absent>");
            if l != r {
                report.push_str(&format!("line {}:\n  render1: {l}\n  render2: {r}\n", i + 1));
                shown += 1;
                if shown >= 40 {
                    report.push_str("… (diff capped at 40 lines)\n");
                    break;
                }
            }
        }
    }
    panic!("tour render is nondeterministic on this host:\n{report}");
}

#[test]
fn displayed_files_use_their_source_language() {
    for (path, expected) in [
        ("Cixfile", "dockerfile"),
        ("Cixfile.dissolved", "dockerfile"),
        ("Cixfile.lock", "json"),
        ("Cargo.lock", "toml"),
        ("Dockerfile", "dockerfile"),
        ("start", "sh"),
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
    let before = BTreeSet::from(["cix-run-decoy-x.service".to_owned()]);
    let after = BTreeSet::from([
        "cix-run-decoy-x.service".to_owned(),
        "cix-run-owned.service".to_owned(),
        "cix-run-owned.slice".to_owned(),
    ]);
    assert_eq!(
        created_cix_units(&before, &after, "cix-run-"),
        ["cix-run-owned.service", "cix-run-owned.slice"]
    );
}

#[test]
fn tour_matches_committed_document() {
    let expected = render_tour();
    assert_generated_matches(&tour_dir(), &expected).unwrap_or_else(|error| {
        panic!(
            "docs/tour drift; run `cargo test --test tour -- --ignored generate_tour`: {error:#}"
        )
    });
}
