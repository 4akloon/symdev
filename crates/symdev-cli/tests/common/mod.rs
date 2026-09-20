// Shared by several integration-test binaries; each uses a different subset.
#![allow(dead_code)]

use assert_cmd::Command;

pub const HELLO: &str = r#"
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

pub fn bin() -> Command {
    Command::cargo_bin("symdev").unwrap()
}

pub fn write_toml(dir: &tempfile::TempDir, src: &str) {
    std::fs::write(dir.path().join("symdev.toml"), src).unwrap();
}

pub fn hello_with_uid3() -> String {
    HELLO.replace(
        "capabilities = []",
        "uid3 = \"0xE0000001\"\ncapabilities = []",
    )
}

pub fn dummy_e32(dir: &tempfile::TempDir) {
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    std::fs::write(dir.path().join("build/hello.exe"), b"").unwrap();
}

/// A minimal `EPOCROOT` for tests that parse a `bld.inf`: the front end preprocesses
/// project files, and that needs the variant header the SDK names in `variant.cfg`.
pub fn fake_epocroot(dir: &tempfile::TempDir) -> std::path::PathBuf {
    let root = dir.path().join("sdk");
    let variant = root.join("epoc32/include/variant");
    std::fs::create_dir_all(&variant).unwrap();
    std::fs::create_dir_all(root.join("epoc32/tools/variant")).unwrap();
    std::fs::write(
        variant.join("Symbian_OS_v9.3.hrh"),
        "#define __SERIES60_3X__\n",
    )
    .unwrap();
    std::fs::write(
        root.join("epoc32/tools/variant/variant.cfg"),
        "epoc32\\include\\variant\\Symbian_OS_v9.3.hrh\n",
    )
    .unwrap();
    root
}
