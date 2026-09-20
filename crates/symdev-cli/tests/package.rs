use predicates::prelude::*;

mod common;
use common::{HELLO, bin, dummy_e32, hello_with_uid3, write_toml};

#[test]
fn package_omitted_uid3_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("package")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "uid3 required for package (set symbian.uid3)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn package_missing_e32() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    bin()
        .current_dir(&dir)
        .arg("package")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "E32 not found: build/hello.exe (run symdev build)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn package_missing_epocroot_still_packages() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    dummy_e32(&dir);
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EPOCROOT")
        .env_remove("SYMDEV_WINE")
        .env("SYMDEV_SIGN_PASSWORD", "secret")
        .arg("package")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("missing toolchain").not())
        .stdout(predicate::str::contains("hello.sisx"));
    assert!(dir.path().join("build/hello.sisx").is_file());
    assert!(dir.path().join("build/hello.cer").is_file());
    assert!(dir.path().join("build/hello.key").is_file());
}

#[test]
fn package_missing_sign_password() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    dummy_e32(&dir);
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EPOCROOT")
        .env_remove("SYMDEV_SIGN_PASSWORD")
        .arg("package")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "SYMDEV_SIGN_PASSWORD must be at least 4 characters",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}
