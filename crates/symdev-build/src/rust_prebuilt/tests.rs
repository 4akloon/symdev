use std::fs;
use std::path::Path;

use super::RustPrebuilt;

fn sdk_with(root: &Path, archives: &[&str]) {
    let lib = root.join("prebuilt/lib");
    fs::create_dir_all(&lib).unwrap();
    for a in archives {
        fs::write(lib.join(a), b"!<arch>\n").unwrap();
    }
}

#[test]
fn a_checkout_without_prebuilt_has_none() {
    let tmp = tempfile::tempdir().unwrap();
    assert_eq!(RustPrebuilt::in_sdk(tmp.path()).unwrap(), None);
}

#[test]
fn a_package_with_the_four_archives_names_the_shims_avkon_first() {
    let tmp = tempfile::tempdir().unwrap();
    sdk_with(tmp.path(), &RustPrebuilt::ARCHIVES);
    let p = RustPrebuilt::in_sdk(tmp.path()).unwrap().unwrap();
    let lib = tmp.path().join("prebuilt/lib");
    assert_eq!(p.lib_dir(), lib);
    assert_eq!(p.shims(false), vec![lib.join("libsymrs.a")]);
    // The Avkon shim refers to the common one, so it is searched first.
    assert_eq!(
        p.shims(true),
        vec![lib.join("libsymrs_ui.a"), lib.join("libsymrs.a")]
    );
}

#[test]
fn a_prebuilt_directory_missing_an_archive_is_refused_with_the_way_out() {
    let tmp = tempfile::tempdir().unwrap();
    sdk_with(tmp.path(), &["libsymrs.a", "libsymrs_ui.a", "libsupc++.a"]);
    let e = RustPrebuilt::in_sdk(tmp.path()).unwrap_err().to_string();
    assert!(e.contains("prebuilt/lib/libgcc.a"), "{e}");
    assert!(
        e.contains("reinstall") && e.contains("SYMDEV_RUST_LINKER=gnu"),
        "{e}"
    );
}
