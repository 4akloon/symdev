//! Uninstalling, listing and reading receipts.

use std::fs;

use super::World;
use crate::{FileFetch, PackageId};

#[test]
fn uninstall_removes_the_package_once() {
    let w = World::new();
    let home = w.home();
    home.install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    assert!(home.uninstall(&w.id).unwrap());
    assert!(!home.package_dir(&w.id).exists());
    assert_eq!(home.installed(&w.id).unwrap(), None);
    assert!(!home.uninstall(&w.id).unwrap());
}

#[test]
fn list_returns_the_receipts_sorted_by_id() {
    let w = World::new();
    let home = w.home();
    assert!(home.list().unwrap().is_empty());
    let later = PackageId::parse("sdk;s60-3rd-fp2;1.1").unwrap();
    let earlier = PackageId::parse("emulator;1").unwrap();
    for id in [&later, &w.id, &earlier] {
        home.install(id, &w.source, &FileFetch, &w.entry, || {})
            .unwrap();
    }
    fs::create_dir_all(home.root().join("gcce/9.9.9/bin")).unwrap();
    let ids: Vec<_> = home.list().unwrap().into_iter().map(|r| r.id).collect();
    assert_eq!(ids, [earlier, w.id.clone(), later]);
}

#[test]
fn a_receipt_for_another_id_names_the_quoted_repair_commands() {
    let w = World::new();
    let home = w.home();
    home.install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let other = PackageId::parse("gcce;12.2.0").unwrap();
    fs::rename(home.package_dir(&w.id), home.package_dir(&other)).unwrap();
    let e = home.installed(&other).unwrap_err().to_string();
    assert!(
        e.contains("symdev sdk uninstall 'gcce;12.2.0' && symdev sdk install 'gcce;12.2.0'"),
        "{e}"
    );
}
