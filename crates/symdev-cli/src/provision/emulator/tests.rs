use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

use symdev_emulator::device::Eka2l1;
use symdev_sdk::Pins;

use crate::provision::Provision;

fn provision(offline: bool, vars: &[(&str, &Path)]) -> Provision {
    let vars: BTreeMap<String, OsString> = vars
        .iter()
        .map(|(k, v)| (k.to_string(), v.as_os_str().to_owned()))
        .collect();
    Provision::from_lookup(offline, None, move |key| vars.get(key).cloned())
}

/// No HOME and no SYMDEV_HOME: any look at the packages would be an error.
#[test]
fn symdev_eka2l1_is_started_as_it_is_and_no_package_is_looked_at() {
    let p = provision(false, &[("SYMDEV_EKA2L1", Path::new("/u/eka2l1"))]);
    assert_eq!(p.eka2l1().unwrap(), Eka2l1::User("/u/eka2l1".into()));
}

#[test]
fn symdev_eka2l1_data_gives_its_firmwares_and_no_package_is_looked_at() {
    let user = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(user.path().join("data/roms/rm-469")).unwrap();
    let p = provision(false, &[("SYMDEV_EKA2L1_DATA", user.path())]);
    let names: Vec<String> = p
        .firmwares()
        .unwrap()
        .iter()
        .map(|f| f.name().to_string())
        .collect();
    assert_eq!(names, ["rm-469"]);
}

#[test]
fn symdev_eka2l1_data_without_a_firmware_names_the_package_instead() {
    let user = tempfile::tempdir().unwrap();
    let p = provision(false, &[("SYMDEV_EKA2L1_DATA", user.path())]);
    let e = p.firmwares().unwrap_err().to_string();
    assert!(e.contains("no firmware in data/roms/"), "{e}");
    assert!(e.contains("firmware;rm-469;1"), "{e}");
}

#[test]
fn without_the_variables_offline_names_the_install_commands() {
    let home = tempfile::tempdir().unwrap();
    let p = provision(true, &[("HOME", home.path())]);
    let e = p.eka2l1().unwrap_err().to_string();
    let install = format!("symdev sdk install {}", Pins::emulator().shell_word());
    assert!(e.contains(&install), "{e}");
    let e = p.firmwares().unwrap_err().to_string();
    assert!(e.contains("symdev sdk install 'firmware;rm-469;1'"), "{e}");
}

#[test]
fn a_relative_symdev_eka2l1_data_is_refused_as_symdev_home_is() {
    let p = provision(
        false,
        &[("SYMDEV_EKA2L1_DATA", Path::new("relative/eka2l1"))],
    );
    let e = p.firmwares().unwrap_err().to_string();
    assert!(
        e.contains("SYMDEV_EKA2L1_DATA") && e.contains("absolute"),
        "{e}"
    );
}
