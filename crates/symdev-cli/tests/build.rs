use predicates::prelude::*;

mod common;
use common::{HELLO, bin, write_toml};

#[test]
fn build_missing_manifest() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .current_dir(&dir)
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: invalid manifest: no symdev.toml in current directory",
        ));
}

#[test]
fn build_valid_manifest_missing_toolchain() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EPOCROOT")
        .env_remove("SYMDEV_GXX")
        .env_remove("SYMDEV_LD")
        .env_remove("SYMDEV_ELF2E32")
        .env_remove("SYMDEV_GCC_LIB")
        .env_remove("SYMDEV_GCC_TARGET_LIB")
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("missing toolchain"))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn build_omitted_uid3_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EPOCROOT")
        .env_remove("SYMDEV_GXX")
        .env_remove("SYMDEV_LD")
        .env_remove("SYMDEV_ELF2E32")
        .env_remove("SYMDEV_GCC_LIB")
        .env_remove("SYMDEV_GCC_TARGET_LIB")
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "uid3 required for build (set symbian.uid3)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn build_valid_manifest_no_bld_inf() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EPOCROOT", "/sdk")
        .env("SYMDEV_GXX", "/gcc/g++")
        .env("SYMDEV_LD", "/gcc/ld")
        .env("SYMDEV_ELF2E32", "/gcc/elf2e32")
        .env("SYMDEV_GCC_LIB", "/gcc/lib")
        .env("SYMDEV_GCC_TARGET_LIB", "/gcc/target")
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no bld.inf"))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn build_does_not_require_external_elf2e32() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &HELLO.replace(
            "capabilities = []",
            "uid3 = \"0xE0000001\"\ncapabilities = []",
        ),
    );
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EPOCROOT", "/sdk")
        .env("SYMDEV_GXX", "/gcc/g++")
        .env("SYMDEV_LD", "/gcc/ld")
        .env_remove("SYMDEV_ELF2E32")
        .env("SYMDEV_GCC_LIB", "/gcc/lib")
        .env("SYMDEV_GCC_TARGET_LIB", "/gcc/target")
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no bld.inf"))
        .stderr(predicate::str::contains("missing toolchain").not());
}

#[test]
fn build_invalid_manifest_not_not_implemented() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &HELLO.replace(r#"name = "cpp""#, r#"name = "java""#));
    bin()
        .current_dir(&dir)
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("error: invalid manifest:"))
        .stderr(predicate::str::contains("not implemented").not());
}
