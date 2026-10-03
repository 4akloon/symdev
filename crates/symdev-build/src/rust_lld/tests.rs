use std::path::PathBuf;

use super::RustLld;

const VERBOSE: &str = "rustc 1.93.0-nightly (abcdef012 2026-09-18)\nbinary: rustc\n\
commit-hash: abcdef0123456789\ncommit-date: 2026-09-18\nhost: x86_64-unknown-linux-gnu\n\
release: 1.93.0-nightly\nLLVM version: 23.1.1\n";

#[test]
fn rust_lld_is_in_the_sysroots_rustlib_for_the_host() {
    let lld = RustLld::in_sysroot("/r/toolchains/nightly\n", VERBOSE).unwrap();
    assert_eq!(
        lld.path(),
        PathBuf::from("/r/toolchains/nightly/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld")
    );
}

#[test]
fn output_without_a_sysroot_or_a_host_is_refused() {
    let e = RustLld::in_sysroot("\n", VERBOSE).unwrap_err().to_string();
    assert!(e.contains("rustc --print sysroot"), "{e}");
    let e = RustLld::in_sysroot("/r", "rustc 1.0\n")
        .unwrap_err()
        .to_string();
    assert!(e.contains("host:"), "{e}");
}
