use std::process::Command;

#[test]
fn logs_explain_prints_the_raw_journalctl_selector() {
    let output = Command::new(env!("CARGO_BIN_EXE_cix"))
        .args([
            "logs",
            "acme/api",
            "--since",
            "yesterday",
            "-n",
            "20",
            "--invocation",
            "0123456789abcdef0123456789abcdef",
            "--explain",
        ])
        .output()
        .expect("run cix logs --explain");
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "journalctl --since yesterday -n 20 CIX_COMPOSITE=acme CIX_SERVICE=api _SYSTEMD_INVOCATION_ID=0123456789abcdef0123456789abcdef\n"
    );
}
