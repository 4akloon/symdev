use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use super::StdSysroot;
use crate::rust_sdk::RustSdk;

const HOST: &str = "x86_64-unknown-linux-gnu";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A fake SDK (what `RustSdk::at` requires, `symbian-sys` and an overlay that replaces no
/// toolchain file), a fake toolchain (`library/std` and the host's `lib/`), a `rustc`
/// script answering `--print sysroot` and `-vV` for it, and an empty project.
fn fixture() -> (tempfile::TempDir, RustSdk, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let t = tmp.path();
    for file in RustSdk::REQUIRED {
        write(&t.join("sdk/symbian-rs").join(file), "");
    }
    write(
        &t.join("sdk/symbian-rs/Cargo.toml"),
        "[workspace]\nmembers = []\n",
    );
    write(
        &t.join("sdk/symbian-rs/crates/symbian-sys/Cargo.toml"),
        "[package]\n",
    );
    write(
        &t.join("sdk/symbian-rs/rust-src/overlay.toml"),
        "[replaces]\n",
    );
    write(
        &t.join("sdk/symbian-rs/rust-src/overlay/library/std/src/sys/pal/symbian/mod.rs"),
        "",
    );
    let toolchain = t.join("toolchain");
    write(
        &toolchain.join("lib/rustlib/src/rust/library/std/Cargo.toml"),
        "[package]\n",
    );
    write(
        &toolchain
            .join("lib/rustlib")
            .join(HOST)
            .join("lib/libstd.rlib"),
        "",
    );
    let rustc = t.join("rustc");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n--print) echo {} ;;\n-vV) printf 'rustc 1.0\\nhost: {HOST}\\n' ;;\nesac\n",
        toolchain.display()
    );
    write(&rustc, &script);
    std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
    let project = t.join("project");
    std::fs::create_dir_all(&project).unwrap();
    let sdk = RustSdk::at(&t.join("sdk/symbian-rs")).unwrap();
    (tmp, sdk, rustc, project)
}

#[test]
fn the_sysroot_holds_the_patched_library_and_links_the_host_libraries() {
    let (_tmp, sdk, rustc, project) = fixture();
    let s = StdSysroot::materialise(&sdk, &rustc, &project).unwrap();
    assert_eq!(s.dir(), project.join("build/sysroot"));
    let library = s.dir().join("lib/rustlib/src/rust/library");
    assert!(library.join("std/Cargo.toml").is_file());
    assert!(library.join("symbian-sys/Cargo.toml").is_file());
    assert!(library.join("std/src/sys/pal/symbian/mod.rs").is_file());
    let host = std::fs::read_link(s.dir().join("lib/rustlib").join(HOST)).unwrap();
    assert!(
        host.ends_with(Path::new("lib/rustlib").join(HOST)),
        "{}",
        host.display()
    );
    let wrapper = std::fs::symlink_metadata(project.join("build/symdev-rustc")).unwrap();
    assert!(wrapper.file_type().is_symlink());
}

#[test]
fn a_second_materialise_replaces_the_first() {
    let (_tmp, sdk, rustc, project) = fixture();
    StdSysroot::materialise(&sdk, &rustc, &project).unwrap();
    let stale = project.join("build/sysroot/lib/rustlib/src/rust/library/stale.rs");
    std::fs::write(&stale, "").unwrap();
    let s = StdSysroot::materialise(&sdk, &rustc, &project).unwrap();
    assert!(!stale.exists());
    assert!(std::fs::read_link(s.dir().join("lib/rustlib").join(HOST)).is_ok());
}
