use std::{fs, path::PathBuf, process::Command};

fn stack() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/compose/stack")
}

#[test]
fn generator_is_byte_identical_to_hand_written_compose() {
    let output = Command::new("python3")
        .arg(stack().join("generate.py"))
        .output()
        .expect("run compose generator");
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        fs::read(stack().join("compose.json")).unwrap()
    );
}

#[test]
fn real_stack_demo_is_root_gated() {
    let uid = Command::new("id").arg("-u").output().unwrap();
    if String::from_utf8_lossy(&uid.stdout).trim() != "0" {
        eprintln!("skipping compose stack integration test: requires root");
        return;
    }
    let Some(cix) = std::env::var_os("CIX_BIN") else {
        eprintln!("skipping compose stack integration test: root run requires CIX_BIN");
        return;
    };
    let status = Command::new(stack().join("demo.sh"))
        .env("CIX_BIN", cix)
        .status()
        .expect("run compose stack demo");
    assert!(status.success());
}
