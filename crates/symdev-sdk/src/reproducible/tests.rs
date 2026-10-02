use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use super::ReproducibleTarGz;
use crate::TarGz;

fn write(root: &Path, rel: &str, data: &[u8], mode: u32) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, data).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
}

/// An SDK-like tree: two included directories, one left out, an internal symlink.
fn tree(root: &Path) {
    write(root, "epoc32/include/e32std.h", b"// e32std", 0o644);
    write(root, "epoc32/include/a-b/x.h", b"// x", 0o600);
    write(root, "epoc32/include/a/b.h", b"// b", 0o664);
    write(
        root,
        "epoc32/release/armv5/urel/eexe.lib",
        b"!<arch>",
        0o644,
    );
    write(root, "epoc32/release/armv5/urel/tool", b"#!/bin/sh", 0o700);
    write(root, "epoc32/tools/left-out.pl", b"perl", 0o755);
    symlink("e32std.h", root.join("epoc32/include/std.h")).unwrap();
}

const INCLUDE: &[&str] = &["epoc32/include", "epoc32/release/armv5/urel"];

fn touch_all(dir: &Path, when: SystemTime) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_symlink() {
            continue;
        }
        if path.is_dir() {
            touch_all(&path, when);
        } else {
            fs::File::options()
                .write(true)
                .open(&path)
                .unwrap()
                .set_modified(when)
                .unwrap();
        }
    }
}

#[test]
fn packing_twice_gives_the_same_bytes_whatever_the_mtimes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("sdk");
    tree(&root);
    let (sha_a, size_a) =
        ReproducibleTarGz::pack(&root, INCLUDE, &tmp.path().join("a.tar.gz")).unwrap();
    touch_all(
        &root,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000),
    );
    fs::set_permissions(
        root.join("epoc32/include/a-b/x.h"),
        fs::Permissions::from_mode(0o644),
    )
    .unwrap();
    let (sha_b, size_b) =
        ReproducibleTarGz::pack(&root, INCLUDE, &tmp.path().join("b.tar.gz")).unwrap();
    assert_eq!((sha_a.as_str(), size_a), (sha_b.as_str(), size_b));
    let bytes = fs::read(tmp.path().join("a.tar.gz")).unwrap();
    assert_eq!(sha_a, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(size_a, bytes.len() as u64);
}

#[test]
fn the_gzip_header_has_no_mtime_and_no_name() {
    let tmp = tempfile::tempdir().unwrap();
    tree(&tmp.path().join("sdk"));
    let out = tmp.path().join("a.tar.gz");
    ReproducibleTarGz::pack(&tmp.path().join("sdk"), INCLUDE, &out).unwrap();
    let bytes = fs::read(out).unwrap();
    assert_eq!(&bytes[..3], [0x1f, 0x8b, 8]);
    assert_eq!(bytes[3], 0, "no FNAME or other flags");
    assert_eq!(&bytes[4..8], [0, 0, 0, 0], "mtime 0");
}

#[test]
fn entries_are_sorted_bytewise_with_normalised_headers() {
    let tmp = tempfile::tempdir().unwrap();
    tree(&tmp.path().join("sdk"));
    let out = tmp.path().join("a.tar.gz");
    ReproducibleTarGz::pack(&tmp.path().join("sdk"), INCLUDE, &out).unwrap();
    let gz = flate2::read::GzDecoder::new(fs::File::open(&out).unwrap());
    let mut archive = tar::Archive::new(gz);
    let mut seen = Vec::new();
    for entry in archive.entries().unwrap() {
        let entry = entry.unwrap();
        let h = entry.header();
        assert_eq!(h.mtime().unwrap(), 0);
        assert_eq!((h.uid().unwrap(), h.gid().unwrap()), (0, 0));
        assert_eq!(h.username().unwrap(), Some(""));
        assert_eq!(h.groupname().unwrap(), Some(""));
        let path = entry
            .path()
            .unwrap()
            .to_str()
            .unwrap()
            .trim_end_matches('/')
            .to_string();
        seen.push((path, h.mode().unwrap()));
    }
    let names: Vec<&str> = seen.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        names,
        [
            "epoc32",
            "epoc32/include",
            "epoc32/include/a",
            "epoc32/include/a-b",
            "epoc32/include/a-b/x.h",
            "epoc32/include/a/b.h",
            "epoc32/include/e32std.h",
            "epoc32/include/std.h",
            "epoc32/release",
            "epoc32/release/armv5",
            "epoc32/release/armv5/urel",
            "epoc32/release/armv5/urel/eexe.lib",
            "epoc32/release/armv5/urel/tool",
        ]
    );
    let mode = |name: &str| seen.iter().find(|(p, _)| p == name).unwrap().1;
    assert_eq!(mode("epoc32/include"), 0o755);
    assert_eq!(mode("epoc32/include/a-b/x.h"), 0o644);
    assert_eq!(mode("epoc32/include/a/b.h"), 0o644);
    assert_eq!(mode("epoc32/release/armv5/urel/tool"), 0o755);
}

#[test]
fn pack_then_extract_round_trips_bytes_modes_and_links() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("sdk");
    tree(&root);
    let out = tmp.path().join("a.tar.gz");
    ReproducibleTarGz::pack(&root, INCLUDE, &out).unwrap();
    let into = tmp.path().join("into");
    fs::create_dir(&into).unwrap();
    TarGz::new(&out, "file:///a.tar.gz").extract(&into).unwrap();
    assert_eq!(
        fs::read(into.join("epoc32/include/a/b.h")).unwrap(),
        b"// b"
    );
    assert_eq!(
        fs::read_link(into.join("epoc32/include/std.h")).unwrap(),
        Path::new("e32std.h")
    );
    let tool = fs::metadata(into.join("epoc32/release/armv5/urel/tool")).unwrap();
    assert_ne!(tool.permissions().mode() & 0o100, 0);
    assert!(!into.join("epoc32/tools").exists());
}

#[test]
fn dot_packs_the_whole_root() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("prefix");
    write(&root, "bin/arm-none-symbianelf-g++", b"elf", 0o755);
    write(&root, "lib/libx.a", b"ar", 0o644);
    let out = tmp.path().join("a.tar.gz");
    ReproducibleTarGz::pack(&root, &["."], &out).unwrap();
    let into = tmp.path().join("into");
    fs::create_dir(&into).unwrap();
    TarGz::new(&out, "file:///a.tar.gz").extract(&into).unwrap();
    assert_eq!(
        fs::read(into.join("bin/arm-none-symbianelf-g++")).unwrap(),
        b"elf"
    );
    assert_eq!(fs::read(into.join("lib/libx.a")).unwrap(), b"ar");
}

#[test]
fn a_missing_include_directory_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    tree(&tmp.path().join("sdk"));
    let e = ReproducibleTarGz::pack(
        &tmp.path().join("sdk"),
        &["epoc32/nope"],
        &tmp.path().join("a"),
    )
    .unwrap_err()
    .to_string();
    assert!(e.contains("epoc32/nope"), "{e}");
}

#[test]
fn an_include_that_leaves_the_root_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    tree(&tmp.path().join("sdk"));
    for include in ["../sdk", "/etc"] {
        let e = ReproducibleTarGz::pack(&tmp.path().join("sdk"), &[include], &tmp.path().join("a"))
            .unwrap_err()
            .to_string();
        assert!(e.contains(include), "{e}");
    }
}

#[test]
fn a_symlink_leaving_the_root_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("sdk");
    tree(&root);
    symlink("../../../etc/passwd", root.join("epoc32/include/bad.h")).unwrap();
    let e = ReproducibleTarGz::pack(&root, INCLUDE, &tmp.path().join("a"))
        .unwrap_err()
        .to_string();
    assert!(e.contains("epoc32/include/bad.h"), "{e}");
    assert!(e.contains("../../../etc/passwd"), "{e}");
}
