use std::{
    env, fs,
    net::TcpListener,
    path::PathBuf,
    process::Command,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cix_common::Ref;
use cix_index::{pull, serve, tag, Store};

fn temporary_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = env::temp_dir().join(format!("cix-index-{label}-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn pull_from_local_store_server_records_upstream() {
    let server_state = temporary_dir("server");
    let client_state = temporary_dir("client");
    let source = temporary_dir("source");
    let input = source.join("input");
    fs::create_dir(&input).unwrap();
    fs::write(input.join("message"), "cix integration test").unwrap();
    let store_path = String::from_utf8(
        Command::new("nix")
            .args(["store", "add-path"])
            .arg(&input)
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let store_path = store_path.trim().to_owned();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let root_url = format!("localhost:{port}");

    env::set_var("CIX_STATE_DIR", &server_state);
    tag(&store_path, &format!("{root_url}/x:v1"), None).unwrap();
    let serve_root = root_url.clone();
    thread::spawn(move || {
        serve(
            &serve_root,
            &format!("127.0.0.1:{port}"),
            Vec::new(),
            true,
            None,
        )
        .unwrap();
    });
    thread::sleep(Duration::from_millis(250));

    env::set_var("CIX_STATE_DIR", &client_state);
    assert_eq!(
        pull(Some(&format!("{root_url}/x:v1")), Some("x")).unwrap(),
        1
    );
    let local = Ref::parse("x").unwrap();
    let metadata = Store::open().unwrap().load(&local).unwrap().unwrap();
    assert_eq!(metadata.upstream.as_deref(), Some(root_url.as_str()));
    assert_eq!(
        metadata
            .entry
            .outputs
            .get("x86_64-linux")
            .unwrap()
            .store_path,
        store_path
    );

    fs::remove_dir_all(server_state).unwrap();
    fs::remove_dir_all(client_state).unwrap();
    fs::remove_dir_all(source).unwrap();
}
