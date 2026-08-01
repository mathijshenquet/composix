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

    let original_workspace = directory.path().join("original-workspaces");
    std::env::set_var("CIX_BUILD_WORKSPACE_DIR", &original_workspace);
    fs::write(directory.path().join("Cixfile.lock"), &clean_lock).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
    })
    .unwrap();
    let original_lock = fs::read(directory.path().join("Cixfile.lock")).unwrap();

    let formatted = fmt::format(COPY_KEYING_FIXTURE).unwrap();
    fs::write(directory.path().join("Cixfile"), formatted).unwrap();
    let formatted_workspace = directory.path().join("formatted-workspaces");
    std::env::set_var("CIX_BUILD_WORKSPACE_DIR", &formatted_workspace);
    fs::write(directory.path().join("Cixfile.lock"), clean_lock).unwrap();
    build(&BuildOptions {
        directory: directory.path().to_owned(),
        update_lock: Some("build".into()),
        tag: None,
        cold: false,
    })
    .unwrap();

    assert_eq!(
        fs::read(directory.path().join("Cixfile.lock")).unwrap(),
        original_lock
    );
}
