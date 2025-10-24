use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_displays_usage() {
    let mut cmd = Command::cargo_bin("terminal-media").expect("binary exists");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Terminal image viewer"));
}
