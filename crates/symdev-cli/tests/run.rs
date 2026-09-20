use predicates::prelude::*;

mod common;
use common::{HELLO, bin, write_toml};

#[test]
fn run_without_sisx_asks_for_package() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EKA2L1", "/emu/eka2l1")
        .arg("run")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("run symdev package"));
}

#[test]
fn run_without_emulator_names_symdev_eka2l1() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    std::fs::create_dir(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/hello.sisx"), b"sisx").unwrap();
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EKA2L1")
        .arg("run")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("SYMDEV_EKA2L1"));
}
