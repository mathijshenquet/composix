use std::fs;
use std::path::{Path, PathBuf};

use cix_cixfile::{fmt, parse};

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
    let input = "# comment  \r\nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\r\nSERVICE app\r\nFILE /etc/app.conf <<EOF\r\nbody  \r\nEOF\r\nEXEC /bin/true\r\n";
    assert_eq!(
        fmt::format(input).unwrap(),
        "# comment  \nFROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\n\nSERVICE app\n  FILE /etc/app.conf <<EOF\nbody  \nEOF\n  EXEC /bin/true\n"
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
