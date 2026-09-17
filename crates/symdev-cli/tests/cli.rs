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
fn new_not_implemented_creates_no_files() {
    let dir = tempfile::tempdir().unwrap();
    bin()
        .current_dir(&dir)
        .args(["new", "hello", "--target", "nokia-e52"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not implemented: 'symdev new' (unlocks at M4)",
        ));
    assert!(dir.path().read_dir().unwrap().next().is_none());
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
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not implemented: 'symdev new' (unlocks at M4)",
        ))
        .stderr(predicate::str::contains("invalid manifest").not());
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
fn package_valid_manifest_not_implemented() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("package")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not implemented: 'symdev package' (unlocks at M2)",
        ));
}

#[test]
fn deploy_valid_manifest_not_implemented() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("deploy")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "error: not implemented: 'symdev deploy' (unlocks at M4)",
        ));
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
