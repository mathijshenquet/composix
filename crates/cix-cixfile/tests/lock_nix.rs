use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use cix_cixfile::{generate_nix, parse, LockFile};

fn committed_lock() -> LockFile {
    serde_json::from_str(include_str!("../../../examples/nginx/Cixfile.lock")).unwrap()
}

fn build_expression(expression: &str) -> anyhow::Result<PathBuf> {
    let output = cix_common::nix(&[
        "build",
        "--no-link",
        "--print-out-paths",
        "--expr",
        expression,
    ])?;
    Ok(PathBuf::from(output.trim()))
}

#[test]
fn nix_rejects_a_committed_lock_with_the_wrong_nar_hash() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse("SERVICE fixture\nEXEC bin/fixture\n").unwrap();
    let mut lock = committed_lock();
    lock.nixpkgs.nar_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_owned();
    let expression = generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux").unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(
        error.contains("hash mismatch") || error.contains("does not match"),
        "{error}"
    );
}

#[test]
fn real_nix_build_assembles_files_scripts_links_and_spec() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"PKG hello
FILE share/content <<EOF
package=${hello}
escaped=$${literal}
runtime=$VALUE
EOF
SCRIPT bin/start <<EOF
exec /item/bin/hello
EOF
LINK bin/hello ${hello}/bin/hello
SERVICE fixture
EXEC bin/start
"#,
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let output = build_expression(&expression).unwrap();

    let contents = fs::read_to_string(output.join("share/content")).unwrap();
    assert!(contents.starts_with("package=/nix/store/"), "{contents}");
    assert!(contents.contains("\nescaped=${literal}\nruntime=$VALUE\n"));

    let script = fs::read_to_string(output.join("bin/start")).unwrap();
    assert!(script.starts_with("#!/nix/store/"), "{script}");
    assert_ne!(
        fs::metadata(output.join("bin/start"))
            .unwrap()
            .permissions()
            .mode()
            & 0o111,
        0
    );
    assert!(fs::read_link(output.join("bin/hello"))
        .unwrap()
        .to_string_lossy()
        .ends_with("/bin/hello"));

    let spec = cix_run::spec::Spec::load(&output).unwrap();
    assert_eq!(spec.cix_spec, 2);
    assert_eq!(spec.services["fixture"].exec, ["bin/start"]);
}
