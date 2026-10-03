use std::fs;
use std::path::{Path, PathBuf};

use super::RustSdkLink;
use crate::rust_sdk::RustSdk;

/// A tree laid out as the `rust-sdk` package (and a checkout) is: `symbian-rs/` with its
/// target spec beside `crates/symdev-locale` and the root `Cargo.toml`, under `dir`.
fn sdk_tree(dir: &Path, sdk_dir: &str) -> RustSdk {
    for file in [
        &format!("{sdk_dir}/targets/arm-symbian-e32.json"),
        &format!("{sdk_dir}/rust-toolchain.toml"),
        "crates/symdev-locale/Cargo.toml",
        "Cargo.toml",
    ] {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "\n").unwrap();
    }
    let workspace = "[workspace]\nmembers = []\n";
    fs::write(dir.join(sdk_dir).join("Cargo.toml"), workspace).unwrap();
    RustSdk::at(&dir.join(sdk_dir)).unwrap()
}

fn project(tmp: &Path) -> PathBuf {
    let root = tmp.join("app");
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn the_project_names_the_sdk_through_the_link() {
    assert_eq!(
        RustSdkLink::crate_dir("symbian-std"),
        "build/rust-sdk/symbian-rs/crates/symbian-std"
    );
    assert_eq!(
        RustSdkLink::target_spec(),
        "build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json"
    );
}

#[test]
fn the_link_names_the_directory_above_symbian_rs() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("rust-sdk/0.2.0"), "symbian-rs");
    let root = project(tmp.path());
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    let target = fs::read_link(root.join("build/rust-sdk")).unwrap();
    assert_eq!(target, sdk.root().parent().unwrap());
    assert!(root.join(RustSdkLink::target_spec()).is_file());
    assert!(
        root.join("build/rust-sdk/crates/symdev-locale/Cargo.toml")
            .is_file()
    );
}

/// cargo cannot tell that `build/rust-sdk/…` now holds another tree: it compares mtimes,
/// and an older SDK's files are older than the last build's outputs (experiment 110 e).
#[test]
fn re_pointing_at_another_sdk_throws_away_cargo_s_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let old = sdk_tree(&tmp.path().join("rust-sdk/0.1.0"), "symbian-rs");
    let new = sdk_tree(&tmp.path().join("rust-sdk/0.2.0"), "symbian-rs");
    let root = project(tmp.path());
    let link = RustSdkLink::of(&root);
    link.point_at(&old).unwrap();
    fs::create_dir_all(root.join("build/cargo/arm-symbian-e32/release")).unwrap();
    link.point_at(&new).unwrap();
    assert_eq!(
        fs::read_link(root.join("build/rust-sdk")).unwrap(),
        new.root().parent().unwrap()
    );
    assert!(!root.join("build/cargo").exists());
}

#[test]
fn the_same_sdk_again_keeps_cargo_s_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("rust-sdk/0.2.0"), "symbian-rs");
    let root = project(tmp.path());
    let link = RustSdkLink::of(&root);
    link.point_at(&sdk).unwrap();
    fs::create_dir_all(root.join("build/cargo/arm-symbian-e32")).unwrap();
    link.point_at(&sdk).unwrap();
    assert!(root.join("build/cargo/arm-symbian-e32").is_dir());
}

/// The first link of a project that a symdev 0.1.0 built: its paths were absolute, so the
/// package paths cargo saw change anyway and it rebuilds without help.
#[test]
fn a_first_link_keeps_cargo_s_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("rust-sdk/0.2.0"), "symbian-rs");
    let root = project(tmp.path());
    fs::create_dir_all(root.join("build/cargo/arm-symbian-e32")).unwrap();
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    assert!(root.join("build/cargo/arm-symbian-e32").is_dir());
}

#[test]
fn an_sdk_directory_not_named_symbian_rs_is_refused() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("clone"), "my-sdk");
    let root = project(tmp.path());
    let err = RustSdkLink::of(&root)
        .point_at(&sdk)
        .unwrap_err()
        .to_string();
    assert!(err.contains(&sdk.root().display().to_string()), "{err}");
    assert!(err.contains("named symbian-rs"), "{err}");
    assert!(!root.join("build/rust-sdk").exists());
}

#[test]
fn a_directory_in_the_link_s_place_is_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("rust-sdk/0.2.0"), "symbian-rs");
    let root = project(tmp.path());
    fs::create_dir_all(root.join("build/rust-sdk/mine")).unwrap();
    let err = RustSdkLink::of(&root)
        .point_at(&sdk)
        .unwrap_err()
        .to_string();
    assert!(err.contains("build/rust-sdk"), "{err}");
    assert!(err.contains("not a link"), "{err}");
    assert!(root.join("build/rust-sdk/mine").is_dir());
}

/// The in-repo `symbian-rs/examples/*` built against their own checkout: they name the SDK
/// by relative paths, and a link to the tree above them would be a cycle that every
/// link-following tool walks again (review 0.2.0, minor 2).
#[test]
fn a_project_inside_the_sdk_tree_gets_no_link() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(tmp.path(), "symbian-rs");
    let root = tmp.path().join("symbian-rs/examples/hello");
    fs::create_dir_all(&root).unwrap();
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    assert!(fs::symlink_metadata(root.join("build/rust-sdk")).is_err());
}

/// A 0.2.0 development build linked such a project too; only symdev makes a link there.
#[test]
fn a_link_left_inside_the_sdk_tree_is_removed() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(tmp.path(), "symbian-rs");
    let root = tmp.path().join("symbian-rs/examples/hello");
    fs::create_dir_all(root.join("build")).unwrap();
    std::os::unix::fs::symlink(tmp.path(), root.join("build/rust-sdk")).unwrap();
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    assert!(fs::symlink_metadata(root.join("build/rust-sdk")).is_err());
    assert!(tmp.path().join("symbian-rs/rust-toolchain.toml").is_file());
}

#[test]
fn a_project_reached_through_a_link_is_still_inside_the_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("repo"), "symbian-rs");
    fs::create_dir_all(tmp.path().join("repo/symbian-rs/examples/hello")).unwrap();
    std::os::unix::fs::symlink(tmp.path().join("repo"), tmp.path().join("alias")).unwrap();
    let root = tmp.path().join("alias/symbian-rs/examples/hello");
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    assert!(fs::symlink_metadata(root.join("build/rust-sdk")).is_err());
}

/// Inside means below the tree, component by component: `sdk-app` is beside `sdk`.
#[test]
fn a_project_beside_the_tree_gets_its_link() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_tree(&tmp.path().join("sdk"), "symbian-rs");
    let root = tmp.path().join("sdk-app");
    fs::create_dir_all(&root).unwrap();
    RustSdkLink::of(&root).point_at(&sdk).unwrap();
    assert!(fs::read_link(root.join("build/rust-sdk")).is_ok());
}
