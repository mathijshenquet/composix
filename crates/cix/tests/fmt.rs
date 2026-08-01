use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

fn cix() -> Command {
    Command::new(env!("CARGO_BIN_EXE_cix"))
}

const MESSY: &str =
    "FROM\tgithub:NixOS/nixpkgs/nixos-unstable\tAS\tpkgs\nSERVICE\tapp\nEXEC\t/bin/true\n";
const CANONICAL: &str =
    "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\n\nSERVICE app\n  EXEC /bin/true\n";

#[test]
fn check_explains_changes_and_unchanged_files_are_not_rewritten() {
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("Cixfile");
    fs::write(&file, MESSY).unwrap();

    let check = cix().arg("fmt").arg("--check").arg(&file).output().unwrap();
    assert_eq!(check.status.code(), Some(1));
    assert!(String::from_utf8(check.stdout).unwrap().contains("@@"));
    assert_eq!(fs::read_to_string(&file).unwrap(), MESSY);

    assert!(cix().arg("fmt").arg(&file).status().unwrap().success());
    assert_eq!(fs::read_to_string(&file).unwrap(), CANONICAL);
    let modified = fs::metadata(&file).unwrap().modified().unwrap();
    let check = cix().arg("fmt").arg("--check").arg(&file).output().unwrap();
    assert!(check.status.success());
    assert!(check.stdout.is_empty());
    assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), modified);
}

#[test]
fn stdin_and_gitignore_discovery_follow_the_cli_contract() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
    fs::create_dir_all(root.join("ignored")).unwrap();
    fs::create_dir_all(root.join("included")).unwrap();
    fs::write(root.join("ignored/Cixfile"), MESSY).unwrap();
    fs::write(root.join("included/Cixfile"), MESSY).unwrap();

    assert!(cix().arg("fmt").arg(root).status().unwrap().success());
    assert_eq!(
        fs::read_to_string(root.join("ignored/Cixfile")).unwrap(),
        MESSY
    );
    assert_eq!(
        fs::read_to_string(root.join("included/Cixfile")).unwrap(),
        CANONICAL
    );
    assert!(cix()
        .arg("fmt")
        .arg(root.join("ignored/Cixfile"))
        .status()
        .unwrap()
        .success());
    assert_eq!(
        fs::read_to_string(root.join("ignored/Cixfile")).unwrap(),
        CANONICAL
    );

    let mut stdin = cix()
        .arg("fmt")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    stdin
        .stdin
        .take()
        .unwrap()
        .write_all(MESSY.as_bytes())
        .unwrap();
    let output = stdin.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), CANONICAL);
}

#[test]
fn parse_failures_write_nothing_and_stdin_cannot_be_mixed() {
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("Cixfile");
    let invalid = "FROM github:NixOS/nixpkgs/nixos-unstable AS pkgs\nSERVICE app\n";
    fs::write(&file, invalid).unwrap();

    let output = cix().arg("fmt").arg(&file).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("has no EXEC"));
    assert_eq!(fs::read_to_string(&file).unwrap(), invalid);

    let output = cix().arg("fmt").arg("-").arg(&file).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot be combined"));
}
