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

/// A project's package, a profile to start (made from a firmware in the user's EKA2L1),
/// and no `SYMDEV_EKA2L1`: starting the emulator is what fails, by name.
#[test]
fn run_with_a_profile_to_start_and_no_emulator_names_symdev_eka2l1() {
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
    let data = dir.path().join("xdg-data");
    let user = data.join("EKA2L1");
    for d in ["data/drives/c", "data/drives/z", "data/roms/rm-469"] {
        std::fs::create_dir_all(user.join(d)).unwrap();
    }
    std::fs::write(user.join("data/devices.yml"), "").unwrap();
    std::fs::write(user.join("config.yml"), "").unwrap();
    bin()
        .current_dir(&dir)
        .env("XDG_DATA_HOME", &data)
        .env("SYMDEV_EKA2L1_DATA", &user)
        .arg("run")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("created profile rm-469"))
        .stderr(predicate::str::contains("SYMDEV_EKA2L1"));
}

/// No running emulator, no profile, no firmware to make one from.
#[test]
fn run_without_any_device_says_to_install_a_firmware() {
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
        .env("XDG_DATA_HOME", dir.path().join("empty"))
        .arg("run")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("install a firmware in EKA2L1"));
}
