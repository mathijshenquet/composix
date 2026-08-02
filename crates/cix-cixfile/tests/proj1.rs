use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use cix_cixfile::{build, build_with_stats, parse, BuildOptions, BuiltItem, LockFile};

const PROJECT_FILES: &[&str] = &[
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

static WORKSPACE_DIRECTORY: Mutex<()> = Mutex::new(());

struct WorkspaceDirectory {
    previous: Option<OsString>,
}

impl WorkspaceDirectory {
    fn set(path: &Path) -> Self {
        let previous = std::env::var_os("CIX_BUILD_WORKSPACE_DIR");
        std::env::set_var("CIX_BUILD_WORKSPACE_DIR", path);
        Self { previous }
    }
}

impl Drop for WorkspaceDirectory {
    fn drop(&mut self) {
        match &self.previous {
            Some(path) => std::env::set_var("CIX_BUILD_WORKSPACE_DIR", path),
            None => std::env::remove_var("CIX_BUILD_WORKSPACE_DIR"),
        }
    }
}

#[test]
fn proj1_multi_item_cache_selectivity_and_clean_rebuild() {
    let _workspace_directory_lock = WORKSPACE_DIRECTORY.lock().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/build/proj1");
    let temporary = tempfile::tempdir().unwrap();
    copy_project(&source, temporary.path());
    let workspace_base = temporary.path().join("workspaces");
    let _workspace_directory = WorkspaceDirectory::set(&workspace_base);
    let mut clean_lock = load_lock(temporary.path());
    clean_lock.fetches.clear();
    clean_lock.memo.clear();
    fs::write(
        temporary.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&clean_lock).unwrap()),
    )
    .unwrap();

    let parsed = parse(&fs::read_to_string(temporary.path().join("Cixfile")).unwrap()).unwrap();
    assert_eq!(parsed.artifact_order, ["proj1-api", "proj1-worker"]);

    let first = run_build(temporary.path(), false);
    assert_items_are_minimal_and_v0(&first);
    let first_lock = load_lock(temporary.path());
    assert_eq!(first_lock.memo.len(), 1);
    assert_consumed_binaries(&first_lock);
    let workspace = only_workspace(&workspace_base);
    assert!(workspace.join("work/target/.cix-warm").is_file());

    let unchanged = run_build(temporary.path(), false);
    assert_eq!(unchanged, first);
    assert_eq!(load_lock(temporary.path()).memo.len(), 1);

    let worker_path = temporary.path().join("rust/worker/src/main.rs");
    let worker = fs::read_to_string(&worker_path).unwrap();
    fs::write(
        &worker_path,
        worker.replace("proj1-worker", "proj1-worker-edited"),
    )
    .unwrap();
    let edited = run_build(temporary.path(), false);
    assert_eq!(path(&edited, "proj1-api"), path(&first, "proj1-api"));
    assert_ne!(path(&edited, "proj1-worker"), path(&first, "proj1-worker"));
    let edited_lock = load_lock(temporary.path());
    assert_eq!(edited_lock.memo.len(), 2);
    assert_consumed_binaries(&edited_lock);

    let clean = run_build(temporary.path(), true);
    assert_eq!(clean, edited);
    assert_consumed_binaries(&load_lock(temporary.path()));

    fs::remove_dir_all(&workspace).unwrap();
    let after_wipe = run_build(temporary.path(), false);
    assert_eq!(after_wipe, edited);
    assert!(!workspace.exists());
}

#[test]
fn local_fetch_fixture_has_a_zero_subprocess_noop_and_cold_convergence() {
    let _workspace_directory_lock = WORKSPACE_DIRECTORY.lock().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 1024];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\nhello\n")
            .unwrap();
    });
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir().unwrap();
    fs::write(
        temporary.path().join("Cixfile"),
        format!(
            "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH web ${{pkgs.curl}}/bin/curl -fsS http://{address} > payload\nBUILDER build\nIMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}\nCOPY ${{web}}/payload payload\nRUN cp payload out\nITEM app\nCOPY ${{build}}/out /out\n"
        ),
    )
    .unwrap();
    let lock = root.join("examples/pack/nginx/Cixfile.lock");
    fs::copy(lock, temporary.path().join("Cixfile.lock")).unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let _workspace_directory = WorkspaceDirectory::set(workspace.path());
    let options = BuildOptions {
        directory: temporary.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
        allow_secret: false,
    };
    let (first, _) = build_with_stats(&options).unwrap();
    server.join().unwrap();
    let (repeat, stats) = build_with_stats(&options).unwrap();
    assert_eq!(repeat, first);
    assert_eq!(stats.nix_subprocesses, 0);
    assert!(stats.steps.iter().all(|step| step.status == "memo-hit"));

    let cixfile = temporary.path().join("Cixfile");
    fs::write(
        &cixfile,
        fs::read_to_string(&cixfile).unwrap().replace(
            "RUN cp payload out",
            "RUN test -f payload && cp payload out",
        ),
    )
    .unwrap();
    let (_, edited) = build_with_stats(&options).unwrap();
    assert!(edited
        .steps
        .iter()
        .any(|step| step.kind == "RUN" && step.status == "executed"));

    let cold = build(&BuildOptions {
        cold: true,
        allow_secret: false,
        ..options
    })
    .unwrap();
    assert_eq!(cold, first);
}

fn run_build(directory: &Path, cold: bool) -> Vec<BuiltItem> {
    build(&BuildOptions {
        directory: directory.to_owned(),
        update_lock: None,
        tag: None,
        cold,
        allow_secret: false,
    })
    .unwrap()
}

fn path<'a>(items: &'a [BuiltItem], name: &str) -> &'a str {
    &items
        .iter()
        .find(|item| item.name == name)
        .unwrap()
        .store_path
}

fn assert_items_are_minimal_and_v0(items: &[BuiltItem]) {
    for item in items {
        let root = Path::new(&item.store_path);
        let mut listing = list_relative(root);
        listing.sort();
        assert_eq!(
            listing,
            [
                "bin".to_owned(),
                format!("bin/{}", item.name),
                "cix-manifest.json".to_owned(),
            ]
        );

        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("cix-manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["cixManifest"], 0);
        assert!(manifest.get("services").is_none());
        if item.name == "proj1-worker" {
            assert_eq!(manifest["claims"], serde_json::json!(["egress"]));
        } else {
            assert!(manifest.get("claims").is_none());
        }
    }
}

fn list_relative(root: &Path) -> Vec<String> {
    fn visit(root: &Path, directory: &Path, entries: &mut Vec<String>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            entries.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
            if path.is_dir() {
                visit(root, &path, entries);
            }
        }
    }

    let mut entries = Vec::new();
    visit(root, root, &mut entries);
    entries
}

fn load_lock(directory: &Path) -> LockFile {
    serde_json::from_slice(&fs::read(directory.join("Cixfile.lock")).unwrap()).unwrap()
}

fn assert_consumed_binaries(lock: &LockFile) {
    let latest = lock.memo.values().next_back().unwrap();
    assert_eq!(
        latest.paths.keys().map(String::as_str).collect::<Vec<_>>(),
        ["target/release/proj1-api", "target/release/proj1-worker"]
    );
    assert!(latest
        .paths
        .values()
        .all(|path| path.store_path.starts_with("/nix/store/")
            && path.nar_hash.starts_with("sha256-")));
}

fn only_workspace(base: &Path) -> PathBuf {
    let entries = fs::read_dir(base)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "{entries:?}");
    entries[0].clone()
}

fn copy_project(source: &Path, destination: &Path) {
    for relative in PROJECT_FILES {
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(relative), target).unwrap();
    }
}
