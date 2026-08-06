use std::fs;
use std::path::{Path, PathBuf};

use cix_cixfile::{build, fmt, parse, BuildOptions, LockFile};

const COPY_KEYING_FIXTURE: &str = "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nFROM . AS src\n\nBUILDER build\nCOPY ${src}/input.txt .\n\nITEM result\nCOPY ${build}/input.txt /input.txt\n";

#[test]
fn golden_messy_input_has_the_v1_canon() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fmt");
    let input = fs::read_to_string(directory.join("messy.cix")).unwrap();
    let expected = fs::read_to_string(directory.join("messy.expected")).unwrap();
    let formatted = fmt::format(&input).unwrap();

    assert_eq!(formatted, expected);
    assert_eq!(fmt::format(&formatted).unwrap(), formatted);
    assert!(fmt::same_semantics(
        &parse(&input).unwrap(),
        &parse(&formatted).unwrap()
    ));
}

#[test]
fn crlf_is_normalized_but_heredoc_and_comments_are_untouched() {
    let input = "# comment  \r\nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\r\nSERVICE app\r\nFILE /etc/app.conf <<EOF\r\nbody  \r\nEOF\r\nSTART /bin/true\r\n";
    assert_eq!(
        fmt::format(input).unwrap(),
        "# comment  \nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\n\nSERVICE app\n  FILE /etc/app.conf <<EOF\nbody  \nEOF\n  START /bin/true\n"
    );
}

#[test]
fn top_fetch_interpreter_heredoc_remains_outside_following_blocks() {
    let input = "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nARG FLAVOR from plain debug\nFETCH ingredient ${pkgs.bash}/bin/bash <<BODY\nprintf '%s' '${FLAVOR}' > flavor\nBODY\nBUILDER build {\nRUN printf ok\n}\nITEM result {\nCOPY ${build} /build\nCOPY ${ingredient} /ingredient\n}\n";
    let formatted = fmt::format(input).unwrap();

    assert_eq!(fmt::format(&formatted).unwrap(), formatted);
    assert!(fmt::same_semantics(
        &parse(input).unwrap(),
        &parse(&formatted).unwrap()
    ));
    assert!(formatted.contains("\nprintf '%s' '${FLAVOR}' > flavor\nBODY\n"));
}

#[test]
fn torture_sweep_is_parse_gated_idempotent_and_semantic_preserving() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/torture");
    let mut fixtures = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cix"))
        .collect::<Vec<_>>();
    fixtures.sort();

    for path in fixtures {
        let input = fs::read_to_string(&path).unwrap();
        match parse(&input) {
            Ok(parsed) => {
                let formatted = fmt::format(&input).unwrap();
                assert_eq!(
                    fmt::format(&formatted).unwrap(),
                    formatted,
                    "{}",
                    path.display()
                );
                assert!(
                    fmt::same_semantics(&parsed, &parse(&formatted).unwrap()),
                    "{}",
                    path.display()
                );
            }
            Err(error) => assert_eq!(
                fmt::format(&input).unwrap_err(),
                error,
                "{}",
                path.display()
            ),
        }
    }
}

#[test]
fn formatting_preserves_builder_keys_and_clean_update_lock() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("Cixfile"), COPY_KEYING_FIXTURE).unwrap();
    fs::write(
        directory.path().join("input.txt"),
        "formatter key regression\n",
    )
    .unwrap();
    let mut lock: LockFile =
        serde_json::from_str(include_str!("../../../examples/pack/nginx/Cixfile.lock")).unwrap();
    lock.artifacts.clear();
    lock.fetches.clear();
    lock.memo.clear();
    let clean_lock = format!("{}\n", serde_json::to_string_pretty(&lock).unwrap());

    let original_workspace = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("Cixfile.lock"), &clean_lock).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: original_workspace.path().to_owned(),
    })
    .unwrap();
    let original_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();

    let formatted = fmt::format(COPY_KEYING_FIXTURE).unwrap();
    fs::write(directory.path().join("Cixfile"), formatted).unwrap();
    let formatted_workspace = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("Cixfile.lock"), clean_lock).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
        allow_secret: false,
        workspace_directory: formatted_workspace.path().to_owned(),
    })
    .unwrap();

    let mut formatted_lock: LockFile =
        serde_json::from_slice(&fs::read(directory.path().join("Cixfile.lock")).unwrap()).unwrap();
    let original_cixfile_hash = &original_lock
        .eval_plan
        .as_ref()
        .expect("original build records an eval plan")
        .cixfile_hash;
    let formatted_eval_plan = formatted_lock
        .eval_plan
        .as_mut()
        .expect("formatted build records an eval plan");
    assert_ne!(formatted_eval_plan.cixfile_hash, *original_cixfile_hash);
    formatted_eval_plan.cixfile_hash = original_cixfile_hash.clone();
    assert_eq!(formatted_lock, original_lock);
}
