use std::fs;
use std::path::{Path, PathBuf};

use super::ForeignSdkPaths;
use crate::rust_sdk::RustSdk;

/// A Rust SDK tree under `dir` (the package's layout) with the two crates the scaffold
/// names.
fn sdk_in(dir: &Path) -> RustSdk {
    for file in [
        "symbian-rs/targets/arm-symbian-e32.json",
        "symbian-rs/rust-toolchain.toml",
        "symbian-rs/crates/symbian-core/Cargo.toml",
        "symbian-rs/crates/symbian-std/Cargo.toml",
        "crates/symdev-locale/Cargo.toml",
        "Cargo.toml",
    ] {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "\n").unwrap();
    }
    RustSdk::at(&dir.join("symbian-rs")).unwrap()
}

/// A project whose `Cargo.toml` and `.cargo/config.toml` name the SDK at `sdk` as a
/// symdev 0.1.0 scaffold wrote them: `sdk` is the `symbian-rs` path, as written.
fn project_naming(dir: &Path, sdk: &str) -> PathBuf {
    let root = dir.join("app");
    fs::create_dir_all(root.join(".cargo")).unwrap();
    let cargo = format!(
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n\n[dependencies]\n\
         symbian-core = {{ path = \"{sdk}/crates/symbian-core\" }}\n\
         symbian-std = {{ path = \"{sdk}/crates/symbian-std\" }}\n"
    );
    fs::write(root.join("Cargo.toml"), cargo).unwrap();
    let config = format!(
        "[build]\ntarget = \"{sdk}/targets/arm-symbian-e32.json\"\ntarget-dir = \"build/cargo\"\n"
    );
    fs::write(root.join(".cargo/config.toml"), config).unwrap();
    root
}

#[test]
fn a_project_naming_another_sdk_is_told_each_line_to_change() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let old = "/home/u/.local/share/symdev/rust-sdk/0.1.0/symbian-rs";
    let root = project_naming(tmp.path(), old);
    let err = ForeignSdkPaths::find(&root, &sdk)
        .unwrap()
        .check()
        .unwrap_err()
        .to_string();
    let expected = format!(
        "this project names a Rust SDK by absolute path, and not the one this build resolved \
         ({}); symdev links build/rust-sdk to the SDK it builds against, so name the SDK \
         through that link. Change:\n\
         \x20 Cargo.toml:6\n\
         \x20   - symbian-core = {{ path = \"{old}/crates/symbian-core\" }}\n\
         \x20   + symbian-core = {{ path = \"build/rust-sdk/symbian-rs/crates/symbian-core\" }}\n\
         \x20 Cargo.toml:7\n\
         \x20   - symbian-std = {{ path = \"{old}/crates/symbian-std\" }}\n\
         \x20   + symbian-std = {{ path = \"build/rust-sdk/symbian-rs/crates/symbian-std\" }}\n\
         \x20 .cargo/config.toml:2\n\
         \x20   - target = \"{old}/targets/arm-symbian-e32.json\"\n\
         \x20   + target = \"build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json\"",
        sdk.root().display()
    );
    assert_eq!(err, expected);
}

/// A project scaffolded by symdev 0.1.0 still builds while the SDK it names is the one
/// resolved: nothing is mixed.
#[test]
fn the_resolved_sdk_named_by_absolute_path_is_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let root = project_naming(tmp.path(), &sdk.root().display().to_string());
    ForeignSdkPaths::find(&root, &sdk).unwrap().check().unwrap();
}

#[test]
fn the_resolved_sdk_named_through_another_link_is_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let alias = tmp.path().join("current");
    std::os::unix::fs::symlink(tmp.path().join("rust-sdk/0.2.0"), &alias).unwrap();
    let root = project_naming(tmp.path(), &alias.join("symbian-rs").display().to_string());
    ForeignSdkPaths::find(&root, &sdk).unwrap().check().unwrap();
}

#[test]
fn paths_through_build_rust_sdk_are_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let root = project_naming(tmp.path(), "build/rust-sdk/symbian-rs");
    ForeignSdkPaths::find(&root, &sdk).unwrap().check().unwrap();
}

/// `/opt/libs/crates/my-lib` is a crates directory, but `my-lib` is no crate of the SDK.
#[test]
fn an_absolute_path_to_a_crate_the_sdk_does_not_have_is_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let root = tmp.path().join("app");
    fs::create_dir_all(&root).unwrap();
    let cargo = "[package]\nname = \"app\"\n\n[dependencies]\n\
                 my-lib = { path = \"/opt/libs/crates/my-lib\" }\n";
    fs::write(root.join("Cargo.toml"), cargo).unwrap();
    ForeignSdkPaths::find(&root, &sdk).unwrap().check().unwrap();
}

#[test]
fn every_dependency_table_is_looked_at() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    let root = tmp.path().join("app");
    fs::create_dir_all(&root).unwrap();
    let old = "/old/symbian-rs";
    let cargo = format!(
        "[package]\nname = \"app\"\n\n[target.'cfg(target_os = \"none\")'.dev-dependencies]\n\
         symbian-std = {{ path = \"{old}/crates/symbian-std\" }}\n"
    );
    fs::write(root.join("Cargo.toml"), cargo).unwrap();
    let err = ForeignSdkPaths::find(&root, &sdk)
        .unwrap()
        .check()
        .unwrap_err()
        .to_string();
    assert!(err.contains("  Cargo.toml:5\n"), "{err}");
}

#[test]
fn a_project_without_cargo_files_names_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    ForeignSdkPaths::find(tmp.path(), &sdk)
        .unwrap()
        .check()
        .unwrap();
}

#[test]
fn a_cargo_file_that_is_not_toml_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let sdk = sdk_in(&tmp.path().join("rust-sdk/0.2.0"));
    fs::write(tmp.path().join("Cargo.toml"), "[package\n").unwrap();
    let err = ForeignSdkPaths::find(tmp.path(), &sdk)
        .err()
        .unwrap()
        .to_string();
    let path = tmp.path().join("Cargo.toml");
    assert!(err.starts_with(&format!("{}: ", path.display())), "{err}");
}
