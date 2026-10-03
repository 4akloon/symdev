use std::path::Path;

use crate::EmulatorData;
use crate::device::EmulatorProfile;

#[test]
fn a_profile_references_the_rom_and_drive_z_and_owns_c_d_e() {
    let user = tempfile::tempdir().unwrap();
    let d = user.path().join("EKA2L1/data");
    for p in [
        "drives/c/private",
        "drives/d",
        "drives/e",
        "drives/z/sys",
        "roms/rm-469",
    ] {
        std::fs::create_dir_all(d.join(p)).unwrap();
    }
    std::fs::write(d.join("devices.yml"), "RM-469:\n  firmcode: RM-469\n").unwrap();
    std::fs::write(
        user.path().join("EKA2L1/config.yml"),
        "log-filter: \"*:trace Emulated.Stdout:off\"\nother: 1\n",
    )
    .unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&EmulatorData::at(&user.path().join("EKA2L1")), "rm-469")
        .unwrap();
    let data = p.dir().join("data");
    assert_eq!(
        std::fs::read_link(data.join("roms/rm-469")).unwrap(),
        d.join("roms/rm-469")
    );
    assert_eq!(
        std::fs::read_link(data.join("drives/z")).unwrap(),
        d.join("drives/z")
    );
    assert!(data.join("drives/c/private").is_dir() && data.join("drives/e").is_dir());
    let config = std::fs::read_to_string(p.dir().join("config.yml")).unwrap();
    assert!(
        config.contains("log-filter: \"*:info Emulated.Stdout:trace Kernel:trace\""),
        "{config}"
    );
    assert!(config.contains("other: 1"));
    assert_eq!(
        p.data().result_file(0xe1234567),
        data.join("drives/e/symdev/results/e1234567.json")
    );
    let again = p
        .create(&EmulatorData::at(&user.path().join("EKA2L1")), "rm-469")
        .unwrap_err();
    assert!(again.to_string().contains("already exists"), "{again}");
}

#[test]
fn a_firmware_the_user_has_not_installed_is_named() {
    let user = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let e = EmulatorProfile::at(root.path(), "rm-469")
        .create(&EmulatorData::at(user.path()), "rm-469")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("rm-469") && e.contains("install the firmware in EKA2L1"),
        "{e}"
    );
}

#[test]
fn the_log_is_the_instances_own_and_the_root_follows_xdg() {
    let p = EmulatorProfile::at(Path::new("/r"), "rm-469");
    assert_eq!(p.dir(), Path::new("/r/rm-469"));
    assert_eq!(p.name(), "rm-469");
    assert_eq!(p.log_file(), Path::new("/r/rm-469/EKA2L1.log"));
    assert_eq!(
        EmulatorProfile::root_in(Some("/x/data".into()), Some("/h".into())).unwrap(),
        Path::new("/x/data/symdev/emulators")
    );
    assert_eq!(
        EmulatorProfile::root_in(None, Some("/h".into())).unwrap(),
        Path::new("/h/.local/share/symdev/emulators")
    );
    assert!(EmulatorProfile::root_in(None, None).is_err());
}

#[test]
fn a_link_in_drive_c_that_leaves_it_is_refused() {
    let user = tempfile::tempdir().unwrap();
    let d = user.path().join("EKA2L1/data");
    for p in ["drives/c", "drives/z", "roms/rm-469"] {
        std::fs::create_dir_all(d.join(p)).unwrap();
    }
    std::fs::write(d.join("devices.yml"), "").unwrap();
    std::fs::write(user.path().join("EKA2L1/config.yml"), "").unwrap();
    std::os::unix::fs::symlink("/etc", d.join("drives/c/out")).unwrap();
    let root = tempfile::tempdir().unwrap();
    let e = EmulatorProfile::at(root.path(), "rm-469")
        .create(&EmulatorData::at(&user.path().join("EKA2L1")), "rm-469")
        .unwrap_err()
        .to_string();
    assert!(e.contains("out") && e.contains("link"), "{e}");
}
