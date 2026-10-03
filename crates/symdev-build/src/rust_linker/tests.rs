use std::path::PathBuf;

use super::RustLinker;
use crate::{RustPrebuilt, SdkLldCache};

fn lld() -> RustLinker {
    RustLinker::Lld {
        rust_lld: None,
        cache: SdkLldCache::at(PathBuf::from("/home/u/.local/share/symdev/cache/sdk-lld")),
    }
}

fn prebuilt() -> (tempfile::TempDir, RustPrebuilt) {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("prebuilt/lib");
    std::fs::create_dir_all(&lib).unwrap();
    for a in RustPrebuilt::ARCHIVES {
        std::fs::write(lib.join(a), b"").unwrap();
    }
    let p = RustPrebuilt::in_sdk(tmp.path()).unwrap().unwrap();
    (tmp, p)
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
    let (_tmp, p) = prebuilt();
    assert!(!lld().needs_gcce(Some(&p)));
    assert_eq!(lld().prebuilt(Some(&p)), Some(&p));
    // A source checkout has no prebuilt set: rust-lld links shims GCCE compiled.
    assert!(lld().needs_gcce(None));
    // GNU ld keeps today's line, shims compiled per application, even beside the set.
    assert!(RustLinker::Gnu.needs_gcce(Some(&p)));
    assert_eq!(RustLinker::Gnu.prebuilt(Some(&p)), None);
}
