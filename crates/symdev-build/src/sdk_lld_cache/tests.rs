use std::fs;

use super::SdkLldCache;
use super::fixture::{archive, dso, object, sdk};

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|n| (*n).to_string()).collect()
}

#[test]
fn makes_fixed_copies_of_exactly_the_named_files() {
    let tmp = tempfile::tempdir().unwrap();
    let epocroot = sdk(&tmp.path().join("sdk"));
    let cache = SdkLldCache::at(tmp.path().join("home/cache/sdk-lld"));
    let wanted = names(&["eexe.lib", "euser.dso", "usrt2_2.lib"]);
    let copy = cache.ensure(&epocroot, &wanted).unwrap();
    assert!(
        copy.lib()
            .starts_with(tmp.path().join("home/cache/sdk-lld"))
    );
    assert_eq!(
        fs::read(copy.lib().join("euser.dso")).unwrap(),
        dso(b"\0\0")
    );
    assert!(
        !copy.lib().join("avkon.dso").exists(),
        "not named, not copied"
    );
    let usrt = fs::read(copy.urel().join("usrt2_2.lib")).unwrap();
    assert_eq!(usrt, archive(&object(2)), "R_ARM_TARGET2 -> R_ARM_ABS32");
    let eexe = fs::read(copy.urel().join("eexe.lib")).unwrap();
    assert_eq!(eexe, archive(&object(2)), "nothing to fix: copied as it is");
}

#[test]
fn a_second_build_reuses_the_copy_and_a_changed_sdk_file_gets_a_new_one() {
    let tmp = tempfile::tempdir().unwrap();
    let epocroot = sdk(&tmp.path().join("sdk"));
    let cache = SdkLldCache::at(tmp.path().join("cache"));
    let wanted = names(&["euser.dso"]);
    let first = cache.ensure(&epocroot, &wanted).unwrap();
    // Content-addressed: the copy is not made again (the marker survives).
    fs::write(first.lib().join("marker"), b"").unwrap();
    let again = cache.ensure(&epocroot, &wanted).unwrap();
    assert_eq!(again, first);
    assert!(again.lib().join("marker").exists());
    let euser = epocroot.join("epoc32/release/armv5/lib/euser.dso");
    fs::write(&euser, dso(b" ")).unwrap();
    let changed = cache.ensure(&epocroot, &wanted).unwrap();
    assert_ne!(changed, first);
    assert_eq!(
        fs::read(changed.lib().join("euser.dso")).unwrap(),
        dso(b"\0")
    );
    // Nothing half-made is left beside the copies.
    let left: Vec<_> = fs::read_dir(tmp.path().join("cache")).unwrap().collect();
    assert_eq!(left.len(), 2);
}

#[test]
fn the_same_files_in_another_sdk_get_their_own_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let cache = SdkLldCache::at(tmp.path().join("cache"));
    let wanted = names(&["euser.dso"]);
    let a = cache.ensure(&sdk(&tmp.path().join("a")), &wanted).unwrap();
    let b = cache.ensure(&sdk(&tmp.path().join("b")), &wanted).unwrap();
    assert_ne!(a, b);
}

#[test]
fn a_file_the_sdk_does_not_have_is_named_with_where_it_was_looked_for() {
    let tmp = tempfile::tempdir().unwrap();
    let epocroot = sdk(&tmp.path().join("sdk"));
    let cache = SdkLldCache::at(tmp.path().join("cache"));
    let e = cache
        .ensure(&epocroot, &names(&["nosuch.dso"]))
        .unwrap_err()
        .to_string();
    assert!(e.contains("nosuch.dso"), "{e}");
    assert!(e.contains("epoc32/release/armv5/lib"), "{e}");
}
