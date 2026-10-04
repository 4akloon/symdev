//! `symdev emulator start` and `symdev devices` against a `file://` source holding the
//! emulator and firmware packages (emulator packages spec §5, Review Focus 2, 4, 5).
use std::path::{Path, PathBuf};

use predicates::prelude::*;
use symdev_sdk::{Host, Pins};

mod common;
use common::bin;
use common::repo::World;

const DEVICE_YML: &str = "RM-469:\n  platver: epoc93fp2\n  firmcode: RM-469\n";

fn with_firmware(world: &mut World) {
    world.add(
        "firmware;rm-469;1",
        Host::Any,
        &[
            ("device.yml", DEVICE_YML, false),
            ("roms/rm-469/SYM.ROM", "rom", false),
            ("drives/z/rm-469/sys/bin/avkonfep.dll.bak", "dll", false),
        ],
    );
}

/// An EKA2L1 whose `--help` lists `--data-dir` only: one without the control server.
fn old_eka2l1(dir: &Path) -> PathBuf {
    let path = dir.join("old-eka2l1");
    std::fs::write(&path, "#!/bin/sh\necho '  --data-dir <dir>'\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn profile(world: &World) -> PathBuf {
    world.tmp.path().join("data/symdev/emulators/rm-469")
}

#[test]
fn emulator_start_installs_the_firmware_package_and_makes_its_profile() {
    let mut world = World::new();
    with_firmware(&mut world);
    world
        .bin()
        .env("SYMDEV_EKA2L1", old_eka2l1(world.tmp.path()))
        .args(["emulator", "start", "rm-469"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("installing firmware;rm-469;1"))
        .stderr(predicate::str::contains("created profile rm-469"));
    let package = world.package_dir("firmware;rm-469;1");
    let data = profile(&world).join("data");
    assert_eq!(
        std::fs::read_link(data.join("roms/rm-469")).unwrap(),
        package.join("roms/rm-469")
    );
    assert_eq!(
        std::fs::read_link(data.join("drives/z")).unwrap(),
        package.join("drives/z")
    );
    assert_eq!(
        std::fs::read_to_string(data.join("devices.yml")).unwrap(),
        DEVICE_YML
    );
}

#[test]
fn an_old_symdev_eka2l1_is_named_with_the_way_to_the_package() {
    let mut world = World::new();
    with_firmware(&mut world);
    world
        .bin()
        .env("SYMDEV_EKA2L1", old_eka2l1(world.tmp.path()))
        .args(["emulator", "start", "rm-469"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("SYMDEV_EKA2L1 ("))
        .stderr(predicate::str::contains("has no --control"))
        .stderr(predicate::str::contains(format!(
            "unset SYMDEV_EKA2L1 to use the {} package",
            Pins::emulator()
        )));
}

#[test]
fn without_symdev_eka2l1_the_emulator_package_is_installed_and_probed() {
    let mut world = World::new();
    with_firmware(&mut world);
    let emulator = Pins::emulator();
    world.add(
        emulator.as_str(),
        Host::X86_64Linux,
        &[(
            "usr/bin/eka2l1_qt",
            "#!/bin/sh\necho '  --data-dir <dir>'\n",
            true,
        )],
    );
    world
        .bin()
        .args(["emulator", "start", "rm-469"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!("installing {emulator}")))
        .stderr(predicate::str::contains("the emulator package's"))
        .stderr(predicate::str::contains(
            "usr/bin/eka2l1_qt has no --control",
        ));
}

#[test]
fn the_default_eka2l1_folder_is_not_read_without_symdev_eka2l1_data() {
    let world = World::new();
    let default = world.tmp.path().join("data/EKA2L1");
    std::fs::create_dir_all(default.join("data/roms/rm-469")).unwrap();
    std::fs::write(default.join("data/devices.yml"), DEVICE_YML).unwrap();
    let before: Vec<_> = walk(&default);
    world
        .bin()
        .args(["emulator", "start", "rm-469"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("firmware;rm-469;1 was not found"))
        .stderr(predicate::str::contains("SYMDEV_EKA2L1_DATA"));
    assert!(!profile(&world).exists());
    assert_eq!(walk(&default), before);
}

#[test]
fn an_existing_profile_needs_no_firmware_package() {
    let home = tempfile::tempdir().unwrap();
    let emulators = home.path().join("data/symdev/emulators/rm-469");
    std::fs::create_dir_all(&emulators).unwrap();
    bin()
        .env("XDG_DATA_HOME", home.path().join("data"))
        .args(["--offline", "devices"])
        .assert()
        .success()
        .stdout(predicate::str::contains("profile rm-469"))
        .stderr(predicate::str::contains("installing").not());
}

#[test]
fn devices_with_no_profile_and_no_firmware_lists_nothing_and_asks_nobody() {
    let world = World::new();
    let emulators = world.tmp.path().join("data/symdev/emulators");
    // A half-made profile of a run that died is not a profile.
    std::fs::create_dir_all(emulators.join("rm-469.partial-42")).unwrap();
    world
        .bin()
        .args(["devices"])
        .assert()
        .success()
        .stdout("")
        .stderr(predicate::str::contains("installing").not())
        .stderr(predicate::str::contains("created profile").not());
    assert!(!profile(&world).exists());
}

/// Every path under `dir` with its modification time, sorted.
fn walk(dir: &Path) -> Vec<(PathBuf, std::time::SystemTime)> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let meta = e.metadata().unwrap();
            if meta.is_dir() {
                todo.push(e.path());
            }
            found.push((e.path(), meta.modified().unwrap()));
        }
    }
    found.sort();
    found
}
