//! A package never lands inside or above another, and `uninstall` removes only a
//! package: never a directory a partial id or an id inside a package names.

use std::fs;

use super::World;
use crate::{FileFetch, PackageId, SdkError};

fn id(s: &str) -> PackageId {
    PackageId::parse(s).unwrap()
}

fn message(e: SdkError) -> String {
    e.to_string()
}

#[test]
fn uninstalling_a_partial_id_removes_nothing_and_names_the_package() {
    let w = World::new();
    let home = w.home();
    let sdk = id("sdk;s60-3rd-fp2;1.1");
    home.install(&sdk, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let e = message(home.uninstall(&id("sdk;s60-3rd-fp2")).unwrap_err());
    assert!(e.contains("sdk;s60-3rd-fp2;1.1"), "{e}");
    assert!(
        e.contains("symdev sdk uninstall 'sdk;s60-3rd-fp2;1.1'"),
        "{e}"
    );
    assert!(home.installed(&sdk).unwrap().is_some());
}

#[test]
fn uninstalling_an_id_inside_a_package_removes_nothing_and_names_the_package() {
    let w = World::new();
    let home = w.home();
    home.install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let e = message(home.uninstall(&id("gcce;12.1.0;bin")).unwrap_err());
    assert!(
        e.contains("inside the installed package gcce;12.1.0"),
        "{e}"
    );
    let gxx = home.package_dir(&w.id).join("bin/arm-none-symbianelf-g++");
    assert!(gxx.is_file());
}

#[test]
fn installing_inside_a_package_is_refused_naming_both_ids() {
    let w = World::new();
    let home = w.home();
    home.install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let inner = id("gcce;12.1.0;bin");
    let e = message(
        home.install(&inner, &w.source, &FileFetch, &w.entry, || {})
            .unwrap_err(),
    );
    assert!(
        e.contains("gcce;12.1.0;bin") && e.contains("package gcce;12.1.0"),
        "{e}"
    );
    let gxx = home.package_dir(&w.id).join("bin/arm-none-symbianelf-g++");
    assert_eq!(fs::read(gxx).unwrap(), b"gxx");
    assert!(home.installed(&w.id).unwrap().is_some());
}

#[test]
fn installing_above_a_package_is_refused_naming_both_ids() {
    let w = World::new();
    let home = w.home();
    let sdk = id("sdk;s60-3rd-fp2;1.1");
    home.install(&sdk, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let mut starts = 0;
    let e = message(
        home.install(
            &id("sdk;s60-3rd-fp2"),
            &w.source,
            &FileFetch,
            &w.entry,
            || starts += 1,
        )
        .unwrap_err(),
    );
    assert!(
        e.contains("install sdk;s60-3rd-fp2:") && e.contains("package sdk;s60-3rd-fp2;1.1"),
        "{e}"
    );
    assert_eq!(
        starts, 0,
        "refused before anything is announced or downloaded"
    );
    assert!(home.installed(&sdk).unwrap().is_some());
}

#[test]
fn uninstall_finishes_an_unfinished_package() {
    let w = World::new();
    let home = w.home();
    let dir = home.package_dir(&w.id);
    fs::create_dir_all(dir.join("bin")).unwrap();
    assert!(home.uninstall(&w.id).unwrap());
    assert!(!dir.exists());
    assert!(!home.root().join("gcce").exists());
}
