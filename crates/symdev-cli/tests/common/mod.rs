// Shared by several integration-test binaries; each uses a different subset.
#![allow(dead_code)]

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use assert_cmd::Command;

pub mod prebuilt;
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

/// `symdev` with none of the developer's `SYMDEV_*` or `PUBLISH_*` variables (toolchain
/// paths, source and publisher keys, the signing password), and with `SYMDEV_HOME` and
/// every `XDG_*` directory in this test process's home, whose `sources.toml` turns the
/// built-in source off: no test can reach the network, use a key or touch the developer's
/// installed packages. A test that needs a variable sets it on the returned command; one
/// that needs a source points the directories elsewhere (`repo::World`).
pub fn bin() -> Command {
    isolated(Command::cargo_bin("symdev").unwrap())
}

/// `cmd` with the environment [`bin`] gives `symdev`.
pub fn isolated(mut cmd: Command) -> Command {
    for (name, _) in std::env::vars_os() {
        let text = name.to_string_lossy();
        if text.starts_with("SYMDEV_") || text.starts_with("PUBLISH_") {
            cmd.env_remove(&name);
        }
    }
    let dir = |name: &str| -> PathBuf { home().join(name) };
    cmd.env("SYMDEV_HOME", dir("data/symdev"))
        .env("XDG_DATA_HOME", dir("data"))
        .env("XDG_CACHE_HOME", dir("cache"))
        .env("XDG_CONFIG_HOME", dir("config"));
    cmd
}

/// This process's home, `CARGO_TARGET_TMPDIR/symdev-cli-homes/home-*`. Its `.lock` stays
/// locked until the process exits (a static is never dropped, so a `TempDir` would stay
/// behind in `target/tmp`); the first call removes every home whose lock it can take —
/// those of finished runs — so homes do not pile up. A home is set up under a dot name
/// and renamed once locked, so a sweep never takes one that is being made.
fn home() -> &'static Path {
    static HOME: OnceLock<(PathBuf, File)> = OnceLock::new();
    let (home, _lock) = HOME.get_or_init(|| {
        let homes = Path::new(env!("CARGO_TARGET_TMPDIR")).join("symdev-cli-homes");
        fs::create_dir_all(&homes).unwrap();
        for entry in fs::read_dir(&homes).unwrap() {
            let dir = entry.unwrap().path();
            let settled = !dir.file_name().unwrap().to_string_lossy().starts_with('.');
            let lock = File::open(dir.join(".lock"));
            if settled && lock.is_ok_and(|lock| lock.try_lock().is_ok()) {
                // Another process may be sweeping it too.
                let _ = fs::remove_dir_all(&dir);
            }
        }
        let new = tempfile::Builder::new()
            .prefix(".home-")
            .tempdir_in(&homes)
            .unwrap()
            .keep();
        let lock = File::create(new.join(".lock")).unwrap();
        lock.lock().unwrap();
        let config = new.join("config/symdev");
        fs::create_dir_all(&config).unwrap();
        fs::write(config.join("sources.toml"), "builtin = false\n").unwrap();
        let name = new.file_name().unwrap().to_string_lossy().into_owned();
        let home = homes.join(name.trim_start_matches('.'));
        fs::rename(&new, &home).unwrap();
        (home, lock)
    });
    home
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
