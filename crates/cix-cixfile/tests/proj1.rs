use std::fs;
use std::path::{Path, PathBuf};

use cix_cixfile::{build, parse, BuildOptions, BuiltItem, LockFile};

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

#[test]
fn proj1_multi_item_cache_selectivity_and_clean_rebuild() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/build/proj1");
    let temporary = tempfile::tempdir().unwrap();
    copy_project(&source, temporary.path());
    let mut clean_lock = load_lock(temporary.path());
    clean_lock.fetches.clear();
    clean_lock.memo.clear();
    fs::write(
        temporary.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&clean_lock).unwrap()),
    )
    .unwrap();

    let parsed = parse(&fs::read_to_string(temporary.path().join("Cixfile")).unwrap()).unwrap();
    assert_eq!(parsed.builders["build"].caches, ["target"]);
    assert_eq!(parsed.artifact_order, ["proj1-api", "proj1-worker"]);

    let first = run_build(temporary.path(), false);
    assert_items_are_minimal_and_v4(&first);
    let first_lock = load_lock(temporary.path());
    assert_eq!(first_lock.memo.len(), 1);

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
    assert_eq!(load_lock(temporary.path()).memo.len(), 2);

    let clean = run_build(temporary.path(), true);
    assert_eq!(clean, edited);
}

fn run_build(directory: &Path, no_cache: bool) -> Vec<BuiltItem> {
    build(&BuildOptions {
        directory: directory.to_owned(),
        update_lock: None,
        tag: None,
        no_cache,
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

fn assert_items_are_minimal_and_v4(items: &[BuiltItem]) {
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
        assert_eq!(manifest["cixManifest"], 4);
        assert!(manifest.get("services").is_none());
        assert_eq!(
            manifest
                .get("outbound")
                .and_then(serde_json::Value::as_bool),
            (item.name == "proj1-worker").then_some(true)
        );
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

fn copy_project(source: &Path, destination: &Path) {
    for relative in PROJECT_FILES {
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(relative), target).unwrap();
    }
}
