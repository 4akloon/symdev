use std::collections::BTreeMap;
use std::fs;

use super::SdkManager;
use crate::{Auth, Host, PackageId, SdkHome, SourceSpec, Sources};

mod lookup;
mod repo;
use repo::Repo;

fn id(s: &str) -> PackageId {
    PackageId::parse(s).unwrap()
}

fn home(tmp: &tempfile::TempDir) -> SdkHome {
    SdkHome::new(tmp.path().join("home"), tmp.path().join("cache"))
}

fn manager<'w>(
    tmp: &tempfile::TempDir,
    sources: Vec<SourceSpec>,
    offline: bool,
    progress: &'w mut Vec<u8>,
) -> SdkManager<'w> {
    let sources = Sources { list: sources };
    SdkManager::new(home(tmp), sources, BTreeMap::new(), offline, progress).unwrap()
}

/// An `s3` source whose keys are not set, so it is never contacted; with keys it is a
/// local port nothing listens on, so no test reaches the network.
fn keyless_private() -> SourceSpec {
    SourceSpec::new("private", "https://127.0.0.1:1/bucket/", Auth::S3).unwrap()
}

#[test]
fn installs_missing_packages_with_one_progress_line_each() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[])
        .add("sdk;s60-3rd-fp2;1.1", Host::Any, &[]);
    let mut progress = Vec::new();
    let receipts = manager(&tmp, vec![repo.source("local")], false, &mut progress)
        .ensure(&[id("gcce;12.1.0"), id("sdk;s60-3rd-fp2;1.1")])
        .unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts.iter().all(|r| r.source == "local"));
    let text = String::from_utf8(progress).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines[0].starts_with("installing gcce;12.1.0 ("), "{text}");
    assert!(lines[0].ends_with(" MB) from local…"), "{text}");
    assert!(
        lines[1].starts_with("installing sdk;s60-3rd-fp2;1.1 ("),
        "{text}"
    );
    assert!(Repo::marker(&home(&tmp), "gcce;12.1.0").starts_with("gcce;12.1.0"));
}

#[test]
fn installed_packages_need_no_index() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let ids = [id("gcce;12.1.0")];
    let mut progress = Vec::new();
    manager(&tmp, vec![repo.source("local")], false, &mut progress)
        .ensure(&ids)
        .unwrap();
    fs::remove_file(repo.index_path()).unwrap();
    let mut again = Vec::new();
    let receipts = manager(&tmp, vec![repo.source("local")], false, &mut again)
        .ensure(&ids)
        .unwrap();
    assert_eq!(receipts[0].id, ids[0]);
    assert!(again.is_empty(), "nothing installed, nothing printed");
}

#[test]
fn offline_names_the_install_command() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("local")], true, &mut progress)
        .ensure(&[id("gcce;12.1.0")])
        .unwrap_err()
        .to_string();
    assert_eq!(
        e,
        "gcce;12.1.0 is not installed and --offline forbids downloading it; run \
         `symdev sdk install 'gcce;12.1.0'`"
    );
    assert!(!home(&tmp).package_dir(&id("gcce;12.1.0")).exists());
}

#[test]
fn offline_names_every_missing_package_in_one_command() {
    let tmp = tempfile::tempdir().unwrap();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![], true, &mut progress)
        .ensure(&[id("gcce;12.1.0"), id("sdk;s60-3rd-fp2;1.1")])
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("run `symdev sdk install 'gcce;12.1.0' 'sdk;s60-3rd-fp2;1.1'`"),
        "{e}"
    );
}

#[test]
fn dependencies_are_installed_first() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("firmware;rm-469;1", Host::Any, &[]).add(
        "emulator;1",
        Host::Any,
        &["firmware;rm-469;1"],
    );
    let mut progress = Vec::new();
    manager(&tmp, vec![repo.source("local")], false, &mut progress)
        .ensure(&[id("emulator;1")])
        .unwrap();
    let text = String::from_utf8(progress).unwrap();
    let order: Vec<_> = text.lines().map(|l| l.split(' ').nth(1).unwrap()).collect();
    assert_eq!(order, ["firmware;rm-469;1", "emulator;1"], "{text}");
    assert!(
        home(&tmp)
            .installed(&id("firmware;rm-469;1"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_dependency_cycle_is_an_error_naming_it() {
    let tmp = tempfile::tempdir().unwrap();
    let mut repo = Repo::new(tmp.path().join("repo"));
    repo.add("a;1", Host::Any, &["b;1"])
        .add("b;1", Host::Any, &["a;1"]);
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![repo.source("local")], false, &mut progress)
        .ensure(&[id("a;1")])
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("a;1") && e.contains("b;1") && e.contains("cycle"),
        "{e}"
    );
}

#[test]
fn available_lists_each_id_once_from_the_first_source() {
    let tmp = tempfile::tempdir().unwrap();
    let mut first = Repo::new(tmp.path().join("first"));
    first.add("gcce;12.1.0", Host::X86_64Linux, &[]);
    let mut second = Repo::new(tmp.path().join("second"));
    second
        .add("gcce;12.1.0", Host::X86_64Linux, &[])
        .add("sdk;s60-3rd-fp2;1.1", Host::Any, &[]);
    let sources = vec![first.source("one"), keyless_private(), second.source("two")];
    let mut progress = Vec::new();
    let listed: Vec<_> = manager(&tmp, sources, false, &mut progress)
        .available()
        .unwrap()
        .into_iter()
        .map(|(source, p)| (source, p.id.to_string()))
        .collect();
    assert_eq!(
        listed,
        [
            ("one".to_string(), "gcce;12.1.0".to_string()),
            ("two".to_string(), "sdk;s60-3rd-fp2;1.1".to_string())
        ]
    );
    let text = String::from_utf8(progress).unwrap();
    assert!(
        text.contains("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID"),
        "{text}"
    );
}

#[test]
fn available_reads_no_index_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let mut progress = Vec::new();
    let e = manager(&tmp, vec![], true, &mut progress)
        .available()
        .unwrap_err()
        .to_string();
    assert!(e.contains("--offline"), "{e}");
}
