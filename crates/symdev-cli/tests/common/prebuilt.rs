//! `symdev` as a prebuilt binary runs: built on another machine, so the source checkout
//! it was built from is not here, and its Rust SDK comes from the `rust-sdk` package.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use assert_cmd::Command;
use symdev_build::RustSdk;

/// [`super::bin`], but a copy of the binary whose `RustSdk::CHECKOUT` names a directory
/// that does not exist. Only those bytes differ, and the path keeps its length.
pub fn bin() -> Command {
    super::isolated(Command::new(binary()))
}

/// The checkout path as the copy has it.
pub fn missing_checkout() -> String {
    let checkout = RustSdk::CHECKOUT.unwrap();
    let stem = checkout.strip_suffix("symbian-rs").unwrap();
    format!("{stem}symbian-xx")
}

fn binary() -> &'static Path {
    static COPY: OnceLock<PathBuf> = OnceLock::new();
    COPY.get_or_init(|| {
        let mut bytes = fs::read(assert_cmd::cargo::cargo_bin("symdev")).unwrap();
        let (from, to) = (RustSdk::CHECKOUT.unwrap().as_bytes(), missing_checkout());
        assert!(!Path::new(&to).exists(), "{to} exists");
        let mut replaced = 0;
        let mut at = 0;
        while let Some(found) = bytes[at..].windows(from.len()).position(|w| w == from) {
            let start = at + found;
            bytes[start..start + from.len()].copy_from_slice(to.as_bytes());
            replaced += 1;
            at = start + from.len();
        }
        assert!(
            replaced > 0,
            "{} is not in the symdev binary",
            RustSdk::CHECKOUT.unwrap()
        );
        let path = super::home().join("symdev-prebuilt");
        let part = path.with_extension("part");
        fs::write(&part, &bytes).unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&part, fs::Permissions::from_mode(0o755)).unwrap();
        fs::rename(&part, &path).unwrap();
        settle(&path);
        path
    })
}

/// Waits until `path` can be executed. A child that another test thread forked while
/// the copy was open for writing keeps that descriptor until it execs, and until then
/// the kernel refuses to run the file (`ETXTBSY`).
fn settle(path: &Path) {
    for _ in 0..100 {
        match std::process::Command::new(path).arg("--help").output() {
            Err(e) if e.kind() == ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("{}: {e}", path.display()),
            Ok(out) => {
                assert!(out.status.success(), "{}: {out:?}", path.display());
                return;
            }
        }
    }
    panic!("{} stayed busy", path.display());
}
