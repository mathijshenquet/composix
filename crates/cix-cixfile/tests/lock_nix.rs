use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use cix_cixfile::{build, build_family, generate_nix, parse, ArtifactPin, BuildOptions, LockFile};

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
        artifacts: std::collections::BTreeMap::new(),
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
    assert_eq!(spec.cix_manifest, 0);
    assert_eq!(
        spec.select_service(None)
            .unwrap()
            .1
            .mounts
            .as_deref()
            .unwrap(),
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
    let cixfile = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nEXEC /bin/fixture\n",
    )
    .unwrap();
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
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nLINK ${pkgs.thisAttributeDoesNotExist}/bin/missing /bin/missing\nEXEC /bin/missing\n",
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
    assert!(error.contains("Cixfile line 3"), "{error}");
    assert!(error.contains("thisAttributeDoesNotExist"), "{error}");
}

#[test]
fn real_nix_build_assembles_files_links_and_spec() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
FILE /share/content <<EOF
package=${pkgs.hello}
escaped=$${literal}
runtime=$VALUE
EOF
LINK ${pkgs.hello}/bin/hello /bin/hello
EXEC hello
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
    assert!(fs::read_link(output.join("bin/hello"))
        .unwrap()
        .to_string_lossy()
        .ends_with("/bin/hello"));

    let spec = cix_run::spec::Spec::load(&output).unwrap();
    assert_eq!(spec.cix_manifest, 0);
    assert_eq!(spec.select_service(None).unwrap().1.exec, ["bin/hello"]);
    assert_eq!(
        spec.select_service(None).unwrap().1.env["PATH"]
            .default
            .as_deref(),
        Some("bin")
    );
}

#[test]
fn bare_commands_resolve_against_item_bin_and_explicit_path_replaces_default() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
LINK ${pkgs.coreutils}/bin/true /bin/true
ENV PATH = ${pkgs.bash}/bin
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
    let service = spec.select_service(None).unwrap().1;
    assert_eq!(service.exec, ["bin/true"]);
    assert_eq!(service.setup.as_ref().unwrap(), &service.exec);
    assert!(service.env["PATH"]
        .default
        .as_deref()
        .is_some_and(|path| path.starts_with("/nix/store/") && path.ends_with("/bin")));
}

#[test]
fn bare_commands_ignore_explicit_path_when_resolving() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
LINK ${pkgs.bash}/bin/bash /bin/bash
ENV PATH = ${pkgs.coreutils}/bin
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
    let service = spec.select_service(None).unwrap().1;
    assert_eq!(service.exec, ["bin/bash"]);
    assert!(service.env["PATH"]
        .default
        .as_deref()
        .is_some_and(|path| path.starts_with("/nix/store/") && path.ends_with("/bin")));
}

#[test]
fn bare_command_failure_lists_the_item_bin_entries() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = parse(
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
SERVICE fixture
LINK ${pkgs.coreutils}/bin/true /bin/true
EXEC definitely-not-in-bin
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
    assert!(error.contains("item's bin/"), "{error}");
    assert!(error.contains("true"), "{error}");
}

#[test]
fn run_executes_outside_nix_and_build_interpolation_reaches_the_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("input"), "sandboxed\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
COPY ${src}/input input
ENV OUTPUT = $PWD/output
RUN <<BUILD
# A RUN heredoc is sent to the same builder shell as a one-line RUN.
cp input "$OUTPUT"
BUILD
SERVICE fixture
COPY ${build}/output /bin/output
EXEC /bin/output
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
        cold: false,
    })
    .unwrap();
    let output = &output[0].store_path;
    assert_eq!(
        fs::read_to_string(PathBuf::from(&output).join("bin/output")).unwrap(),
        "sandboxed\n"
    );
    let spec = cix_run::spec::Spec::load(&PathBuf::from(&output)).unwrap();
    assert_eq!(spec.select_service(None).unwrap().1.exec[0], "/bin/output");

    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.memo.len(), 1);

    let repeated = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(repeated[0].store_path, *output);
}

#[test]
fn selected_member_executes_only_its_backward_builder_slice() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER wanted
IMPORT ${pkgs.bash}
RUN printf wanted > wanted
BUILDER unrelated
IMPORT ${pkgs.bash}
RUN exit 42
SERVICE api
COPY ${wanted}/wanted /payload
EXEC /bin/true
SERVICE worker
COPY ${unrelated}/missing /missing
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let output = build_family(
        &BuildOptions {
            directory: directory.path().to_owned(),
            update_lock: None,
            tag: None,
            cold: false,
        },
        &[],
        None,
        Some("api"),
    )
    .unwrap();
    assert_eq!(output.len(), 1);
    assert_eq!(output[0].name, "api");
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("payload")).unwrap(),
        "wanted"
    );
}

#[test]
fn fetch_expect_matches_in_both_forms_and_records_the_declared_hash() {
    let directory = tempfile::tempdir().unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("payload"), "fixed\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    fs::write(
        directory.path().join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FETCH ingredient EXPECT {expected} ${{pkgs.coreutils}}/bin/printf 'fixed\n' > payload
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
FETCH EXPECT {expected} printf 'fixed\n' > payload
SERVICE top
COPY ${{ingredient}}/payload /payload
EXEC /bin/true
SERVICE nested
COPY ${{build}}/payload /payload
EXEC /bin/true
"#,
        ),
    )
    .unwrap();
    let mut lock = committed_lock();
    lock.fetches.insert(
        "ingredient".into(),
        cix_cixfile::FetchPin::expected("sha256-stale".into()),
    );
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
    )
    .unwrap();

    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(output.len(), 2);
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.fetches.len(), 2);
    assert!(lock.fetches.values().all(|pin| pin.nar_hash == expected));
}

#[test]
fn fetch_expect_mismatch_names_declared_and_actual_hashes() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFETCH ingredient EXPECT sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= ${pkgs.coreutils}/bin/printf payload > payload\nSERVICE app\nCOPY ${ingredient}/payload /payload\nEXEC /bin/true\n",
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("declared sha256-AAAA"), "{rendered}");
    assert!(rendered.contains("fetched sha256-"), "{rendered}");
    assert!(!rendered.contains("--update-lock to accept"), "{rendered}");
}

#[test]
fn imported_cacert_enables_bare_git_over_https() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let cixfile = |cacert: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER fetch
IMPORT ${{pkgs.bash}} ${{pkgs.gitMinimal}}{cacert}
FETCH git ls-remote https://github.com/NixOS/nixpkgs.git HEAD > head
SERVICE result
COPY ${{fetch}}/head /head
EXEC /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("FETCH failed"), "{rendered}");
    assert!(
        rendered.to_ascii_lowercase().contains("certificate"),
        "{rendered}"
    );

    fs::write(directory.path().join("Cixfile"), cixfile(" ${pkgs.cacert}")).unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    let head = fs::read_to_string(PathBuf::from(&output[0].store_path).join("head")).unwrap();
    assert!(head.ends_with("\tHEAD\n"), "{head}");
}

#[test]
fn usr_bin_env_shebang_requires_an_imported_env() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let script = directory.path().join("script");
    fs::write(&script, "#!/usr/bin/env bash\nprintf shebang-ok\n").unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).unwrap();
    let cixfile = |coreutils: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${{pkgs.bash}}{coreutils}
COPY ${{src}}/script script
RUN ./script > output
SERVICE result
COPY ${{build}}/output /output
EXEC /bin/true
"#
        )
    };

    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let error = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap_err();
    let rendered = format!("{error:#}");
    assert!(rendered.contains("RUN failed"), "{rendered}");
    assert!(rendered.contains("/usr/bin/env"), "{rendered}");
    assert!(rendered.contains("IMPORT ${pkgs.coreutils}"), "{rendered}");

    fs::write(
        directory.path().join("Cixfile"),
        cixfile(" ${pkgs.coreutils}"),
    )
    .unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&output[0].store_path).join("output")).unwrap(),
        "shebang-ok"
    );
}

#[test]
fn newly_consumed_path_reruns_the_chain_and_extends_its_record() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let cixfile = |extra_copy: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
RUN printf x >> runs; printf one > one; printf two > two
SERVICE result
COPY ${{build}}/runs /runs
COPY ${{build}}/one /one
{extra_copy}EXEC /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("runs")).unwrap(),
        "x"
    );

    fs::write(
        directory.path().join("Cixfile"),
        cixfile("COPY ${build}/two /two\n"),
    )
    .unwrap();
    let second = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&second[0].store_path).join("runs")).unwrap(),
        "xx"
    );
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(lock.memo.len(), 1);
    assert_eq!(
        lock.memo
            .values()
            .next()
            .unwrap()
            .paths
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one", "runs", "two"]
    );
}

#[test]
fn automatic_fetch_pins_only_consumed_paths_and_cold_replays_its_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
FETCH test ! -e fetch-ran; touch fetch-ran; printf payload > wanted; printf incidental > ignored
RUN printf suffix >> wanted; cp wanted result
SERVICE result
COPY ${build}/result /result
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let pin = lock.fetches.values().next().unwrap();
    assert_eq!(
        pin.paths.keys().map(String::as_str).collect::<Vec<_>>(),
        ["result"]
    );
    assert!(pin.store_path.is_none());

    // The replay snapshot already contains fetch-ran. Re-executing FETCH would fail
    // its first command, so a successful cold build proves no fetch process spawned.
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("result")).unwrap(),
        "payloadsuffix"
    );
}

#[test]
fn cold_replays_a_top_level_fetch_snapshot_without_executing_fetch() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FETCH ingredient test ! -e fetch-ran; printf ran > fetch-ran; printf payload > payload
SERVICE result
COPY ${ingredient}/payload /payload
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    let output = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&output[0].store_path).join("payload")).unwrap(),
        "payload"
    );
}

#[test]
fn newly_consumed_fetch_path_extends_an_automatic_pin() {
    let directory = tempfile::tempdir().unwrap();
    let cixfile = |extra_copy: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
FETCH printf one > one; printf two > two
RUN cp one result
SERVICE result
COPY ${{build}}/result /result
{extra_copy}EXEC /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("")).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let options = BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    };
    build(&options).unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        cixfile("COPY ${build}/two /two\n"),
    )
    .unwrap();
    build(&options).unwrap();

    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    assert_eq!(
        lock.fetches
            .values()
            .next()
            .unwrap()
            .paths
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["result", "two"]
    );
}

#[test]
fn update_lock_ignores_unconsumed_timestamped_fetch_output() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils} ${pkgs.findutils}
FETCH mkdir -p .npm/_logs; date +%s%N > .npm/_logs/$(date +%s%N)-debug.log; find .npm/_logs -type f -print >/dev/null; printf payload > result
SERVICE result
COPY ${build}/result /result
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
    })
    .unwrap();
    let first_lock = fs::read(directory.path().join("Cixfile.lock")).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        first_lock,
        fs::read(directory.path().join("Cixfile.lock")).unwrap()
    );
    let lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let pin = lock.fetches.values().next().unwrap();
    assert_eq!(
        pin.paths.keys().map(String::as_str).collect::<Vec<_>>(),
        ["result"]
    );
    assert!(pin.volatile.is_empty());
}

#[test]
fn warm_source_edit_after_fetch_reuses_the_pinned_prefix() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("manifest"), "manifest\n").unwrap();
    fs::write(directory.path().join("source"), "v1\n").unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("manifest"), "manifest\n").unwrap();
    fs::write(expected_tree.path().join("payload"), "fixed\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    fs::write(
        directory.path().join("Cixfile"),
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
COPY ${{src}}/manifest manifest
FETCH EXPECT {expected} printf 'fixed\n' > payload
COPY ${{src}}/source source
RUN cp source output
SERVICE result
COPY ${{build}}/output /output
EXEC /bin/true
"#
        ),
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("output")).unwrap(),
        "v1\n"
    );

    fs::write(directory.path().join("source"), "v2\n").unwrap();
    let second = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&second[0].store_path).join("output")).unwrap(),
        "v2\n"
    );
}

#[test]
fn changed_step_before_fetch_reuses_its_builder_underlay() {
    let directory = tempfile::tempdir().unwrap();
    let expected_tree = tempfile::tempdir().unwrap();
    fs::write(expected_tree.path().join("required"), "present\n").unwrap();
    let expected = cix_common::nix(&[
        "hash",
        "path",
        "--mode",
        "nar",
        expected_tree.path().to_str().unwrap(),
    ])
    .unwrap()
    .trim()
    .to_owned();
    let cixfile = |middle: &str| {
        format!(
            r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
BUILDER build
IMPORT ${{pkgs.bash}} ${{pkgs.coreutils}}
RUN printf 'present\n' > required
RUN {middle}
FETCH EXPECT {expected} test -f required
SERVICE result
COPY ${{build}}/required /required
EXEC /bin/true
"#
        )
    };
    fs::write(directory.path().join("Cixfile"), cixfile("true")).unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();

    fs::write(directory.path().join("Cixfile"), cixfile("test 1 = 1")).unwrap();
    let rebuilt = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&rebuilt[0].store_path).join("required")).unwrap(),
        "present\n"
    );
}

#[test]
fn warm_rerun_starts_on_its_builder_end_state_while_cold_does_not() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("source"), "v1\n").unwrap();
    fs::write(
        directory.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM . AS src
BUILDER build
IMPORT ${pkgs.bash} ${pkgs.coreutils}
RUN printf 'before\n' >> history
COPY ${src}/source source
RUN cat source >> history && cp history output
SERVICE result
COPY ${build}/output /output
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        directory.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();

    let first = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&first[0].store_path).join("output")).unwrap(),
        "before\nv1\n"
    );

    fs::write(directory.path().join("source"), "v2\n").unwrap();
    let warm = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(PathBuf::from(&warm[0].store_path).join("output")).unwrap(),
        "before\nv1\nv2\n"
    );

    let cold = build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: true,
    })
    .unwrap_err()
    .to_string();
    assert!(
        cold.contains("COPY ${build}/output") && cold.contains("differs between warm and cold"),
        "{cold}"
    );
}

#[test]
fn bare_and_explicit_local_copy_contexts_are_byte_identical() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("payload"), "same context\n").unwrap();
    let bare = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE fixture\nCOPY payload /share/payload\nEXEC /bin/true\n",
    )
    .unwrap();
    let explicit = parse(
        "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\nSERVICE fixture\nCOPY ${src}/payload /share/payload\nEXEC /bin/true\n",
    )
    .unwrap();
    let bare = build_expression(
        &generate_nix(&bare, directory.path(), &committed_lock(), "x86_64-linux").unwrap(),
    )
    .unwrap();
    let explicit = build_expression(
        &generate_nix(
            &explicit,
            directory.path(),
            &committed_lock(),
            "x86_64-linux",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(nar_hash(&bare), nar_hash(&explicit));
    assert_eq!(
        fs::read_to_string(bare.join("share/payload")).unwrap(),
        "same context\n"
    );
    let manifest = cix_run::spec::Spec::load(&bare).unwrap();
    assert_eq!(manifest.kind, cix_run::spec::ManifestKind::Service);
}

#[test]
fn cix_item_from_copies_a_lock_pinned_tag_and_rejects_a_bad_nar_hash() {
    let state = tempfile::tempdir().unwrap();
    std::env::set_var("CIX_STATE_DIR", state.path());

    let missing = tempfile::tempdir().unwrap();
    fs::write(
        missing.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM family/missing:v1 AS missing
SERVICE consumer
COPY ${missing}/payload /payload
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        missing.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let missing_error = build(&BuildOptions {
        directory: missing.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap_err()
    .to_string();
    assert!(
        missing_error.contains("pull it or tag it first"),
        "{missing_error}"
    );

    let producer = tempfile::tempdir().unwrap();
    fs::write(
        producer.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
ITEM source
FILE /payload <<EOF
first
EOF
"#,
    )
    .unwrap();
    fs::write(
        producer.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let producer_output = build(&BuildOptions {
        directory: producer.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap()
    .remove(0)
    .store_path;
    assert!(!Path::new(&producer_output)
        .join("cix-manifest.json")
        .exists());
    let manifest_error = cix_run::spec::Spec::load(Path::new(&producer_output))
        .unwrap_err()
        .to_string();
    assert!(
        manifest_error.contains("manifest-less ITEM (D68)"),
        "{manifest_error}"
    );
    assert!(manifest_error.contains("SERVICE/APP"), "{manifest_error}");
    cix_index::tag(&producer_output, "family/source:v1", None).unwrap();

    let consumer = tempfile::tempdir().unwrap();
    fs::write(
        consumer.path().join("Cixfile"),
        r#"FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs
FROM family/source:v1 AS source
SERVICE consumer
COPY ${source}/payload /payload
EXEC /bin/true
"#,
    )
    .unwrap();
    fs::write(
        consumer.path().join("Cixfile.lock"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&committed_lock()).unwrap()
        ),
    )
    .unwrap();
    let first = build(&BuildOptions {
        directory: consumer.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&first[0].store_path).join("payload")).unwrap(),
        "first\n"
    );
    let mut lock: LockFile =
        serde_json::from_slice(&fs::read(consumer.path().join("Cixfile.lock")).unwrap()).unwrap();
    let pin = lock.artifacts["family/source:v1"].clone();
    assert_eq!(pin.store_path, producer_output);

    let moved_tree = tempfile::tempdir().unwrap();
    fs::write(moved_tree.path().join("payload"), "moved\n").unwrap();
    let moved = cix_common::nix(&["store", "add-path", moved_tree.path().to_str().unwrap()])
        .unwrap()
        .trim()
        .to_owned();
    cix_index::tag(&moved, "family/source:v1", None).unwrap();

    let pinned = build(&BuildOptions {
        directory: consumer.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&pinned[0].store_path).join("payload")).unwrap(),
        "first\n"
    );

    let updated = build(&BuildOptions {
        directory: consumer.path().to_owned(),
        update_lock: Some("source".into()),
        tag: None,
        cold: false,
    })
    .unwrap();
    assert_eq!(
        fs::read_to_string(Path::new(&updated[0].store_path).join("payload")).unwrap(),
        "moved\n"
    );

    lock.artifacts.insert(
        "family/source:v1".into(),
        ArtifactPin {
            store_path: moved,
            nar_hash: "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
        },
    );
    fs::write(
        consumer.path().join("Cixfile.lock"),
        format!("{}\n", serde_json::to_string_pretty(&lock).unwrap()),
    )
    .unwrap();
    let error = build(&BuildOptions {
        directory: consumer.path().to_owned(),
        update_lock: None,
        tag: None,
        cold: false,
    })
    .unwrap_err()
    .to_string();
    assert!(error.contains("narHash mismatch"), "{error}");
}

fn nar_hash(path: &std::path::Path) -> String {
    let output = Command::new("nix-store")
        .args(["--query", "--hash"])
        .arg(path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}
