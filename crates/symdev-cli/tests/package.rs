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

/// gap 11: `symdev build` writes `build/<MMP TARGET>.exe`; packaging must look for that
/// name, not the manifest's, and name the registration resource after it too.
#[test]
fn package_names_the_binary_after_the_mmp_target() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    std::fs::create_dir_all(dir.path().join("group")).unwrap();
    std::fs::write(
        dir.path().join("group/bld.inf"),
        "PRJ_MMPFILES\nPuzzles.mmp\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("group/Puzzles.mmp"),
        "TARGET Puzzles_0xa000ef77.exe\nTARGETTYPE EXE\nSOURCE main.cpp\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/Puzzles_0xa000ef77.exe"), b"").unwrap();
    bin()
        .current_dir(&dir)
        .env("SYMDEV_SIGN_PASSWORD", "secret")
        .arg("package")
        .assert()
        .success()
        .stdout(predicate::str::contains("hello.sisx"));
    let pkg = std::fs::read_to_string(dir.path().join("build/hello.pkg")).unwrap();
    assert!(
        pkg.contains("\"Puzzles_0xa000ef77.exe\"\t\t-\"!:\\sys\\bin\\Puzzles_0xa000ef77.exe\""),
        "{pkg}"
    );
    assert!(pkg.contains("Puzzles_0xa000ef77_reg.rsc"), "{pkg}");
    assert!(
        dir.path()
            .join("build/Puzzles_0xa000ef77_reg.rsc")
            .is_file()
    );
}

/// gap 12: a project ships files the MMPs do not build (a bitmap store, a font).
#[test]
fn package_carries_install_files_from_the_manifest() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &format!(
            "{}\n[[install]]\nsource = \"gfx/games.mbm\"\ndest = \"\\\\resource\\\\apps\\\\games.mbm\"\n",
            hello_with_uid3()
        ),
    );
    dummy_e32(&dir);
    std::fs::create_dir_all(dir.path().join("gfx")).unwrap();
    std::fs::write(dir.path().join("gfx/games.mbm"), b"mbm").unwrap();
    bin()
        .current_dir(&dir)
        .env("SYMDEV_SIGN_PASSWORD", "secret")
        .arg("package")
        .assert()
        .success();
    let pkg = std::fs::read_to_string(dir.path().join("build/hello.pkg")).unwrap();
    assert!(pkg.contains("-\"!:\\resource\\apps\\games.mbm\""), "{pkg}");
}

#[test]
fn package_reports_a_missing_install_file() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(
        &dir,
        &format!(
            "{}\n[[install]]\nsource = \"gfx/games.mbm\"\ndest = \"\\\\resource\\\\apps\\\\games.mbm\"\n",
            hello_with_uid3()
        ),
    );
    dummy_e32(&dir);
    bin()
        .current_dir(&dir)
        .env("SYMDEV_SIGN_PASSWORD", "secret")
        .arg("package")
        .assert()
        .failure()
        .stderr(predicate::str::contains("file to install not found"))
        .stderr(predicate::str::contains("games.mbm"));
}
