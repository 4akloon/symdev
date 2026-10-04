//! The way around the sources that a failed lookup names, per kind of package.

use super::repo::Repo;
use super::{id, keyless_private, manager};
use crate::Host;

#[test]
fn an_emulator_found_nowhere_names_symdev_eka2l1_and_sources_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("public")], false, &mut progress)
        .ensure(&[id("emulator;2026.10.04")])
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "emulator;2026.10.04 was not found in the sources searched: `public`; set \
         SYMDEV_EKA2L1 to your own EKA2L1 with --control and --data-dir, or add a source \
         that has it in /config/symdev/sources.toml"
    );
}

#[test]
fn a_firmware_behind_a_keyless_private_source_names_the_keys_and_symdev_eka2l1_data() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repo::new(tmp.path().join("repo"));
    repo.write_index();
    let sources = vec![repo.source("public"), keyless_private()];
    let mut progress = Vec::new();
    let e = manager(&tmp, sources, false, &mut progress)
        .ensure(&[id("firmware;rm-469;1")])
        .unwrap_err()
        .to_string();
    assert!(e.contains("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID"), "{e}");
    assert!(
        e.contains(
            "or set SYMDEV_EKA2L1_DATA to an EKA2L1 data folder that has this firmware installed"
        ),
        "{e}"
    );
    assert_eq!(e.matches("SYMDEV_EKA2L1_DATA").count(), 1, "{e}");
}

#[test]
fn a_gcce_found_nowhere_names_no_variable() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = Repo::new(tmp.path().join("repo"));
    repo.write_index();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("public")], false, &mut progress)
        .ensure(&[id("gcce;99.0")])
        .unwrap_err()
        .to_string();
    assert!(!e.contains("SYMDEV_"), "{e}");
}
