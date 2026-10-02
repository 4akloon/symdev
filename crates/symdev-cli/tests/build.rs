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
    // No variable, nothing installed, no source to install from (`bin` turns the
    // built-in source off).
    bin()
        .current_dir(&dir)
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: gcce;12.1.0 was not found: no package source is configured",
        ))
        .stderr(predicate::str::contains("sources.toml"))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn build_omitted_uid3_errors() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "uid3 required for build (set symbian.uid3)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

/// A toolchain whose every `SYMDEV_*` path exists (the files are empty): enough for a
/// build to get past resolving its toolchain.
fn fake_toolchain(dir: &tempfile::TempDir) -> Vec<(&'static str, std::path::PathBuf)> {
    let tools = dir.path().join("tools");
    std::fs::create_dir_all(&tools).unwrap();
    let file = |name: &str| {
        let path = tools.join(name);
        std::fs::write(&path, b"").unwrap();
        path
    };
    vec![
        ("SYMDEV_EPOCROOT", tools.clone()),
        ("SYMDEV_GXX", file("g++")),
        ("SYMDEV_LD", file("ld")),
        ("SYMDEV_GCC_LIB", file("gcc-lib")),
        ("SYMDEV_GCC_TARGET_LIB", file("gcc-target-lib")),
    ]
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
    let elf2e32 = dir.path().join("elf2e32");
    std::fs::write(&elf2e32, b"").unwrap();
    bin()
        .current_dir(&dir)
        .envs(fake_toolchain(&dir))
        .env("SYMDEV_ELF2E32", &elf2e32)
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
        .envs(fake_toolchain(&dir))
        .arg("build")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no bld.inf"))
        .stderr(predicate::str::contains("SYMDEV_ELF2E32").not())
        .stderr(predicate::str::contains("installing").not());
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
