use std::path::PathBuf;

use super::RustLinker;
use crate::{RustPrebuilt, RustSdk, SdkLldCache};

fn lld() -> RustLinker {
    RustLinker::Lld {
        rust_lld: None,
        cache: SdkLldCache::at(PathBuf::from("/home/u/.local/share/symdev/cache/sdk-lld")),
    }
}

/// A Rust SDK with an empty workspace and no `prebuilt/`, as a source checkout is.
fn sdk_without_prebuilt() -> (tempfile::TempDir, RustSdk) {
    let (tmp, sdk) = sdk_with_prebuilt(&[]);
    std::fs::remove_dir_all(sdk.root().join("prebuilt")).unwrap();
    (tmp, sdk)
}

/// A Rust SDK with an empty workspace and `archives` in `prebuilt/lib`.
fn sdk_with_prebuilt(archives: &[&str]) -> (tempfile::TempDir, RustSdk) {
    let tmp = tempfile::tempdir().unwrap();
    for file in RustSdk::REQUIRED {
        let path = tmp.path().join("symbian-rs").join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }
    let sdk = tmp.path().join("symbian-rs");
    std::fs::write(sdk.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    std::fs::create_dir_all(sdk.join("prebuilt/lib")).unwrap();
    for a in archives {
        std::fs::write(sdk.join("prebuilt/lib").join(a), b"").unwrap();
    }
    let sdk = RustSdk::at(&sdk).unwrap();
    (tmp, sdk)
}

#[test]
fn rust_lld_is_the_default_and_gnu_is_asked_for_by_name() {
    assert!(!RustLinker::wants_gnu(None).unwrap());
    assert!(!RustLinker::wants_gnu(Some("")).unwrap());
    assert!(!RustLinker::wants_gnu(Some("lld")).unwrap());
    assert!(RustLinker::wants_gnu(Some("gnu")).unwrap());
    let e = RustLinker::wants_gnu(Some("GNU ld"))
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("SYMDEV_RUST_LINKER") && e.contains("`gnu`"),
        "{e}"
    );
}

#[test]
fn only_rust_lld_with_the_prebuilt_set_does_without_gcce() {
    let (_tmp, sdk) = sdk_with_prebuilt(&RustPrebuilt::ARCHIVES);
    assert!(!lld().needs_gcce(&sdk).unwrap());
    assert_eq!(lld().prebuilt(&sdk).unwrap(), sdk.prebuilt().unwrap());
    // A source checkout has no prebuilt set: rust-lld links shims GCCE compiled.
    let (_tmp2, checkout) = sdk_without_prebuilt();
    assert!(lld().needs_gcce(&checkout).unwrap());
    // GNU ld keeps today's line, shims compiled per application, even beside the set.
    assert!(RustLinker::Gnu.needs_gcce(&sdk).unwrap());
    assert_eq!(RustLinker::Gnu.prebuilt(&sdk).unwrap(), None);
}

#[test]
fn gnu_ld_does_not_look_at_a_damaged_prebuilt_set() {
    // The error for rust-lld names SYMDEV_RUST_LINKER=gnu as the way out, so that way
    // must not trip over the same directory.
    let (_tmp, sdk) = sdk_with_prebuilt(&["libsymrs.a"]);
    assert!(lld().needs_gcce(&sdk).is_err());
    assert!(RustLinker::Gnu.needs_gcce(&sdk).unwrap());
    assert_eq!(RustLinker::Gnu.prebuilt(&sdk).unwrap(), None);
}
