#[cfg(unix)]
#[test]
fn real_cli_restores_terminal_and_preserves_piped_input() {
    assert_cmd::Command::new("python3")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/pty_smoke.py"))
        .arg("--binary")
        .arg(assert_cmd::cargo::cargo_bin!("showme"))
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success();
}
