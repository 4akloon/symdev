//! `common::bin()` keeps the developer's shell and earlier test runs out of a test. The
//! only test of this binary: it changes the process environment, and it must run before
//! the first `bin()` of the process, which is when old homes are swept.

use std::fs::{self, File};
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;

mod common;

#[test]
fn bin_hides_the_shell_and_sweeps_the_homes_of_finished_runs() {
    let homes = Path::new(env!("CARGO_TARGET_TMPDIR")).join("symdev-cli-homes");
    let finished = homes.join("home-finished");
    let running = homes.join("home-running");
    for dir in [&finished, &running] {
        fs::create_dir_all(dir).unwrap();
        File::create(dir.join(".lock")).unwrap();
    }
    let held = File::open(running.join(".lock")).unwrap();
    held.lock().unwrap();
    // SAFETY: the only test of this binary; no other thread reads the environment.
    unsafe {
        std::env::set_var("SYMDEV_GXX", "/nonexistent/symdev/g++");
        std::env::set_var("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "AKID");
        std::env::set_var("PUBLISH_SECRET_ACCESS_KEY", "secret");
    }
    common::isolated(Command::new("env"))
        .assert()
        .success()
        .stdout(predicate::str::contains("SYMDEV_HOME="))
        .stdout(predicate::str::contains("SYMDEV_GXX").not())
        .stdout(predicate::str::contains("SYMDEV_SOURCE_").not())
        .stdout(predicate::str::contains("PUBLISH_").not());
    assert!(!finished.exists(), "a home nobody holds is swept");
    assert!(running.exists(), "a home whose process runs is kept");
    drop(held);
    fs::remove_dir_all(&running).unwrap();
}
