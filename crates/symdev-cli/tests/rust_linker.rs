//! Which linker links a Rust project, and what it needs installed (experiment 113):
//! rust-lld with a `rust-sdk` that has the prebuilt set needs no GCCE; GNU ld
//! (`SYMDEV_RUST_LINKER=gnu`) and a Rust SDK without the set do.

use predicates::prelude::*;
use symdev_sdk::Pins;

mod common;
use common::repo::World;

fn world(prebuilt: bool) -> World {
    let mut w = World::new();
    match prebuilt {
        true => w.add_stub_rust_sdk_with_prebuilt(),
        false => w.add_stub_rust_sdk(),
    }
    w.add_stub_gcce();
    w.add_stub_sdk();
    w
}

#[test]
fn sdk_install_for_a_rust_project_with_the_prebuilt_set_installs_no_gcce() {
    let w = world(true);
    w.prebuilt()
        .current_dir(w.rust_project())
        .args(["sdk", "install"])
        .assert()
        .success()
        .stdout(format!(
            "installed  {}  (local)\ninstalled  sdk;s60-3rd-fp2;1.1  (local)\n",
            Pins::rust_sdk()
        ));
    assert!(!w.package_dir("gcce;12.1.0").exists());
}

#[test]
fn building_a_rust_project_with_the_prebuilt_set_installs_no_gcce() {
    let w = world(true);
    w.prebuilt()
        .current_dir(w.rust_project())
        .arg("build")
        .env("SYMDEV_CARGO", w.stub_cargo())
        .assert()
        .failure()
        .stderr(predicate::str::contains("stub cargo build"))
        .stderr(predicate::str::contains("installing sdk;s60-3rd-fp2;1.1 ("))
        .stderr(predicate::str::contains("gcce").not());
    assert!(!w.package_dir("gcce;12.1.0").exists());
}

#[test]
fn gnu_ld_on_request_installs_gcce_beside_the_prebuilt_set() {
    let w = world(true);
    w.prebuilt()
        .current_dir(w.rust_project())
        .args(["sdk", "install"])
        .env("SYMDEV_RUST_LINKER", "gnu")
        .assert()
        .success()
        .stdout(predicate::str::contains("installed  gcce;12.1.0  (local)"));
}

#[test]
fn rust_lld_without_the_prebuilt_set_compiles_the_shims_so_installs_gcce() {
    let w = world(false);
    w.prebuilt()
        .current_dir(w.rust_project())
        .arg("build")
        .env("SYMDEV_CARGO", w.stub_cargo())
        .assert()
        .failure()
        .stderr(predicate::str::contains("installing gcce;12.1.0 ("));
}

#[test]
fn an_unknown_linker_is_refused_with_the_two_values() {
    let w = world(true);
    w.prebuilt()
        .current_dir(w.rust_project())
        .arg("build")
        .env("SYMDEV_RUST_LINKER", "mold")
        .assert()
        .failure()
        .stderr(predicate::str::contains("SYMDEV_RUST_LINKER is `mold`"))
        .stderr(predicate::str::contains("`gnu`"));
}
