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

use regex::Regex;

const TOUR_LISTEN: &str = "127.0.0.1:8420";
static NEXT_TOUR_PORT: AtomicU16 = AtomicU16::new(10_000);
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
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &self.name])
            .output();
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

    fn sh_in(
        &mut self,
        prompt: &str,
        state_dir: &Path,
        command: &str,
        expect_success: bool,
    ) -> String {
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

        let displayed_command = normalize(command, &self.base);
        writeln!(self.text, "```sh\n{prompt} {displayed_command}").expect("writing command");
        let normalized = normalize(&raw, &self.base);
        if !normalized.is_empty() {
            self.text.push_str(&normalized);
            if !normalized.ends_with('\n') {
                self.text.push('\n');
            }
        }
        writeln!(self.text, "```\n").expect("writing transcript");
        raw
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
    let age = Regex::new(r"age=\d+s").expect("valid age regex");
    let unit_name =
        Regex::new(r"cix-run-tour-service-[0-9a-f]+\.service").expect("valid unit name regex");
    let capability_diagnostic = Regex::new(
        r"(?s)(warning: user manager rejected capability controls \().*?(\)\nwarning: retrying)",
    )
    .expect("valid capability diagnostic regex");
    let namespace_diagnostic = Regex::new(
        r"(?s)(warning: the user manager rejected mount-namespace sandboxing \().*?(\)\nwarning: retrying)",
    )
    .expect("valid namespace diagnostic regex");

    let normalized = store_hash.replace_all(raw, "/nix/store/…-");
    let normalized = port.replace_all(&normalized, TOUR_LISTEN);
    let normalized = created_at.replace_all(&normalized, "${1}1700000000${2}");
    let normalized = age.replace_all(&normalized, "age=0s");
    let normalized = unit_name.replace_all(&normalized, "cix-run-tour-service-NONCE.service");
    let normalized =
        capability_diagnostic.replace_all(&normalized, "${1}host-specific diagnostic${2}");
    let normalized =
        namespace_diagnostic.replace_all(&normalized, "${1}host-specific diagnostic${2}");
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
        &format!("mkdir -p {name} && printf '%s\\n' '{contents}' > {name}/README"),
        true,
    );
    let output = doc.sh_in(
        prompt,
        state_dir,
        &format!("nix store add-path {name}"),
        true,
    );
    let path = output.trim().to_owned();
    assert!(
        path.starts_with("/nix/store/"),
        "unexpected store path: {path}"
    );
    path
}

fn service_fixture(doc: &Doc) -> String {
    let fixture = doc.base.join("service-fixture");
    let executable = fixture.join("bin/service");
    fs::create_dir_all(executable.parent().expect("service executable parent"))
        .expect("creating service fixture directory");
    fs::write(&executable, "#!/bin/sh\nexec /bin/sleep 300\n").expect("writing service executable");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&executable)
            .expect("reading service executable permissions")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable, permissions).expect("making service executable");
    }
    fs::write(
        fixture.join("cix-spec.json"),
        r#"{
  "cixSpec": 2,
  "services": {
    "tour-service": {
      "exec": ["bin/service"],
      "dirs": {"state": ["/var/lib/tour-service"]}
    }
  }
}
"#,
    )
    .expect("writing service spec");

    let output = Command::new("nix")
        .args(["store", "add-path"])
        .arg(&fixture)
        .output()
        .expect("adding service fixture to the Nix store");
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

fn root_filename() -> &'static str {
    "bXktYXBwOnYx"
}

fn scenario_tagging_a_build() -> String {
    let mut doc = Doc::new("tagging");
    doc.para("Nix produced a store path. Give that immutable build a memorable local name.");

    let store_path = fixture(&mut doc, "fixture-v1", "hello from my app v1");
    doc.sh(&format!("cix tag {store_path} my-app:v1"), true);
    let listing = doc.sh("cix ls -l", true);
    assert!(listing.contains("my-app:v1"));
    assert!(listing.contains(&store_path));

    doc.para("The tag database is an `ls`-able symlink farm. Each symlink is a Nix GC root, so the pin *is* the name.");
    let roots = doc.sh("ls \"$CIX_STATE_DIR/roots\"", true);
    assert_eq!(roots.trim(), root_filename());
    let link = doc.sh(
        &format!("readlink \"$CIX_STATE_DIR/roots/{}\"", root_filename()),
        true,
    );
    assert_eq!(link.trim(), store_path);
    let sidecar = doc.sh(
        &format!("cat \"$CIX_STATE_DIR/tags/{}.json\"", root_filename()),
        true,
    );
    assert!(sidecar.contains("\"reference\": \"my-app:v1\""));
    assert!(sidecar.contains(&store_path));

    doc.finish()
}

fn scenario_moving_a_tag() -> String {
    let mut doc = Doc::new("moving");
    doc.para(
        "A tag can move to a newer build without changing the immutable store paths behind it.",
    );

    let first = fixture(&mut doc, "fixture-v1", "hello from my app v1");
    doc.sh(&format!("cix tag {first} my-app:v1"), true);
    let second = fixture(&mut doc, "fixture-v2", "hello from my app v2");
    doc.sh(&format!("cix tag {second} my-app:v1"), true);
    let listing = doc.sh("cix ls -l", true);
    assert!(listing.contains(&second));
    assert!(!listing.contains(&first));

    doc.para("Tags are mutable pointers over immutable store paths. Retagging changes the symlink; the old path is now unpinned by this tag.");
    let link = doc.sh(
        &format!("readlink \"$CIX_STATE_DIR/roots/{}\"", root_filename()),
        true,
    );
    assert_eq!(link.trim(), second);

    doc.finish()
}

fn scenario_untagging() -> String {
    let mut doc = Doc::new("untagging");
    doc.para("Removing a tag removes its local GC root and its metadata sidecar.");

    let store_path = fixture(&mut doc, "fixture-v1", "hello from my app v1");
    doc.sh(&format!("cix tag {store_path} my-app:v1"), true);
    doc.sh("cix untag my-app:v1", true);
    let listing = doc.sh("cix ls", true);
    assert!(listing.trim().is_empty());

    doc.para("Unpinned means the next `nix-collect-garbage` may reclaim the build; nothing else in cix holds it.");
    doc.finish()
}

fn scenario_serving_your_store() -> String {
    let mut doc = Doc::new("serving");
    let publisher = doc.state_dir.clone();
    doc.para("Publication is not a ceremony — serving exposes your bare tags at whatever URL reaches the box.");

    let store_path = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "fixture-v1",
        "hello from my app v1",
    );
    doc.sh_in(
        "publisher $",
        &publisher,
        &format!("cix tag {store_path} my-app:v1"),
        true,
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
    assert!(entry.contains(&store_path));

    doc.para("The same URL in a browser is an informative HTML page; here is only a short teaser, not the page dump.");
    let html = doc.sh_in(
        "publisher $",
        &publisher,
        &format!("curl -s http://{listen}/my-app:v1 | head -c 120"),
        true,
    );
    assert!(html.contains("<!doctype html>"));
    drop(server);
    doc.finish()
}

fn scenario_pulling_on_another_machine() -> String {
    let mut doc = Doc::new("pulling");
    let publisher = doc.state_dir.clone();
    let consumer = doc.base.join("consumer-state");
    doc.para("A second machine is just a second state dir.");

    let store_path = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "fixture-v1",
        "hello from my app v1",
    );
    doc.sh_in(
        "publisher $",
        &publisher,
        &format!("cix tag {store_path} my-app:v1"),
        true,
    );
    let listen = next_listen();
    doc.background(
        "publisher $",
        &format!("cix serve --with-store --listen {listen}"),
    );
    let server = start_server(&doc, &publisher, &listen);
    let pulled = doc.sh_in(
        "consumer $",
        &consumer,
        &format!("cix pull {listen}/my-app:v1 --as my-app"),
        true,
    );
    assert!(pulled.contains("updated 1 tag(s)"));
    let listing = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(listing.contains("my-app:latest"));
    assert!(listing.contains(&store_path));
    assert!(listing.contains(&format!("upstream={listen}")));

    doc.para("The qualified ref is self-describing; `--as` adopts it under a bare local name. A mirror keeps its qualified remote identity, while adoption makes the name local.");
    drop(server);
    doc.finish()
}

fn scenario_tags_move_pull_follows() -> String {
    let mut doc = Doc::new("pull-follows");
    let publisher = doc.state_dir.clone();
    let consumer = doc.base.join("consumer-state");
    doc.para("A consumer can track a remote tag without making the publisher's name local.");

    let first = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "fixture-v1",
        "hello from my app v1",
    );
    doc.sh_in(
        "publisher $",
        &publisher,
        &format!("cix tag {first} my-app:v1"),
        true,
    );
    let listen = next_listen();
    doc.background(
        "publisher $",
        &format!("cix serve --with-store --listen {listen}"),
    );
    let server = start_server(&doc, &publisher, &listen);
    doc.sh_in(
        "consumer $",
        &consumer,
        &format!("cix pull {listen}/my-app:v1"),
        true,
    );
    let second = fixture_in(
        &mut doc,
        "publisher $",
        &publisher,
        "fixture-v2",
        "hello from my app v2",
    );
    doc.sh_in(
        "publisher $",
        &publisher,
        &format!("cix tag {second} my-app:v1"),
        true,
    );
    let refreshed = doc.sh_in("consumer $", &consumer, "cix pull", true);
    assert!(refreshed.contains("updated 1 tag(s)"));
    let listing = doc.sh_in("consumer $", &consumer, "cix ls -l", true);
    assert!(listing.contains(&second));
    assert!(!listing.contains(&first));

    doc.para("Tags are mutable names over immutable paths, refreshed like git remotes. GC follows the pins: after the refresh, this consumer tag roots the new path, not the old one.");
    drop(server);
    doc.finish()
}

fn scenario_running_a_service() -> String {
    let mut doc = Doc::new("running-service");
    doc.para("A spec'd store item can become a transient systemd service without root.");

    let store_path = service_fixture(&doc);
    let started = doc.sh(&format!("cix run {store_path} --detach --user"), true);
    let unit_name = started
        .lines()
        .find(|line| line.starts_with("cix-run-tour-service-") && line.ends_with(".service"))
        .expect("cix run printed a transient unit name")
        .to_owned();
    let _unit = UserUnit {
        name: unit_name.clone(),
    };

    doc.para("`--user` is the rootless development mode. The product target is the system manager, with `DynamicUser` and the full hardening profile; see the [design document](../design.html). The VM check exercises that system path.");
    let running = doc.sh("cix ps", true);
    assert!(
        running.contains(&unit_name),
        "cix ps did not show {unit_name}"
    );

    doc.sh(&format!("systemctl --user stop {unit_name}"), true);
    let stopped = doc.sh("cix ps", true);
    assert!(
        !stopped.contains(&unit_name),
        "cix ps still showed stopped unit {unit_name}"
    );

    doc.para("The unit disappears once stopped; its managed state directory follows the user-manager lifecycle.");
    doc.finish()
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
        "# cix — local index tour\n\n{}\nThis five-minute tour covers local tags, serving a store, pulling from it, and running a service.\n\n## Scenarios\n",
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
            filename: "01-tagging.md",
            title: "Tagging a build",
            description: "Give an immutable Nix store path a memorable local name.",
            body: scenario_tagging_a_build(),
        },
        Scenario {
            filename: "02-moving.md",
            title: "Moving a tag",
            description: "Retag a name to point at a newer build.",
            body: scenario_moving_a_tag(),
        },
        Scenario {
            filename: "03-untagging.md",
            title: "Untagging",
            description: "Remove a local tag and its GC root.",
            body: scenario_untagging(),
        },
        Scenario {
            filename: "04-serving.md",
            title: "Serving your store",
            description: "Expose bare local tags over HTTP.",
            body: scenario_serving_your_store(),
        },
        Scenario {
            filename: "05-pulling.md",
            title: "Pulling on another machine",
            description: "Adopt a qualified remote tag under a local name.",
            body: scenario_pulling_on_another_machine(),
        },
        Scenario {
            filename: "06-pull-follows.md",
            title: "Tags move; pull follows",
            description: "Refresh a remote mirror after its publisher retags it.",
            body: scenario_tags_move_pull_follows(),
        },
        Scenario {
            filename: "07-running-service.md",
            title: "Running a service",
            description: "Start and inspect a spec'd service in rootless development mode.",
            body: scenario_running_a_service(),
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
