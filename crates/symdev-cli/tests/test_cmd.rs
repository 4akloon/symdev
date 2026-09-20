//! `symdev test`: the refusals a caller can hit before an emulator is ever started.
//! The emulator half of the command is exercised by running `symbian-rs/examples/files`
//! (experiment 79), not from a host test.
use predicates::prelude::*;

mod common;
use common::{HELLO, bin, write_toml};

/// A manifest with a UID3, which `test` needs to name the result file.
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    dir
}

#[test]
fn test_without_emulator_says_there_is_no_other_backend() {
    // Bind the temporary: `current_dir(project())` drops it, and the directory with it,
    // before the command is spawned.
    let dir = project();
    bin()
        .current_dir(&dir)
        .arg("test")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("--emulator"));
}

#[test]
fn test_without_sisx_asks_for_package() {
    let dir = project();
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EKA2L1", "/emu/eka2l1")
        .args(["test", "--emulator"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("symdev package"));
}

#[test]
fn test_without_emulator_binary_names_symdev_eka2l1() {
    let dir = project();
    std::fs::create_dir(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/hello.sisx"), b"sisx").unwrap();
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EKA2L1")
        .env("SYMDEV_EKA2L1_DATA", dir.path())
        .args(["test", "--emulator"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("SYMDEV_EKA2L1"));
}
