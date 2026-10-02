use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use flate2::Compression;
use flate2::write::GzEncoder;
use tar::{EntryType, Header};

use super::TarGz;
use crate::SdkError;

const URL: &str = "file:///srv/pkg/x.tar.gz";

/// One raw tar entry; the name and link are written into the header as bytes, so the
/// test can build archives the `tar` crate's safe setters refuse to write.
struct Raw<'a> {
    name: &'a str,
    kind: EntryType,
    link: &'a str,
    mode: u32,
    data: &'a [u8],
}

fn file<'a>(name: &'a str, data: &'a [u8]) -> Raw<'a> {
    Raw {
        name,
        kind: EntryType::Regular,
        link: "",
        mode: 0o644,
        data,
    }
}

fn link(name: &'static str, kind: EntryType, target: &'static str) -> Raw<'static> {
    Raw {
        name,
        kind,
        link: target,
        mode: 0o777,
        data: b"",
    }
}

/// `tmp/outer/into` (empty) and `tmp/outer/x.tar.gz` holding `entries`.
fn archive(entries: &[Raw]) -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let outer = tmp.path().join("outer");
    let into = outer.join("into");
    fs::create_dir_all(&into).unwrap();
    let path = outer.join("x.tar.gz");
    let gz = GzEncoder::new(fs::File::create(&path).unwrap(), Compression::default());
    let mut tar = tar::Builder::new(gz);
    for e in entries {
        let mut h = Header::new_gnu();
        h.as_old_mut().name[..e.name.len()].copy_from_slice(e.name.as_bytes());
        h.as_old_mut().linkname[..e.link.len()].copy_from_slice(e.link.as_bytes());
        h.set_entry_type(e.kind);
        h.set_mode(e.mode);
        h.set_size(e.data.len() as u64);
        h.set_cksum();
        tar.append(&h, e.data).unwrap();
    }
    tar.into_inner().unwrap().finish().unwrap();
    (tmp, path, into)
}

fn refused(entries: &[Raw], entry: &str, reason: &str) {
    let (tmp, path, into) = archive(entries);
    match TarGz::new(&path, URL).extract(&into) {
        Err(SdkError::UnsafeEntry {
            url,
            entry: e,
            reason: r,
        }) => {
            assert_eq!((url.as_str(), e.as_str(), r), (URL, entry, reason));
        }
        other => panic!("expected UnsafeEntry for {entry}, got {other:?}"),
    }
    let outer = tmp.path().join("outer");
    let mut left: Vec<_> = fs::read_dir(&outer)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    left.sort();
    assert_eq!(
        left,
        ["into", "x.tar.gz"],
        "something was written outside `into`"
    );
    assert!(!tmp.path().join("escape").exists());
}

#[test]
fn refuses_a_parent_dir_entry() {
    refused(
        &[file("../escape", b"x")],
        "../escape",
        "has a `..` component",
    );
}

#[test]
fn refuses_a_parent_dir_inside_the_path() {
    refused(
        &[file("a/../../escape", b"x")],
        "a/../../escape",
        "has a `..` component",
    );
}

#[test]
fn refuses_an_absolute_path() {
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("abs");
    let name = target.to_str().unwrap().to_string();
    refused(&[file(&name, b"x")], &name, "is an absolute path");
    assert!(!target.exists());
}

#[test]
fn refuses_a_symlink_that_leaves_the_package() {
    let e = [link("link", EntryType::Symlink, "../../etc")];
    refused(&e, "link", "links outside the package");
    refused(
        &[link("l", EntryType::Symlink, "/etc")],
        "l",
        "links outside the package",
    );
}

#[test]
fn refuses_a_symlink_that_leaves_through_another_symlink() {
    // `s` points at the package root, so `t = s/..` is its parent, although `a/b/c/s/..`
    // looks like `a/b/c` when read as text. Order must not matter either.
    let s = link("a/b/c/s", EntryType::Symlink, "../../..");
    let t = link("a/b/c/t", EntryType::Symlink, "s/..");
    refused(&[s, t], "a/b/c/t", "links outside the package");
    let s = link("a/b/c/s", EntryType::Symlink, "../../..");
    let t = link("a/b/c/t", EntryType::Symlink, "s/..");
    refused(&[t, s], "a/b/c/t", "links outside the package");
}

#[test]
fn never_writes_through_a_symlinked_directory() {
    let d = link("d", EntryType::Symlink, "../..");
    let x = link("d/x", EntryType::Symlink, "y");
    refused(&[d, x], "d/x", "is inside a symlink or a file");
}

#[test]
fn refuses_a_hard_link_that_leaves_the_package() {
    let e = [link("h", EntryType::Link, "../outside")];
    refused(&e, "h", "has a `..` component");
}

#[test]
fn refuses_devices_and_fifos() {
    for kind in [EntryType::Fifo, EntryType::Char, EntryType::Block] {
        let e = [Raw {
            name: "dev",
            kind,
            link: "",
            mode: 0o644,
            data: b"",
        }];
        refused(&e, "dev", "is a device or a fifo");
    }
}

#[test]
fn extracts_files_dirs_and_internal_links() {
    let (_tmp, path, into) = archive(&[
        Raw {
            name: "bin/",
            kind: EntryType::Directory,
            link: "",
            mode: 0o755,
            data: b"",
        },
        Raw {
            name: "bin/g++",
            kind: EntryType::Regular,
            link: "",
            mode: 0o755,
            data: b"gxx",
        },
        link("bin/c++", EntryType::Symlink, "g++"),
        link("bin/cc", EntryType::Link, "bin/g++"),
        link("lib/up", EntryType::Symlink, "../bin/g++"),
        file("./share/doc.txt", b"doc"),
    ]);
    TarGz::new(&path, URL).extract(&into).unwrap();
    assert_eq!(fs::read(into.join("bin/g++")).unwrap(), b"gxx");
    assert_eq!(
        fs::read_link(into.join("bin/c++")).unwrap(),
        Path::new("g++")
    );
    assert_eq!(fs::read(into.join("bin/c++")).unwrap(), b"gxx");
    assert_eq!(fs::read(into.join("bin/cc")).unwrap(), b"gxx");
    assert_eq!(fs::read(into.join("lib/up")).unwrap(), b"gxx");
    assert_eq!(fs::read(into.join("share/doc.txt")).unwrap(), b"doc");
}

#[test]
fn keeps_the_executable_bit() {
    let (_tmp, path, into) = archive(&[
        Raw {
            name: "run",
            kind: EntryType::Regular,
            link: "",
            mode: 0o700,
            data: b"#!",
        },
        file("data", b"d"),
    ]);
    TarGz::new(&path, URL).extract(&into).unwrap();
    let mode = |p: &str| fs::metadata(into.join(p)).unwrap().permissions().mode();
    assert_ne!(mode("run") & 0o100, 0);
    assert_eq!(mode("data") & 0o111, 0);
}

#[test]
fn refuses_a_directory_that_is_not_empty() {
    let (_tmp, path, into) = archive(&[file("a", b"a")]);
    fs::write(into.join("old"), b"old").unwrap();
    let e = TarGz::new(&path, URL)
        .extract(&into)
        .unwrap_err()
        .to_string();
    assert!(e.contains("empty"), "{e}");
}

#[test]
fn a_corrupt_archive_names_the_file() {
    let (_tmp, path, into) = archive(&[file("a", b"a")]);
    fs::write(&path, b"not gzip at all").unwrap();
    let e = TarGz::new(&path, URL)
        .extract(&into)
        .unwrap_err()
        .to_string();
    assert!(e.contains(path.to_str().unwrap()), "{e}");
}
