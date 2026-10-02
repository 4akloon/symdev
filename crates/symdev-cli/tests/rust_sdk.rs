//! Where a Rust project's Rust SDK comes from (spec §12): `SYMDEV_RUST_SDK`, else the
//! checkout symdev was built from, else the `rust-sdk` package, installed like GCCE.

use std::fs;
use std::path::{Path, PathBuf};

use predicates::prelude::*;
use symdev_build::RustSdk;
use symdev_sdk::{Pins, RustSdkPackage};

mod common;
use common::repo::World;

/// A cargo that prints its arguments and fails, so a build stops right after the SDK is
/// chosen and shows which one it chose.
fn stub_cargo(w: &World) -> PathBuf {
    let path = w.tmp.path().join("cargo");
    fs::write(&path, "#!/bin/sh\necho \"stub cargo $*\" >&2\nexit 1\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn world() -> World {
    let mut w = World::new();
    w.add_stub_rust_sdk();
    w.add_stub_gcce();
    w.add_stub_sdk();
    w
}

fn canonical(path: &Path) -> String {
    path.canonicalize().unwrap().display().to_string()
}

#[test]
fn a_prebuilt_symdev_installs_the_rust_sdk_first_and_builds_against_it() {
    let w = world();
    let id = Pins::rust_sdk();
    let output = w
        .prebuilt()
        .current_dir(w.rust_project())
        .arg("build")
        .env("SYMDEV_CARGO", stub_cargo(&w))
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(output).unwrap();
    let rust_sdk = stderr.find(&format!("installing {id} (")).expect(&stderr);
    let gcce = stderr.find("installing gcce;12.1.0 (").expect(&stderr);
    assert!(rust_sdk < gcce, "{stderr}");
    let sdk = canonical(&w.package_dir(id.as_str()).join("symbian-rs"));
    let target = format!("--target {sdk}/targets/arm-symbian-e32.json");
    assert!(stderr.contains("stub cargo build"), "{stderr}");
    assert!(stderr.contains(&target), "{stderr}");
}

#[test]
fn a_prebuilt_symdev_installs_the_rust_sdk_with_what_a_rust_project_needs() {
    let w = world();
    let id = Pins::rust_sdk();
    w.prebuilt()
        .current_dir(w.rust_project())
        .args(["sdk", "install"])
        .assert()
        .success()
        .stdout(format!(
            "installed  {id}  (local)\ninstalled  gcce;12.1.0  (local)\ninstalled  \
             sdk;s60-3rd-fp2;1.1  (local)\n"
        ));
    // A C++ project needs no Rust SDK.
    w.prebuilt()
        .args(["sdk", "uninstall", id.as_str()])
        .assert()
        .success();
    w.prebuilt()
        .current_dir(w.project())
        .args(["sdk", "install"])
        .assert()
        .success()
        .stdout(predicate::str::contains("rust-sdk").not());
}

#[test]
fn a_prebuilt_symdev_offline_names_the_install_command_and_the_variable() {
    let w = world();
    let id = Pins::rust_sdk();
    w.prebuilt()
        .current_dir(w.rust_project())
        .args(["build", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "run `symdev sdk install {}`; or set SYMDEV_RUST_SDK to a symbian-rs directory",
            id.shell_word()
        )));
    assert!(!w.package_dir(id.as_str()).exists());
    assert!(!w.package_dir("gcce;12.1.0").exists());
}

#[test]
fn a_prebuilt_symdev_scaffolds_against_the_installed_package() {
    let w = world();
    let id = Pins::rust_sdk();
    w.prebuilt()
        .current_dir(w.tmp.path())
        .args(["new", "app", "--lang", "rust"])
        .assert()
        .success()
        .stderr(predicate::str::contains(format!("installing {id} (")));
    let cargo = fs::read_to_string(w.tmp.path().join("app/Cargo.toml")).unwrap();
    let sdk = canonical(&w.package_dir(id.as_str()).join("symbian-rs"));
    assert!(
        cargo.contains(&format!("{sdk}/crates/symbian-std")),
        "{cargo}"
    );
}

#[test]
fn a_symdev_built_here_scaffolds_against_its_checkout_and_installs_nothing() {
    let w = world();
    w.bin()
        .current_dir(w.tmp.path())
        .args(["new", "app", "--lang", "rust"])
        .assert()
        .success()
        .stderr(predicate::str::contains("installing").not());
    let cargo = fs::read_to_string(w.tmp.path().join("app/Cargo.toml")).unwrap();
    let checkout = canonical(Path::new(RustSdk::CHECKOUT));
    assert!(
        cargo.contains(&format!("{checkout}/crates/symbian-std")),
        "{cargo}"
    );
    assert!(!w.package_dir(Pins::rust_sdk().as_str()).exists());
}

#[test]
fn the_variable_wins_over_the_checkout() {
    let w = world();
    let clone = w.tmp.path().join("my-symdev");
    for file in RustSdkPackage::REQUIRED {
        fs::create_dir_all(clone.join(file).parent().unwrap()).unwrap();
        fs::write(clone.join(file), "{}\n").unwrap();
    }
    let sdk = clone.join(RustSdkPackage::SDK_DIR);
    w.bin()
        .current_dir(w.tmp.path())
        .args(["new", "app", "--lang", "rust"])
        .env("SYMDEV_RUST_SDK", &sdk)
        .assert()
        .success();
    let cargo = fs::read_to_string(w.tmp.path().join("app/Cargo.toml")).unwrap();
    assert!(
        cargo.contains(&format!("{}/crates/symbian-std", canonical(&sdk))),
        "{cargo}"
    );
}

/// The prebuilt copy differs from the binary under test only in the checkout it names.
#[test]
fn the_prebuilt_copy_names_a_checkout_that_does_not_exist() {
    let missing = common::prebuilt::missing_checkout();
    assert_eq!(missing.len(), RustSdk::CHECKOUT.len());
    assert!(!Path::new(&missing).exists());
    assert!(Path::new(RustSdk::CHECKOUT).exists());
}
