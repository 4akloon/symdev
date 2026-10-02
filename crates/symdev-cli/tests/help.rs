use predicates::prelude::*;

mod common;
use common::bin;

#[test]
fn help_lists_only_the_eight_commands() {
    // `run` (EKA2L1, M5) joined the §17 four on 2026-09-19, `freeze` (DLL exports,
    // experiment 54) the same day, `test` (the result protocol, experiment 79) on
    // 2026-09-20 and `sdk` (the toolchain manager) on 2026-10-02; the other north-star
    // verbs are still not subcommands.
    let assert = bin().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let commands: Vec<_> = stdout
        .lines()
        .skip_while(|line| *line != "Commands:")
        .skip(1)
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next())
        .collect();
    assert_eq!(
        commands,
        [
            "new", "build", "package", "deploy", "run", "test", "freeze", "sdk"
        ],
        "{stdout}"
    );
    assert!(stdout.contains("--offline"), "{stdout}");
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
