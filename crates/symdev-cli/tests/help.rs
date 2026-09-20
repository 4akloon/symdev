use predicates::prelude::*;

mod common;
use common::bin;

#[test]
fn help_lists_only_the_seven_commands() {
    // `run` (EKA2L1, M5) joined the §17 four on 2026-09-19, `freeze` (DLL exports,
    // experiment 54) the same day, and `test` (the result protocol, experiment 79) on
    // 2026-09-20; the other north-star verbs are still not subcommands.
    let assert = bin().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for cmd in ["new", "build", "package", "deploy", "run", "freeze", "test"] {
        assert!(stdout.contains(cmd), "missing {cmd}: {stdout}");
    }
    for cmd in ["doctor", "debug", "sdk", "toolchain", "emulator", "devices"] {
        assert!(!stdout.contains(cmd), "help must not list {cmd}: {stdout}");
    }
    assert!(
        !stdout
            .lines()
            .any(|line| line.trim_start() == "help" || line.trim_start().starts_with("help ")),
        "help must not be listed as a subcommand: {stdout}"
    );
}

#[test]
fn no_args_prints_help() {
    bin()
        .assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("build"));
}

#[test]
fn doctor_is_unknown_subcommand() {
    bin().arg("doctor").assert().failure().code(2);
}
