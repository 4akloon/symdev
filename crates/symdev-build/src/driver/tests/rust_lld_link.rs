use std::fs;
use std::path::Path;

use symdev_core::RemotePath;

use super::rust_build::rust;
use super::*;
use crate::rust_sdk::RustSdk;
use crate::sdk_lld_cache::fixture::{archive, dso, object};
use crate::{RustLinker, SdkLldCache};

/// Experiment 112's golden lld ELF: 16 imported functions called through lld's PLT.
const HELLO_LLD: &str = include_str!("../../../../symdev-elf2e32/src/testdata/hello_lld.elf.hex");

fn unhex(hex: &str) -> Vec<u8> {
    let digits: Vec<u8> = hex.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}

/// An SDK holding every file a console Rust link names, as the cache can fix them.
fn sdk(root: &Path) -> std::path::PathBuf {
    let armv5 = root.join("epoc32/release/armv5");
    fs::create_dir_all(armv5.join("lib")).unwrap();
    fs::create_dir_all(armv5.join("urel")).unwrap();
    for lib in ["eexe.lib", "usrt2_2.lib"] {
        fs::write(armv5.join("urel").join(lib), archive(&object(2))).unwrap();
    }
    let runtime = [
        "euser",
        "dfpaeabi",
        "dfprvct2_2",
        "drtaeabi",
        "scppnwdl",
        "drtrvct2_2",
    ];
    let dsos = runtime.iter().map(|d| format!("{d}.dso"));
    for name in dsos.chain(RustSdk::LIBRARIES.iter().map(|l| l.to_string())) {
        fs::write(armv5.join("lib").join(name), dso(b" ")).unwrap();
    }
    root.to_path_buf()
}

/// A rust-lld that logs its argv (one per line, then `--`) and writes `elf` to its `-o`.
fn stub_lld(dir: &Path, elf: &Path) -> std::path::PathBuf {
    let path = dir.join("rust-lld");
    let log = dir.join("argv.log");
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do echo \"$a\"; done >> {log}\necho -- >> {log}\n\
         while [ $# -gt 0 ]; do [ \"$1\" = -o ] && cp {elf} \"$2\"; shift; done\n",
        log = log.display(),
        elf = elf.display()
    );
    fs::write(&path, script).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[test]
fn two_links_around_the_stubs_and_a_plt_left_after_them_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let (got, log) = link_with(tmp.path(), &unhex(HELLO_LLD), false);
    let e = got.unwrap_err().to_string();
    let build = tmp.path().join("build");
    // The stub's "second link" is the first one again, so its PLT calls are still there.
    assert!(e.contains("_ZN4User4ExitEi"), "{e}");
    assert!(e.contains("SYMDEV_RUST_LINKER=gnu"), "{e}");
    let links: Vec<&str> = log.split("--\n").filter(|l| !l.is_empty()).collect();
    assert_eq!(links.len(), 2, "{log}");
    assert!(links[0].contains(&format!("{}\n", build.join("hello.first.elf").display())));
    let wraps = links[1]
        .lines()
        .filter(|l| l.starts_with("--wrap="))
        .count();
    assert_eq!(wraps, 16);
    assert!(links[1].contains("import_stubs.o\n"));
    assert!(build.join("import_stubs.o").is_file());
}

/// Experiment 109's GNU ELF of the same program, which no lld link writes.
const HELLO_GNU: &str = include_str!("../../../../symdev-elf2e32/src/testdata/hello.elf.hex");

/// `link_lld` on the fake SDK with a stub rust-lld that writes `elf` (or fails with `fail`).
fn link_with(tmp: &Path, elf: &[u8], fail: bool) -> (Result<(), symdev_core::Error>, String) {
    let golden = tmp.join("golden.elf");
    fs::write(&golden, elf).unwrap();
    let lld = stub_lld(tmp, &golden);
    if fail {
        fs::write(
            &lld,
            "#!/bin/sh\necho 'rust-lld: error: boom' >&2\nexit 1\n",
        )
        .unwrap();
    }
    let cache = SdkLldCache::at(tmp.join("cache"));
    let b = RustBuild {
        gcce: fake_at(sdk(&tmp.join("sdk"))),
        linker: RustLinker::Lld {
            rust_lld: Some(lld.clone()),
            cache: cache.clone(),
        },
        ..rust()
    };
    let build = tmp.join("build");
    fs::create_dir_all(&build).unwrap();
    let (archive, libcalls) = (build.join("libhello.a"), build.join("libcalls.rlib"));
    let inputs = LinkInputs {
        rust: std::slice::from_ref(&archive),
        shims: &[],
        libcalls: &libcalls,
    };
    let got = b.link_lld(
        &lld,
        &cache,
        None,
        &inputs,
        &build.join("hello.elf"),
        &build.join("hello.exe.map"),
        &RemotePath::new(tmp.display().to_string()),
    );
    (
        got,
        fs::read_to_string(tmp.join("argv.log")).unwrap_or_default(),
    )
}

#[test]
fn a_failing_rust_lld_says_which_link_failed_and_how_to_go_back_to_gnu_ld() {
    let tmp = tempfile::tempdir().unwrap();
    let e = link_with(tmp.path(), &unhex(HELLO_LLD), true)
        .0
        .unwrap_err()
        .to_string();
    assert!(e.contains("the first rust-lld link failed"), "{e}");
    assert!(e.contains("rust-lld: error: boom"), "{e}");
    assert!(e.contains("SYMDEV_RUST_LINKER=gnu"), "{e}");
}

#[test]
fn a_first_link_that_lld_did_not_write_is_named() {
    let tmp = tempfile::tempdir().unwrap();
    let e = link_with(tmp.path(), &unhex(HELLO_GNU), false)
        .0
        .unwrap_err()
        .to_string();
    assert!(e.contains("hello.first.elf"), "{e}");
    assert!(e.contains("not linked by lld"), "{e}");
}

/// No PLT call (every `R_ARM_JUMP_SLOT` of the golden made a `R_ARM_GLOB_DAT`): one link,
/// renamed into place, still checked.
#[test]
fn without_plt_calls_the_first_link_is_the_result() {
    let tmp = tempfile::tempdir().unwrap();
    let mut elf = unhex(HELLO_LLD);
    let (shoff, shnum) = (u32_at(&elf, 0x20) as usize, u16_at(&elf, 0x30) as usize);
    for i in 0..shnum {
        let sh = shoff + 40 * i;
        if u32_at(&elf, sh + 4) != 9 {
            continue; // SHT_REL only
        }
        let (off, size) = (
            u32_at(&elf, sh + 16) as usize,
            u32_at(&elf, sh + 20) as usize,
        );
        for at in (off..off + size).step_by(8).map(|e| e + 4) {
            if elf[at] == 22 {
                elf[at] = 21;
            }
        }
    }
    let (got, log) = link_with(tmp.path(), &elf, false);
    got.unwrap();
    assert_eq!(log.matches("--\n").count(), 1, "{log}");
    let build = tmp.path().join("build");
    assert!(build.join("hello.elf").is_file() && !build.join("hello.first.elf").exists());
    assert!(!build.join("import_stubs.o").exists());
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}
