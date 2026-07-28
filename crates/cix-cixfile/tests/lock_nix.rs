use cix_cixfile::{generate_nix, parse, LockFile};

#[test]
fn nix_rejects_a_committed_lock_with_the_wrong_nar_hash() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse("SERVICE fixture\nEXEC bin/fixture\n").unwrap();
    let mut lock: LockFile =
        serde_json::from_str(include_str!("../../../examples/nginx/Cixfile.lock")).unwrap();
    lock.nixpkgs.nar_hash =
        "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_owned();
    let expression =
        generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux").unwrap();
    let error = cix_common::nix(&[
        "build",
        "--no-link",
        "--print-out-paths",
        "--expr",
        &expression,
    ])
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("hash mismatch") || error.contains("does not match"),
        "{error}"
    );
}
