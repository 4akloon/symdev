//! What a Rust build checks and sets up before cargo runs (experiment 110).
use std::fs;
use std::path::Path;

use super::rust_build::rust;

fn project_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for (file, text) in files {
        let path = tmp.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    tmp
}

fn sdk_toolchain() -> String {
    fs::read_to_string(rust().sdk.root().join("rust-toolchain.toml")).unwrap()
}

#[test]
fn prepare_links_build_rust_sdk_to_the_tree_above_the_sdk() {
    let b = rust();
    let project = project_with(&[("rust-toolchain.toml", &sdk_toolchain())]);
    b.prepare(project.path()).unwrap();
    let target = fs::read_link(project.path().join("build/rust-sdk")).unwrap();
    assert_eq!(target, b.sdk.root().parent().unwrap());
}

#[test]
fn prepare_refuses_another_nightly_before_linking() {
    let toolchain = "[toolchain]\nchannel = \"nightly-2020-01-01\"\n";
    let project = project_with(&[("rust-toolchain.toml", toolchain)]);
    let err = rust().prepare(project.path()).unwrap_err().to_string();
    assert!(
        err.contains("names the toolchain `nightly-2020-01-01`"),
        "{err}"
    );
    assert!(!project.path().join("build/rust-sdk").exists());
}

#[test]
fn prepare_refuses_paths_into_another_sdk_before_linking() {
    let cargo = "[package]\nname = \"hello\"\n\n[dependencies]\n\
                 symbian-std = { path = \"/old/symbian-rs/crates/symbian-std\" }\n";
    let project = project_with(&[("Cargo.toml", cargo)]);
    let err = rust().prepare(project.path()).unwrap_err().to_string();
    assert!(err.contains("  Cargo.toml:5\n"), "{err}");
    assert!(!project.path().join("build/rust-sdk").exists());
}

/// The SDK's own examples have no `rust-toolchain.toml`: rustup finds `symbian-rs`'s
/// above them.
#[test]
fn a_project_without_a_toolchain_file_of_its_own_is_not_checked() {
    let project = project_with(&[]);
    rust().prepare(project.path()).unwrap();
    assert!(Path::new(&project.path().join("build/rust-sdk")).exists());
}
