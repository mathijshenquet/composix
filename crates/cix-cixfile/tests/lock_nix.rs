use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use cix_cixfile::{build, generate_nix, parse, BuildOptions, LockFile};

fn committed_lock() -> LockFile {
    let input = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../examples/pack/nginx/Cixfile.lock"
        ))
        .unwrap()["inputs"]["pkgs"]
            .clone(),
    )
    .unwrap();
    LockFile {
        inputs: std::collections::BTreeMap::from([("pkgs".into(), input)]),
        fetches: std::collections::BTreeMap::new(),
        memo: std::collections::BTreeMap::new(),
    }
}

#[test]
fn with_spec_redis_builds_mounts_and_parses() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().unwrap();
    let expression = format!(
        r#"let pkgs = import (builtins.getFlake "path:{}").inputs.nixpkgs {{ system = "x86_64-linux"; }}; in import {}/examples/pack/redis {{ inherit pkgs; }}"#,
        root.display(),
        root.display(),
    );
    let output = build_expression(&expression).unwrap();

    assert!(output.join("etc/redis/redis.conf").is_file());
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    assert_eq!(spec.cix_manifest, 2);
    assert_eq!(
        spec.services["redis"].mounts.as_deref().unwrap(),
        [PathBuf::from("/etc/redis")]
    );
}

fn build_expression(expression: &str) -> anyhow::Result<PathBuf> {
    let output = cix_common::nix(&[
        "build",
        "--impure",
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
    let cixfile = parse("FROM nixpkgs AS pkgs\nSERVICE fixture\nEXEC bin/fixture\n").unwrap();
    let mut lock = committed_lock();
    lock.inputs.get_mut("pkgs").unwrap().nar_hash =
        "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_owned();
    let expression = generate_nix(&cixfile, directory.path(), &lock, "x86_64-linux").unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(
        error.contains("hash mismatch") || error.contains("does not match"),
        "{error}"
    );
}

#[test]
fn unknown_nixpkgs_attribute_includes_the_cixfile_line() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        "FROM nixpkgs AS pkgs\nLINK bin/missing ${pkgs.thisAttributeDoesNotExist}/bin/missing\nSERVICE fixture\nEXEC bin/missing\n",
    )
    .unwrap();
    let expression = generate_nix(
        &cixfile,
        directory.path(),
        &committed_lock(),
        "x86_64-linux",
    )
    .unwrap();
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(error.contains("Cixfile line 2"), "{error}");
    assert!(error.contains("thisAttributeDoesNotExist"), "{error}");
}

#[test]
fn real_nix_build_assembles_files_scripts_links_and_spec() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("asset"),
        "literal=${hello}\nruntime=$VALUE\n",
    )
    .unwrap();
    let cixfile = parse(
        r#"FROM nixpkgs AS pkgs
COPY asset share/copied
FILE share/content <<EOF
package=${pkgs.hello}
escaped=$${literal}
runtime=$VALUE
EOF
SCRIPT bin/start <<EOF
exec /app/bin/hello
EOF
LINK bin/hello ${pkgs.hello}/bin/hello
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
    assert_eq!(
        fs::read_to_string(output.join("share/copied")).unwrap(),
        "literal=${hello}\nruntime=$VALUE\n"
    );

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
    assert_eq!(spec.cix_manifest, 2);
    assert_eq!(spec.services["fixture"].exec, ["bin/start"]);
}

#[test]
fn path_resolution_writes_the_real_executable_and_runtime_default() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM nixpkgs AS pkgs
PATH ${pkgs.coreutils}/bin
SERVICE fixture
SETUP true
EXEC true
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
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    let service = &spec.services["fixture"];
    assert!(
        service.exec[0].starts_with("/nix/store/"),
        "{:?}",
        service.exec
    );
    assert!(service.exec[0].ends_with("/bin/true"), "{:?}", service.exec);
    assert_eq!(service.setup.as_ref().unwrap(), &service.exec);
    assert!(service.env["PATH"]
        .default
        .as_deref()
        .is_some_and(|path| path.starts_with("/nix/store/") && path.ends_with("/bin")));
}

#[test]
fn path_resolution_prefers_the_first_matching_directory() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM nixpkgs AS pkgs
PATH ${pkgs.bash}/bin ${pkgs.bashInteractive}/bin
SERVICE fixture
EXEC bash
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
    let spec = cix_run::spec::Spec::load(&output).unwrap();
    let service = &spec.services["fixture"];
    let first_directory = service.env["PATH"]
        .default
        .as_deref()
        .unwrap()
        .split(':')
        .next()
        .unwrap();
    assert_eq!(service.exec[0], format!("{first_directory}/bash"));
}

#[test]
fn path_resolution_fails_with_line_and_searched_directories() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM nixpkgs AS pkgs
PATH ${pkgs.coreutils}/bin
SERVICE fixture
EXEC definitely-not-a-coreutils-command
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
    let error = build_expression(&expression).unwrap_err().to_string();
    assert!(error.contains("line 4"), "{error}");
    assert!(error.contains("declared PATH directories"), "{error}");
    assert!(error.contains("/bin"), "{error}");
}

#[test]
fn run_executes_outside_nix_and_build_interpolation_reaches_the_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("input"), "sandboxed\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM nixpkgs AS pkgs
PATH ${pkgs.bash}/bin ${pkgs.coreutils}/bin
COPY input input
RUN cp input output
SERVICE fixture
EXEC ${build}/output
"#,
    )
    .unwrap();
    let mut lock_json = serde_json::to_value(committed_lock()).unwrap();
    lock_json.as_object_mut().unwrap().remove("fetches");
    lock_json.as_object_mut().unwrap().remove("memo");
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock_json).unwrap()),
    )
    .unwrap();

    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&output).join("output")).unwrap(),
        "sandboxed\n"
    );
    let spec = cix_run::spec::Spec::load(&PathBuf::from(&output)).unwrap();
    assert!(spec.services["fixture"].exec[0].starts_with("/nix/store/"));
    assert!(spec.services["fixture"].exec[0].ends_with("-cix-build-snapshot/output"));

    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.memo.len(), 1);

    let repeated = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
    })
    .unwrap();
    assert_eq!(repeated, output);
}
