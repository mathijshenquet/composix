use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use cix_index::{Entry, Output, PointerChanged, Store, TagMetadata};

const NAME: &str = "hammer";
const PROCESSES: usize = 4;
const ROUNDS: usize = 8;

fn temporary_path(label: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "cix-index-hammer-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos()
    ))
}

fn artifact(root: &Path, worker: usize) -> Output {
    let source = root.join(format!("artifact-{worker}"));
    fs::create_dir_all(&source).expect("create artifact source");
    fs::write(source.join("payload"), "hammer").expect("write artifact payload");
    let source = source.to_string_lossy().into_owned();
    let store_path = Command::new("nix")
        .args(["store", "add-path", &source])
        .output()
        .expect("run nix store add-path");
    assert!(store_path.status.success(), "{store_path:?}");
    let store_path = String::from_utf8(store_path.stdout)
        .expect("nix path is UTF-8")
        .trim()
        .to_owned();
    let info = Command::new("nix")
        .args(["path-info", "--json", "--json-format", "1", &store_path])
        .output()
        .expect("run nix path-info");
    assert!(info.status.success(), "{info:?}");
    let info: BTreeMap<String, serde_json::Value> =
        serde_json::from_slice(&info.stdout).expect("parse nix path-info");
    Output {
        store_path: store_path.clone(),
        nar_hash: info[&store_path]["narHash"]
            .as_str()
            .expect("nar hash")
            .to_owned(),
        drv_path: None,
    }
}

fn metadata(tag: &str, output: &Output) -> TagMetadata {
    TagMetadata {
        reference: format!("{NAME}:{tag}"),
        entry: Entry {
            outputs: BTreeMap::from([("x86_64-linux".into(), output.clone())]),
            substituters: Vec::new(),
            trusted_keys: Vec::new(),
            created_at: "1".into(),
        },
        upstream: None,
    }
}

fn worker(root: &Path, worker: usize) {
    let store = Store::open().expect("open worker store");
    let output = artifact(root, worker);
    let mut operations = Vec::new();
    for round in 0..ROUNDS {
        let tag = format!("p{worker}-r{round}");
        match store.publish_many(NAME, vec![(tag.clone(), metadata(&tag, &output))]) {
            Ok(()) => operations.push(format!("P {tag}")),
            Err(error) if error.downcast_ref::<PointerChanged>().is_some() => {
                operations.push("C publish".into())
            }
            Err(error) => panic!("publish corruption: {error:#}"),
        }
        if round % 3 == 2 {
            let previous = format!("p{worker}-r{}", round - 1);
            match store.yank(NAME, &previous) {
                Ok(true) => operations.push(format!("Y {previous}")),
                Ok(false) => operations.push(format!("M {previous}")),
                Err(error) if error.downcast_ref::<PointerChanged>().is_some() => {
                    operations.push("C yank".into())
                }
                Err(error) => panic!("yank corruption: {error:#}"),
            }
        }
    }
    fs::write(root.join(format!("report-{worker}")), operations.join("\n")).expect("write report");
}

#[test]
#[ignore = "slow OS-process concurrency hammer; run explicitly"]
fn concurrent_publish_many_and_yank_preserve_the_table() {
    let root = temporary_path("root");
    fs::create_dir_all(&root).expect("create hammer root");
    if let Ok(worker_number) = env::var("CIX_INDEX_HAMMER_WORKER") {
        let root = PathBuf::from(env::var_os("CIX_STATE_DIR").expect("worker state directory"));
        worker(&root, worker_number.parse().expect("worker number"));
        return;
    }

    let executable = env::current_exe().expect("test executable");
    let mut children = Vec::new();
    for worker in 0..PROCESSES {
        children.push(
            Command::new(&executable)
                .args([
                    "--exact",
                    "concurrent_publish_many_and_yank_preserve_the_table",
                    "--ignored",
                    "--nocapture",
                ])
                .env("CIX_STATE_DIR", &root)
                .env("CIX_INDEX_HAMMER_WORKER", worker.to_string())
                .spawn()
                .expect("spawn hammer worker"),
        );
    }
    for mut child in children {
        assert!(
            child.wait().expect("wait worker").success(),
            "worker failed"
        );
    }

    let mut expected = BTreeSet::new();
    let mut conflicts = 0;
    for worker in 0..PROCESSES {
        let report =
            fs::read_to_string(root.join(format!("report-{worker}"))).expect("read report");
        for operation in report.lines() {
            let (kind, tag) = operation.split_once(' ').expect("operation has a tag");
            match kind {
                "P" => {
                    expected.insert(tag.to_owned());
                }
                "Y" => {
                    expected.remove(tag);
                }
                "C" => conflicts += 1,
                "M" => {}
                _ => panic!("unknown operation {operation:?}"),
            }
        }
    }
    env::set_var("CIX_STATE_DIR", &root);
    let store = Store::open().expect("open final store");
    let actual = store
        .all()
        .expect("read final table")
        .into_iter()
        .filter_map(|entry| entry.reference.strip_prefix("hammer:").map(str::to_owned))
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "no successful pointer flip was lost");
    let history = store.history(NAME).expect("walk available parent chain");
    assert!(!history.is_empty(), "current pointer has a table");
    assert_eq!(
        history[0].tags.iter().cloned().collect::<BTreeSet<_>>(),
        expected,
        "the current table is the process-linearized result"
    );
    eprintln!("hammer observed {conflicts} surfaced PointerChanged conflicts");
    fs::remove_dir_all(root).expect("remove hammer state");
}
