use std::path::Path;

use crate::device::{EmulatorProfile, Firmware};

fn package(root: &Path) -> Firmware {
    for d in ["roms/rm-469", "drives/z/rm-469/sys/bin"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("device.yml"), "RM-469:\n").unwrap();
    Firmware::Package {
        root: root.to_path_buf(),
        name: "rm-469".into(),
    }
}

#[test]
fn a_half_made_folder_is_not_a_profile() {
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    std::fs::create_dir_all(p.dir().join("data/drives/d")).unwrap();
    let e = p.check().unwrap_err().to_string();
    assert!(e.contains("rm-469") && e.contains("remove"), "{e}");
}

#[test]
fn a_profile_without_roms_is_refused_with_the_fix() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    std::fs::remove_file(p.dir().join("data/roms/rm-469")).unwrap();
    let e = p.check().unwrap_err().to_string();
    assert!(e.contains("data/roms") && e.contains("remove"), "{e}");
}

#[test]
fn creating_leaves_no_partial_folder_and_an_existing_profile_is_not_a_race_loss() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    let names: Vec<String> = std::fs::read_dir(root.path())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["rm-469"]);
    p.check().unwrap();
}

#[test]
fn a_failed_create_leaves_no_folder_at_all() {
    let user = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    let failed = p.create(&Firmware::Package {
        root: user.path().to_path_buf(),
        name: "rm-469".into(),
    });
    assert!(failed.is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn the_gone_message_names_the_firmware_id_and_the_install_command() {
    let pkg = tempfile::tempdir().unwrap();
    let home = pkg.path().join("firmware/rm-469/1");
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(&home)).unwrap();
    std::fs::remove_dir_all(&home).unwrap();
    let e = p.check().unwrap_err().to_string();
    assert!(e.contains("firmware;rm-469;1"), "{e}");
    assert!(e.contains("symdev sdk install 'firmware;rm-469;1'"), "{e}");
}
