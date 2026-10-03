use std::path::Path;

use crate::EmulatorData;
use crate::device::{DeviceRegistry, Eka2l1, EmulatorInstance, EmulatorProfile, Firmware};

/// An installed `firmware;rm-469;1` as Task 5's stage.sh makes it.
fn package(root: &Path) -> Firmware {
    for d in ["roms/rm-469", "drives/z/rm-469/sys/bin"] {
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
    std::fs::write(root.join("roms/rm-469/SYM.ROM"), b"rom").unwrap();
    std::fs::write(root.join("device.yml"), "RM-469:\n  firmcode: RM-469\n").unwrap();
    Firmware::Package {
        root: root.to_path_buf(),
        name: "rm-469".into(),
    }
}

#[test]
fn a_profile_from_a_package_links_its_rom_and_drive_z_and_starts_with_empty_drives() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    let data = p.dir().join("data");
    assert_eq!(
        std::fs::read_link(data.join("roms/rm-469")).unwrap(),
        pkg.path().join("roms/rm-469")
    );
    assert_eq!(
        std::fs::read_link(data.join("drives/z")).unwrap(),
        pkg.path().join("drives/z")
    );
    for d in ["c", "d", "e"] {
        let drive = data.join("drives").join(d);
        assert!(
            drive.is_dir() && std::fs::read_dir(&drive).unwrap().next().is_none(),
            "{d}"
        );
    }
}

#[test]
fn a_profile_from_a_package_lists_the_packages_device_and_only_symdevs_log_filter() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    assert_eq!(
        std::fs::read_to_string(p.dir().join("data/devices.yml")).unwrap(),
        "RM-469:\n  firmcode: RM-469\n"
    );
    assert_eq!(
        std::fs::read_to_string(p.dir().join("config.yml")).unwrap(),
        "log-filter: \"*:info Emulated.Stdout:trace Kernel:trace\"\n"
    );
}

#[test]
fn a_profile_whose_package_is_gone_is_refused_before_start() {
    let pkg = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let run = tempfile::tempdir().unwrap();
    let p = EmulatorProfile::at(root.path(), "rm-469");
    p.create(&package(pkg.path())).unwrap();
    assert!(p.check().is_ok());
    let gone = pkg.path().to_path_buf();
    drop(pkg);
    let e = p.check().unwrap_err().to_string();
    assert!(e.contains("emulator profile rm-469"), "{e}");
    assert!(
        e.contains(&gone.join("drives/z").display().to_string()),
        "{e}"
    );
    assert!(e.contains(&p.dir().display().to_string()), "{e}");
    let registry = DeviceRegistry::at(run.path().join("devices"));
    let start = EmulatorInstance::start(&Eka2l1::User("/nonexistent/eka2l1".into()), &p, &registry)
        .unwrap_err()
        .to_string();
    assert_eq!(start, e, "start must refuse before it runs anything");
}

#[test]
fn the_users_firmwares_are_the_folders_of_data_roms_by_name() {
    let user = tempfile::tempdir().unwrap();
    for f in ["rm-469", "rm-356"] {
        std::fs::create_dir_all(user.path().join("data/roms").join(f)).unwrap();
    }
    let data = EmulatorData::at(user.path());
    let names: Vec<String> = Firmware::in_user_data(&data)
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    assert_eq!(names, ["rm-356", "rm-469"]);
    assert!(Firmware::in_user_data(&EmulatorData::at(&user.path().join("none"))).is_empty());
}
