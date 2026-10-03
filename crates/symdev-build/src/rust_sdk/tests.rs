use symdev_sdk::RustSdkPackage;

use super::*;

#[test]
fn checkout_sdk_is_found_and_has_the_target() {
    let sdk = RustSdk::at(Path::new(RustSdk::CHECKOUT.unwrap())).unwrap();
    assert!(sdk.target_spec().ends_with("targets/arm-symbian-e32.json"));
    assert!(sdk.crate_dir("symbian-std").join("Cargo.toml").is_file());
    assert!(RustSdk::HELLO_MAIN.contains("#[symbian_std::main]"));
    assert!(RustSdk::HELLO_MAIN.contains("fn main() -> Result<()>"));
    assert!(!RustSdk::HELLO_MAIN.contains("no_main"));
}

/// A release build (`SYMDEV_RELEASE` set when it is compiled) has no checkout to fall
/// back on: on another machine, anyone could plant a `symbian-rs` at the path of the
/// machine that built it.
#[test]
fn a_release_build_has_no_checkout() {
    let tree = "/build/symdev/crates/symdev-build/../../symbian-rs";
    assert_eq!(RustSdk::checkout(Some("1"), tree), None);
    assert_eq!(RustSdk::checkout(None, tree), Some(tree));
    assert_eq!(RustSdk::checkout(Some(""), tree), Some(tree));
}

/// A tree holding `files` (relative paths), as an installed package would. Among them,
/// `symbian-rs/Cargo.toml` is a workspace with one member, `examples/hello`, which is
/// there too.
fn tree_with(files: &[&str]) -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    for file in files {
        let path = tmp.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"{}").unwrap();
    }
    if files.contains(&"symbian-rs/Cargo.toml") {
        let workspace = "[workspace]\nmembers = [\"examples/hello\"]\n";
        std::fs::write(tmp.path().join("symbian-rs/Cargo.toml"), workspace).unwrap();
        let hello = tmp.path().join("symbian-rs/examples/hello");
        std::fs::create_dir_all(&hello).unwrap();
        std::fs::write(hello.join("Cargo.toml"), "").unwrap();
    }
    tmp
}

/// `RustSdkPackage::REQUIRED` is exactly what `at` checks: the `symbian-rs` of a package
/// with those files is an SDK, and that of a package missing any one of them is not.
#[test]
fn the_installed_package_checks_what_at_requires() {
    let required = RustSdkPackage::REQUIRED;
    let whole = tree_with(required);
    RustSdk::at(&whole.path().join(RustSdkPackage::SDK_DIR)).unwrap();
    for missing in required {
        let rest: Vec<_> = required.iter().copied().filter(|f| f != missing).collect();
        let partial = tree_with(&rest);
        std::fs::create_dir_all(partial.path().join(RustSdkPackage::SDK_DIR)).unwrap();
        let sdk = partial.path().join(RustSdkPackage::SDK_DIR);
        assert!(RustSdk::at(&sdk).is_err(), "{missing}");
    }
}

/// Any cargo build in the SDK's workspace — the libcalls build — loads every member.
#[test]
fn an_sdk_missing_a_member_of_its_workspace_is_no_sdk() {
    let tree = tree_with(RustSdkPackage::REQUIRED);
    let sdk = tree.path().join(RustSdkPackage::SDK_DIR);
    std::fs::remove_file(sdk.join("examples/hello/Cargo.toml")).unwrap();
    let err = RustSdk::at(&sdk).unwrap_err().to_string();
    assert!(
        err.contains("has no examples/hello/Cargo.toml, a member of its workspace"),
        "{err}"
    );
}

#[test]
fn a_symbian_rs_without_symdev_locale_beside_it_is_no_sdk() {
    let alone = tree_with(&[
        "symbian-rs/targets/arm-symbian-e32.json",
        "symbian-rs/rust-toolchain.toml",
        "symbian-rs/Cargo.toml",
    ]);
    let err = RustSdk::at(&alone.path().join("symbian-rs"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("../crates/symdev-locale/Cargo.toml"), "{err}");
}

#[test]
fn missing_sdk_names_the_path() {
    let err = RustSdk::at(Path::new("/nonexistent/symbian-rs")).unwrap_err();
    assert!(
        err.to_string()
            .starts_with("Rust SDK not found at /nonexistent/symbian-rs ("),
        "{err}"
    );
}

#[test]
fn a_directory_without_the_target_spec_is_no_sdk() {
    let empty = tree_with(&[]);
    let err = RustSdk::at(empty.path()).unwrap_err().to_string();
    assert!(err.contains("has no targets/arm-symbian-e32.json"), "{err}");
}

#[test]
fn the_checkout_names_its_nightly() {
    let sdk = RustSdk::at(Path::new(RustSdk::CHECKOUT.unwrap())).unwrap();
    let toolchain = sdk.toolchain().unwrap();
    assert!(toolchain.channel().unwrap().starts_with("nightly-"));
    assert_eq!(toolchain.path(), sdk.root().join("rust-toolchain.toml"));
}

#[test]
fn an_sdk_whose_toolchain_file_names_no_channel_says_so() {
    let tree = tree_with(RustSdkPackage::REQUIRED);
    let sdk = RustSdk::at(&tree.path().join(RustSdkPackage::SDK_DIR)).unwrap();
    let file = sdk.root().join("rust-toolchain.toml");
    std::fs::write(file, "[toolchain]\ncomponents = [\"rust-src\"]\n").unwrap();
    let err = sdk.toolchain().unwrap_err().to_string();
    assert!(
        err.contains("has no rust-toolchain.toml that names a [toolchain] channel"),
        "{err}"
    );
    assert!(err.contains(&sdk.root().display().to_string()), "{err}");
}

#[test]
fn the_checkout_ships_the_lld_script_and_no_prebuilt_set() {
    let sdk = RustSdk::at(Path::new(RustSdk::CHECKOUT.unwrap())).unwrap();
    let script = sdk.lld_script().unwrap();
    assert!(script.ends_with("targets/symbian-lld.ld"));
    assert!(std::fs::read_to_string(script).unwrap().contains("PHDRS"));
    // `prebuilt/` is made by the release recipe, never tracked.
    assert_eq!(sdk.prebuilt().unwrap(), None);
}

#[test]
fn an_sdk_without_the_lld_script_names_the_way_back_to_gnu_ld() {
    let tree = tree_with(RustSdkPackage::REQUIRED);
    let sdk = RustSdk::at(&tree.path().join("symbian-rs")).unwrap();
    let e = sdk.lld_script().unwrap_err().to_string();
    assert!(e.contains("targets/symbian-lld.ld"), "{e}");
    assert!(e.contains("SYMDEV_RUST_LINKER=gnu"), "{e}");
}
