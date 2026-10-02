//! An archive cannot bring its own receipt: the names symdev writes its receipt under are
//! refused at the archive's root, and the package appears only with symdev's receipt.

use std::fs;
use std::path::PathBuf;

use super::World;
use crate::{ArchiveEntry, FileFetch, ReproducibleTarGz, SdkError};

/// `w`'s package plus `extra` (a file or, ending in `/`, a directory) at the root.
fn with_extra(w: &World, extra: &str) -> ArchiveEntry {
    let tree = w.tmp.path().join("forged");
    fs::create_dir_all(tree.join("bin")).unwrap();
    fs::write(tree.join("bin/arm-none-symbianelf-g++"), b"gxx").unwrap();
    match extra.strip_suffix('/') {
        Some(dir) => fs::create_dir_all(tree.join(dir).join("inner")).unwrap(),
        None => fs::write(
            tree.join(extra),
            "id = \"gcce;12.1.0\"\nsha256 = \"forged\"\nsource = \"evil\"\nurl = \"x\"\n",
        )
        .unwrap(),
    }
    let repo = PathBuf::from(w.source.base.trim_start_matches("file://"));
    let (sha256, size) =
        ReproducibleTarGz::pack(&tree, &["."], &repo.join("forged.tar.gz")).unwrap();
    ArchiveEntry {
        url: "forged.tar.gz".into(),
        sha256,
        size,
        ..w.entry.clone()
    }
}

fn refused(w: &World, entry: &ArchiveEntry, name: &str) {
    let home = w.home();
    match home.install(&w.id, &w.source, &FileFetch, entry, || {}) {
        Err(SdkError::UnsafeEntry { entry, .. }) => assert_eq!(entry, name),
        other => panic!("expected UnsafeEntry for {name}, got {other:?}"),
    }
    assert!(!home.package_dir(&w.id).exists());
    assert_eq!(home.installed(&w.id).unwrap(), None);
    let staging = home.root().join(".staging");
    assert_eq!(fs::read_dir(&staging).unwrap().count(), 0);
}

#[test]
fn an_archive_with_its_own_receipt_is_refused() {
    let w = World::new();
    let entry = with_extra(&w, ".symdev-package.toml");
    refused(&w, &entry, ".symdev-package.toml");
}

#[test]
fn an_archive_with_the_partial_receipt_name_is_refused() {
    let w = World::new();
    let entry = with_extra(&w, ".symdev-package.toml.partial/");
    refused(&w, &entry, ".symdev-package.toml.partial");
}

#[test]
fn the_installed_package_holds_only_symdevs_receipt() {
    let w = World::new();
    let home = w.home();
    home.install(&w.id, &w.source, &FileFetch, &w.entry, || {})
        .unwrap();
    let mut names: Vec<_> = fs::read_dir(home.package_dir(&w.id))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, [".symdev-package.toml", "bin"]);
}
