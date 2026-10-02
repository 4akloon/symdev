//! Which source an id comes from, and what an error says when none can provide it.

use std::collections::BTreeMap;

use super::repo::Repo;
use super::{home, id, keyless_private, manager};
use crate::{Host, S3Keys, SdkManager, Sources};

#[test]
fn a_keyless_private_source_is_skipped_when_a_later_source_has_the_id() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("sdk;s60-3rd-fp2;1.1", Host::Any, &[]);
    let sources = vec![keyless_private(), repo.source("mirror")];
    let mut progress = Vec::new();
    let receipts = manager(&tmp, sources, false, &mut progress)
        .ensure(&[id("sdk;s60-3rd-fp2;1.1")])
        .unwrap();
    assert_eq!(receipts[0].source, "mirror");
}

#[test]
fn an_unreadable_source_is_skipped_when_a_later_source_has_the_id() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let gone = Repo::new(tmp.path().join("gone")).source("gone");
    let mut progress = Vec::new();
    let receipts = manager(
        &tmp,
        vec![gone, repo.source("mirror")],
        false,
        &mut progress,
    )
    .ensure(&[id("gcce;12.1.0")])
    .unwrap();
    assert_eq!(receipts[0].source, "mirror");
}

#[test]
fn an_sdk_found_nowhere_with_a_keyless_source_names_the_keys_and_the_epocroot() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let sources = vec![repo.source("public"), keyless_private()];
    let mut progress = Vec::new();
    let e = manager(&tmp, sources, false, &mut progress)
        .ensure(&[id("sdk;s60-3rd-fp2;1.1")])
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("sdk;s60-3rd-fp2;1.1 was not found"), "{e}");
    assert!(e.contains("`public`"), "{e}");
    assert!(e.contains("source `private`"), "{e}");
    assert!(e.contains("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID"), "{e}");
    assert!(e.contains("SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY"), "{e}");
    assert!(e.contains("or set SYMDEV_EPOCROOT to your own SDK"), "{e}");
}

#[test]
fn an_id_found_nowhere_names_the_sources_searched_and_why_one_was_unreadable() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let gone = Repo::new(tmp.path().join("gone")).source("gone");
    let mut progress = Vec::new();
    let e = manager(
        &tmp,
        vec![repo.source("public"), gone],
        false,
        &mut progress,
    )
    .ensure(&[id("gcce;99.0")])
    .unwrap_err()
    .to_string();
    assert!(e.starts_with("gcce;99.0 was not found"), "{e}");
    assert!(e.contains("`public`"), "{e}");
    assert!(e.contains("source `gone` could not be read"), "{e}");
    assert!(e.contains("index.toml"), "{e}");
    assert!(!e.contains("SYMDEV_EPOCROOT"), "{e}");
}

#[test]
fn no_configured_source_says_where_to_add_one() {
    let tmp = tempfile::tempdir().unwrap();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![], false, &mut progress)
        .ensure(&[id("gcce;12.1.0")])
        .unwrap_err()
        .to_string();
    assert!(e.contains("sources.toml"), "{e}");
}

#[test]
fn an_id_listed_only_for_another_host_says_so() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut progress = Vec::new();
    let mut m = manager(&tmp, vec![repo.source("local")], false, &mut progress);
    // The only host other than this one: an archive for x86_64-linux does not serve it.
    m.host = Host::Any;
    let e = m.ensure(&[id("gcce;12.1.0")]).unwrap_err().to_string();
    assert_eq!(e, "gcce;12.1.0 has no archive for any in source `local`");
}

#[test]
fn keys_make_an_s3_source_searchable() {
    let tmp = tempfile::tempdir().unwrap();
    let keys = BTreeMap::from([(
        "private".to_string(),
        S3Keys {
            access_key_id: "AKID".into(),
            secret_access_key: "secret".into(),
        },
    )]);
    let sources = Sources {
        list: vec![keyless_private()],
    };
    let mut progress = Vec::new();
    let mut m = SdkManager::new(home(&tmp), sources, keys, false, &mut progress).unwrap();
    // With keys the source is asked (and, being unreachable, could not be read): the
    // keys hint is gone.
    let e = m
        .ensure(&[id("sdk;s60-3rd-fp2;1.1")])
        .unwrap_err()
        .to_string();
    assert!(e.contains("source `private` could not be read"), "{e}");
    assert!(!e.contains("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID"), "{e}");
}

#[test]
fn available_lists_what_install_takes_when_the_first_source_lacks_this_host() {
    let tmp = tempfile::tempdir().unwrap();
    let mut first = Repo::new(tmp.path().join("first"));
    first.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut second = Repo::new(tmp.path().join("second"));
    second.add("gcce;12.1.0", Host::Any, &[]);
    let sources = vec![first.source("one"), second.source("two")];
    let mut progress = Vec::new();
    let mut m = manager(&tmp, sources, false, &mut progress);
    // The only host other than this one: `one` lists the id without an archive for it.
    m.host = Host::Any;
    let e = m.ensure(&[id("gcce;12.1.0")]).unwrap_err().to_string();
    assert_eq!(e, "gcce;12.1.0 has no archive for any in source `one`");
    let listed: Vec<_> = m.available().unwrap().into_iter().map(|(s, _)| s).collect();
    assert!(listed.is_empty(), "the first source decides: {listed:?}");
}
