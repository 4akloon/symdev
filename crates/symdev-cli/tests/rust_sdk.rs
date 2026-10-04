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
    let project = w.rust_project();
    let output = w
        .prebuilt()
        .current_dir(&project)
        .arg("build")
        .env("PATH", path_with_symdev_ld(&w))
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
    // cargo builds against the SDK through `build/rust-sdk` (the project's
    // .cargo/config.toml names it), which the build points at the installed package.
    let link = project.join("build/rust-sdk");
    assert_eq!(
        canonical(&link),
        canonical(&w.package_dir(id.as_str())),
        "{stderr}"
    );
    assert!(stderr.contains("stub cargo build --release"), "{stderr}");
}

/// `PATH` with a `symdev-ld` first: `symdev build` runs cargo only with cargo's linker there.
fn path_with_symdev_ld(w: &World) -> std::ffi::OsString {
    let bin = w.tmp.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("symdev-ld"), "").unwrap();
    let rest = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(&rest))).unwrap()
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
    assert_links_to(&w.tmp.path().join("app"), &w.package_dir(id.as_str()));
}

/// The scaffold names the SDK through `build/rust-sdk` (experiment 110), and links it to
/// `tree`, the directory that holds the SDK's `symbian-rs`.
fn assert_links_to(project: &Path, tree: &Path) {
    let cargo = fs::read_to_string(project.join("Cargo.toml")).unwrap();
    let relative = "path = \"build/rust-sdk/symbian-rs/crates/symbian-std\"";
    assert!(cargo.contains(relative), "{cargo}");
    let link = fs::read_link(project.join("build/rust-sdk")).unwrap();
    assert_eq!(link.display().to_string(), canonical(tree));
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
    let checkout = Path::new(RustSdk::CHECKOUT.unwrap()).join("..");
    assert_links_to(&w.tmp.path().join("app"), &checkout);
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
    fs::write(sdk.join("rust-toolchain.toml"), common::SDK_TOOLCHAIN).unwrap();
    let workspace = "[workspace]\nmembers = []\n";
    fs::write(sdk.join("Cargo.toml"), workspace).unwrap();
    w.bin()
        .current_dir(w.tmp.path())
        .args(["new", "app", "--lang", "rust"])
        .env("SYMDEV_RUST_SDK", &sdk)
        .assert()
        .success();
    assert_links_to(&w.tmp.path().join("app"), &clone);
}

/// A project that a symdev 0.1.0 scaffolded names its SDK by absolute path; against
/// another SDK, the build stops before cargo with the lines to change.
#[test]
fn a_project_naming_another_sdk_by_absolute_path_is_refused_before_cargo() {
    let w = world();
    let project = w.rust_project();
    let old = "/home/u/.local/share/symdev/rust-sdk/0.1.0/symbian-rs";
    let cargo = format!(
        "[package]\nname = \"hello\"\nversion = \"0.1.0\"\n\n[dependencies]\n\
         symbian-std = {{ path = \"{old}/crates/symbian-std\" }}\n"
    );
    fs::write(project.join("Cargo.toml"), cargo).unwrap();
    let stderr = w
        .prebuilt()
        .current_dir(&project)
        .arg("build")
        .env("SYMDEV_CARGO", stub_cargo(&w))
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(stderr).unwrap();
    let fixed = "    + symbian-std = { path = \"build/rust-sdk/symbian-rs/crates/symbian-std\" }";
    assert!(stderr.contains("  Cargo.toml:6\n"), "{stderr}");
    assert!(stderr.contains(fixed), "{stderr}");
    assert!(!stderr.contains("stub cargo"), "{stderr}");
    assert!(!project.join("build/rust-sdk").exists());
}

/// The prebuilt copy differs from the binary under test only in the checkout it names.
#[test]
fn the_prebuilt_copy_names_a_checkout_that_does_not_exist() {
    let missing = common::prebuilt::missing_checkout();
    assert_eq!(missing.len(), RustSdk::CHECKOUT.unwrap().len());
    assert!(!Path::new(&missing).exists());
    assert!(Path::new(RustSdk::CHECKOUT.unwrap()).exists());
}
