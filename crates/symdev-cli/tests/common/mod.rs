// Shared by several integration-test binaries; each uses a different subset.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use assert_cmd::Command;

pub mod repo;

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

/// The toolchain variables a test may not inherit from the developer's shell when it
/// means "no toolchain configured".
pub const TOOLCHAIN_VARIABLES: [&str; 7] = [
    "SYMDEV_EPOCROOT",
    "SYMDEV_GXX",
    "SYMDEV_LD",
    "SYMDEV_AR",
    "SYMDEV_ELF2E32",
    "SYMDEV_GCC_LIB",
    "SYMDEV_GCC_TARGET_LIB",
];

/// `symdev` with `SYMDEV_HOME` and every `XDG_*` directory in a temporary directory of
/// this test process, whose `sources.toml` turns the built-in source off: no test can
/// reach the network or touch the developer's installed packages. A test that needs a
/// source points these variables somewhere else (`repo::World`).
pub fn bin() -> Command {
    static ROOT: OnceLock<tempfile::TempDir> = OnceLock::new();
    let root = ROOT.get_or_init(|| {
        let tmp = tempfile::Builder::new()
            .prefix("symdev-cli-home-")
            .tempdir_in(env!("CARGO_TARGET_TMPDIR"))
            .unwrap();
        let config = tmp.path().join("config/symdev");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(config.join("sources.toml"), "builtin = false\n").unwrap();
        tmp
    });
    let dir = |name: &str| -> PathBuf { root.path().join(name) };
    let mut cmd = Command::cargo_bin("symdev").unwrap();
    cmd.env("SYMDEV_HOME", dir("data/symdev"))
        .env("XDG_DATA_HOME", dir("data"))
        .env("XDG_CACHE_HOME", dir("cache"))
        .env("XDG_CONFIG_HOME", dir("config"));
    cmd
}

/// [`bin`] without any toolchain variable of the developer's shell.
pub fn bin_without_toolchain() -> Command {
    let mut cmd = bin();
    for variable in TOOLCHAIN_VARIABLES {
        cmd.env_remove(variable);
    }
    cmd
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
