//! A source with a `key` installs only from an index its key signed; `file://` sources
//! of other tests stay unsigned because they set no key.

use std::fs;

use super::repo::Repo;
use super::{id, manager};
use crate::{Host, IndexSigningKey};

/// The base64 of 32 bytes 0x07 and 0x08: throwaway seeds.
const SEED_7: &str = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
const SEED_8: &str = "CAgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAg=";

fn key(seed: &str) -> IndexSigningKey {
    IndexSigningKey::from_base64(seed).unwrap()
}

#[test]
fn a_keyed_source_installs_from_an_index_its_key_signed() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.sign_with(key(SEED_7))
        .add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let source = repo.source("local").with_key(key(SEED_7).trusted());
    let mut progress = Vec::new();
    let receipts = manager(&tmp, vec![source], false, &mut progress)
        .ensure(&[id("gcce;12.1.0")])
        .unwrap();
    assert_eq!(receipts[0].source, "local");
}

/// The index lists a package whose archive is not the one signed: an attacker who can
/// write the bucket swapped the hash. The source is unreadable, and the error says why.
#[test]
fn a_tampered_index_makes_its_source_unreadable_with_the_url_named() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.sign_with(key(SEED_7))
        .add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let signed = fs::read_to_string(repo.index_path()).unwrap();
    let sha = signed.split("sha256 = \"").nth(1).unwrap()[..64].to_string();
    fs::write(repo.index_path(), signed.replace(&sha, &"f".repeat(64))).unwrap();
    let source = repo.source("local").with_key(key(SEED_7).trusted());
    let url = source.index_url();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![source], false, &mut progress)
        .ensure(&[id("gcce;12.1.0")])
        .unwrap_err()
        .to_string();
    assert!(
        e.contains(&format!("source `local` could not be read: {url}: ")),
        "{e}"
    );
    assert!(e.contains("tampered"), "{e}");
}

#[test]
fn an_index_signed_by_another_key_is_not_used_and_a_later_source_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let mut forged = Repo::new(tmp.path().join("forged"));
    forged
        .sign_with(key(SEED_8))
        .add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut mirror = Repo::new(tmp.path().join("mirror"));
    mirror.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let sources = vec![
        forged.source("public").with_key(key(SEED_7).trusted()),
        mirror.source("mirror"),
    ];
    let mut progress = Vec::new();
    let receipts = manager(&tmp, sources, false, &mut progress)
        .ensure(&[id("gcce;12.1.0")])
        .unwrap();
    assert_eq!(receipts[0].source, "mirror");
}

#[test]
fn an_unsigned_index_of_a_keyed_source_is_listed_as_a_warning() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let source = repo.source("local").with_key(key(SEED_7).trusted());
    let mut progress = Vec::new();
    let available = manager(&tmp, vec![source], false, &mut progress)
        .available()
        .unwrap();
    assert!(available.is_empty());
    let text = String::from_utf8(progress).unwrap();
    assert!(
        text.starts_with("warning: source `local` could not be read: ")
            && text.contains("unsigned"),
        "{text}"
    );
}
