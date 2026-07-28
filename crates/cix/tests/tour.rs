//! Executed local-index scenarios that generate `docs/tour.md`.
//!
//! Run `cargo test --test tour -- --ignored generate_tour` to update the document.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

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

    fn heading(&mut self, level: usize, text: &str) {
        writeln!(self.text, "\n{} {text}\n", "#".repeat(level)).expect("writing heading");
    }

    fn para(&mut self, text: &str) {
        writeln!(self.text, "{text}\n").expect("writing paragraph");
    }

    fn sh(&mut self, command: &str, expect_success: bool) -> String {
        let mut path = self.bin_dir.display().to_string();
        if let Some(existing) = std::env::var_os("PATH") {
            path.push(':');
            path.push_str(&existing.to_string_lossy());
        }
        let output = Command::new("sh")
            .args(["-c", command])
            .current_dir(&self.base)
            .env("CIX_STATE_DIR", &self.state_dir)
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
        writeln!(self.text, "```sh\n$ {displayed_command}").expect("writing command");
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
    let created_at =
        Regex::new(r#"(\"createdAt\"\s*:\s*\")\d{10}(\")"#).expect("valid createdAt regex");
    let age = Regex::new(r"age=\d+s").expect("valid age regex");

    let normalized = store_hash.replace_all(raw, "/nix/store/…-");
    let normalized = created_at.replace_all(&normalized, "${1}1700000000${2}");
    let normalized = age.replace_all(&normalized, "age=0s");
    normalized.replace(base.to_string_lossy().as_ref(), "~")
}

fn fixture(doc: &mut Doc, name: &str, contents: &str) -> String {
    doc.sh(
        &format!("mkdir -p {name} && printf '%s\\n' '{contents}' > {name}/README"),
        true,
    );
    let output = doc.sh(&format!("nix store add-path {name}"), true);
    let path = output.trim().to_owned();
    assert!(
        path.starts_with("/nix/store/"),
        "unexpected store path: {path}"
    );
    path
}

fn root_filename() -> &'static str {
    "bXktYXBwOnYx"
}

fn scenario_tagging_a_build() -> String {
    let mut doc = Doc::new("tagging");
    doc.heading(2, "Tagging a build");
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
    doc.heading(2, "Moving a tag");
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
    doc.heading(2, "Untagging");
    doc.para("Removing a tag removes its local GC root and its metadata sidecar.");

    let store_path = fixture(&mut doc, "fixture-v1", "hello from my app v1");
    doc.sh(&format!("cix tag {store_path} my-app:v1"), true);
    doc.sh("cix untag my-app:v1", true);
    let listing = doc.sh("cix ls", true);
    assert!(listing.trim().is_empty());

    doc.para("Unpinned means the next `nix-collect-garbage` may reclaim the build; nothing else in cix holds it.");
    doc.finish()
}

fn generate_header() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let commit = option_env!("GIT_COMMIT_HASH").unwrap_or("unknown");
    format!(
        "# cix — local index tour\n\n> **Auto-generated** by `cargo test --test tour -- --ignored generate_tour`.\n> All outputs reflect actual behavior: each scenario drives the real `cix` binary in an isolated local index.\n> Version **{version}**, commit `{commit}`.\n> **Do not edit** — re-run the test to regenerate.\n\nThis five-minute tour covers local tags only: naming a build, moving that name, and removing it.\n"
    )
}

fn render_tour() -> String {
    let mut doc = generate_header();
    doc.push_str(&scenario_tagging_a_build());
    doc.push_str(&scenario_moving_a_tag());
    doc.push_str(&scenario_untagging());
    doc
}

fn tour_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("docs/tour.md")
}

#[test]
#[ignore = "run explicitly to update docs/tour.md"]
fn generate_tour() {
    let path = tour_path();
    fs::create_dir_all(path.parent().expect("tour parent directory"))
        .expect("creating docs directory");
    fs::write(&path, render_tour()).expect("writing tour document");
    eprintln!("wrote {}", path.display());
}

#[test]
fn generated_tour_is_deterministic() {
    assert_eq!(render_tour(), render_tour());
}

#[test]
fn tour_matches_committed_document() {
    let expected = render_tour();
    let actual = fs::read_to_string(tour_path()).unwrap_or_default();
    assert_eq!(
        actual, expected,
        "docs/tour.md has drifted; run `cargo test --test tour -- --ignored generate_tour`"
    );
}
