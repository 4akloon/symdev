use predicates::prelude::*;

mod common;
use common::repo::World;
use common::{bin, hello_with_uid3, write_toml};

#[test]
fn build_offline_without_packages_names_the_install_command() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    bin()
        .current_dir(&dir)
        .args(["build", "--offline"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "--offline forbids downloading them; run `symdev sdk install 'gcce;12.1.0' \
             'sdk;s60-3rd-fp2;1.1'`",
        ));
}

#[test]
fn install_list_and_uninstall_a_package_from_a_file_source() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.add_stub_sdk();
    w.bin()
        .args(["sdk", "install", "gcce;12.1.0"])
        .assert()
        .success()
        .stderr(predicate::str::contains("installing gcce;12.1.0 ("))
        .stderr(predicate::str::contains(" MB) from local…"))
        .stdout("installed  gcce;12.1.0  (local)\n");
    assert!(
        w.package_dir("gcce;12.1.0")
            .join("bin/arm-none-symbianelf-g++")
            .is_file()
    );
    w.bin()
        .args(["sdk", "list"])
        .assert()
        .success()
        .stdout("installed  gcce;12.1.0  (local)\navailable  sdk;s60-3rd-fp2;1.1  (local)\n");
    w.bin()
        .args(["sdk", "list", "--offline"])
        .assert()
        .success()
        .stdout("installed  gcce;12.1.0  (local)\n");
    w.bin()
        .args(["sdk", "uninstall", "gcce;12.1.0"])
        .assert()
        .success()
        .stdout("removed  gcce;12.1.0\n");
    assert!(!w.package_dir("gcce;12.1.0").exists());
    w.bin()
        .args(["sdk", "uninstall", "gcce;12.1.0"])
        .assert()
        .success()
        .stderr(predicate::str::contains("gcce;12.1.0 is not installed"));
}

#[test]
fn install_without_ids_installs_what_the_project_needs() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.add_stub_sdk();
    w.bin()
        .current_dir(w.project())
        .args(["sdk", "install"])
        .assert()
        .success()
        .stdout("installed  gcce;12.1.0  (local)\ninstalled  sdk;s60-3rd-fp2;1.1  (local)\n");
}

#[test]
fn install_without_ids_outside_a_project_says_what_to_name() {
    let w = World::new();
    w.bin()
        .current_dir(w.tmp.path())
        .args(["sdk", "install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no symdev.toml"))
        .stderr(predicate::str::contains("symdev sdk install 'gcce;12.1.0'"));
}

#[test]
fn uninstall_needs_an_id_and_install_refuses_a_bad_one() {
    let w = World::new();
    w.bin().args(["sdk", "uninstall"]).assert().failure();
    w.bin()
        .args(["sdk", "install", "gcce"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("needs a kind and a version"));
}

#[test]
fn build_installs_both_packages_then_runs_the_installed_compiler() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.add_stub_sdk();
    w.bin()
        .current_dir(w.project())
        .arg("build")
        .assert()
        .failure()
        .stderr(predicate::str::contains("installing gcce;12.1.0 ("))
        .stderr(predicate::str::contains("installing sdk;s60-3rd-fp2;1.1 ("))
        .stderr(predicate::str::contains("stub g++"));
    // Installed now: a second build reads no source, so it works without one.
    w.sources("builtin = false\n");
    w.bin()
        .current_dir(w.project())
        .args(["build", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("installing").not())
        .stderr(predicate::str::contains("stub g++"));
}

/// Once everything is installed, what only a download needs — a readable `sources.toml`,
/// whole key pairs — cannot stop a build.
#[test]
fn an_installed_build_ignores_what_only_a_download_needs() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.add_stub_sdk();
    w.bin()
        .current_dir(w.project())
        .args(["sdk", "install"])
        .assert()
        .success();
    w.sources("builtin = maybe\n");
    let stub_compiler_ran = predicate::str::contains("stub g++");
    w.bin()
        .current_dir(w.project())
        .arg("build")
        .assert()
        .failure()
        .stderr(stub_compiler_ran.clone());
    w.sources("[[source]]\nname = \"private\"\nurl = \"https://127.0.0.1:1/b/\"\nauth = \"s3\"\n");
    w.bin()
        .current_dir(w.project())
        .arg("build")
        .env("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "AKID")
        .assert()
        .failure()
        .stderr(stub_compiler_ran);
}

/// A Rust build that cannot find its Rust SDK says so before anything is downloaded.
#[test]
fn a_rust_build_without_its_rust_sdk_downloads_nothing() {
    let mut w = World::new();
    w.add_stub_gcce();
    w.add_stub_sdk();
    let project = w.project();
    let toml = std::fs::read_to_string(project.join("symdev.toml")).unwrap();
    let rust = toml.replace(r#"name = "cpp""#, r#"name = "rust""#);
    std::fs::write(project.join("symdev.toml"), rust).unwrap();
    w.bin()
        .current_dir(&project)
        .arg("build")
        .env("SYMDEV_RUST_SDK", "/nonexistent/symdev/symbian-rs")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "SYMDEV_RUST_SDK is set, but Rust SDK not found at /nonexistent/symdev/symbian-rs",
        ))
        .stderr(predicate::str::contains("installing").not());
    assert!(!w.package_dir("gcce;12.1.0").exists());
}

#[test]
fn an_sdk_only_in_a_keyless_private_source_names_the_keys_and_the_epocroot() {
    let mut w = World::new();
    w.add_stub_gcce();
    let local = format!("file://{}", w.repo().display());
    w.sources(&format!(
        "builtin = false\n\n[[source]]\nname = \"local\"\nurl = \"{local}\"\n\n\
         [[source]]\nname = \"private\"\nurl = \"https://127.0.0.1:1/bucket/\"\nauth = \"s3\"\n"
    ));
    w.bin()
        .current_dir(w.project())
        .arg("build")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "source `private` was skipped because its keys are not set: set \
             SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID and SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY, \
             or set SYMDEV_EPOCROOT to your own SDK",
        ));
}

#[test]
fn an_id_in_no_source_names_the_sources_searched() {
    let w = World::new();
    w.bin()
        .args(["sdk", "install", "gcce;99.0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "error: gcce;99.0 was not found in the sources searched: `local`",
        ));
}

#[test]
fn an_sdk_in_no_source_names_the_epocroot_and_this_sources_toml() {
    let w = World::new();
    let sources = w.tmp.path().join("config/symdev/sources.toml");
    w.bin()
        .args(["sdk", "install", "sdk;s60-3rd-fp2;1.1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(format!(
            "error: sdk;s60-3rd-fp2;1.1 was not found in the sources searched: `local`; set \
             SYMDEV_EPOCROOT to your own SDK, or add a source that has it in {}",
            sources.display()
        )));
}

#[test]
fn package_installs_nothing() {
    let mut w = World::new();
    w.add_stub_sdk();
    let project = w.project();
    std::fs::create_dir_all(project.join("build")).unwrap();
    std::fs::write(project.join("build/hello.exe"), b"").unwrap();
    w.bin()
        .current_dir(&project)
        .arg("package")
        .assert()
        .failure()
        .stderr(predicate::str::contains("installing").not())
        .stderr(predicate::str::contains(
            "`symdev package` installs nothing",
        ));
    assert!(!w.package_dir("sdk;s60-3rd-fp2;1.1").exists());
}

#[test]
fn a_set_variable_that_does_not_exist_names_itself() {
    let w = World::new();
    w.bin()
        .current_dir(w.project())
        .arg("build")
        .env("SYMDEV_GXX", "/nonexistent/symdev/g++")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "SYMDEV_GXX is set to /nonexistent/symdev/g++, which does not exist",
        ));
}

/// A partial id names a directory that holds a package; uninstalling it removes nothing.
#[test]
fn uninstall_of_a_partial_id_keeps_the_package_it_holds() {
    let mut w = World::new();
    w.add_stub_sdk();
    w.bin()
        .args(["sdk", "install", "sdk;s60-3rd-fp2;1.1"])
        .assert()
        .success();
    w.bin()
        .args(["sdk", "uninstall", "sdk;s60-3rd-fp2"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "its directory holds the installed package sdk;s60-3rd-fp2;1.1",
        ));
    assert!(w.package_dir("sdk;s60-3rd-fp2;1.1").join("epoc32").is_dir());
}
