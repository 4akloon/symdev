use assert_cmd::Command;
use predicates::prelude::*;

const HELLO: &str = r#"
[package]
name = "hello"
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
capabilities = []
vendor = "symdev"

[signing]
mode = "self-signed"
"#;

fn bin() -> Command {
    Command::cargo_bin("symdev").unwrap()
}

fn write_toml(dir: &tempfile::TempDir, src: &str) {
    std::fs::write(dir.path().join("symdev.toml"), src).unwrap();
}

fn hello_with_uid3() -> String {
    HELLO.replace(
        "capabilities = []",
        "uid3 = \"0xE0000001\"\ncapabilities = []",
    )
}

fn dummy_e32(dir: &tempfile::TempDir) {
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/hello.exe"), b"").unwrap();
}

#[test]
fn help_lists_only_four_commands() {
    let assert = bin().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for cmd in ["new", "build", "package", "deploy"] {
        assert!(stdout.contains(cmd), "missing {cmd}: {stdout}");
    }
    for cmd in [
        "doctor",
        "test",
        "run",
        "debug",
        "sdk",
        "toolchain",
        "emulator",
        "devices",
    ] {
        assert!(!stdout.contains(cmd), "help must not list {cmd}: {stdout}");
    }
    assert!(
        !stdout
            .lines()
            .any(|line| line.trim_start() == "help" || line.trim_start().starts_with("help ")),
        "help must not be listed as a subcommand: {stdout}"
    );
}

#[test]
fn no_args_prints_help() {
    bin()
        .assert()
        .success()
        .stdout(predicate::str::contains("new"))
        .stdout(predicate::str::contains("build"));
}

#[test]
fn doctor_is_unknown_subcommand() {
    bin().arg("doctor").assert().failure().code(2);
}

#[test]
fn new_creates_hello_tree() {
    let dir = tempfile::tempdir().unwrap();
    let hello = dir.path().join("hello");
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(format!("{}\n", hello.display())));

    assert!(hello.join("symdev.toml").is_file());
    assert!(hello.join("group/bld.inf").is_file());
    assert!(hello.join("group/hello.mmp").is_file());
    assert!(hello.join("src/hello.cpp").is_file());
    assert!(hello.join("src/hello.h").is_file());

    let toml = std::fs::read_to_string(hello.join("symdev.toml")).unwrap();
    assert!(toml.contains("uid3 = \"0xef9f2cab\""));
    assert_eq!(
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        include_bytes!("../templates/hello.cpp")
    );
}

#[test]
fn new_existing_dir_errors() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success();

    let hello = dir.path().join("hello");
    let snapshot = [
        std::fs::read(hello.join("symdev.toml")).unwrap(),
        std::fs::read(hello.join("group/bld.inf")).unwrap(),
        std::fs::read(hello.join("group/hello.mmp")).unwrap(),
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        std::fs::read(hello.join("src/hello.h")).unwrap(),
    ];

    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: directory `hello` already exists",
        ));

    assert_eq!(
        std::fs::read(hello.join("symdev.toml")).unwrap(),
        snapshot[0]
    );
    assert_eq!(
        std::fs::read(hello.join("group/bld.inf")).unwrap(),
        snapshot[1]
    );
    assert_eq!(
        std::fs::read(hello.join("group/hello.mmp")).unwrap(),
        snapshot[2]
    );
    assert_eq!(
        std::fs::read(hello.join("src/hello.cpp")).unwrap(),
        snapshot[3]
    );
    assert_eq!(
        std::fs::read(hello.join("src/hello.h")).unwrap(),
        snapshot[4]
    );
}

#[test]
fn new_rejects_invalid_name() {
    bin()
        .args(["new", "1bad", "--target", "nokia-e52"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn new_rejects_java() {
    bin()
        .args(["new", "hello", "--target", "nokia-e52", "--lang", "java"])
        .assert()
        .failure()
        .code(2);
}

#[test]
fn new_does_not_read_toml() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &HELLO.replace(r#"name = "cpp""#, r#"name = "java""#));
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .success()
        .stderr(predicate::str::contains("invalid manifest").not());
    assert!(dir.path().join("hello").is_dir());
}

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
fn package_valid_manifest_missing_toolchain() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    dummy_e32(&dir);
    bin()
        .current_dir(&dir)
        .env_remove("SYMDEV_EPOCROOT")
        .arg("package")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "missing toolchain: SYMDEV_EPOCROOT",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn package_missing_sign_password() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    dummy_e32(&dir);
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EPOCROOT", "/sdk")
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

#[test]
fn deploy_missing_sisx() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("deploy")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "SISX not found: build/hello.sisx (run symdev package)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn deploy_prints_existing_sisx_path() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    let sisx = dir.path().join("build/hello.sisx");
    std::fs::write(&sisx, b"").unwrap();
    bin()
        .current_dir(&dir)
        .arg("deploy")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(format!("{}\n", sisx.display())));
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
