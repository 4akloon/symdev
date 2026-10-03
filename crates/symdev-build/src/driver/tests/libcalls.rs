//! The SDK's compiler-runtime archive: how it is built and where it goes on the link line.
use std::path::{Path, PathBuf};

use super::rust_build::rust;
use super::s;
use crate::rust_sdk::RustSdk;
use symdev_core::Project;

/// The compiler-runtime archive is searched **after** the application archive and the
/// C++ shim, because both may refer to a routine it defines and an archive is searched
/// only for what is undefined where it appears.
///
/// It is a separate archive on purpose: its entry points are `#[unsafe(no_mangle)]`, so
/// they are global symbols and therefore `--gc-sections` roots in a `-shared` link.
/// Compiled into the application instead, they survived into every program and cost
/// `hello` 756 bytes for code it never calls.
#[test]
fn the_libcall_archive_is_searched_last() {
    let b = rust();
    let (a, elf, map) = (
        Path::new("/p/build/cargo/arm-symbian-e32/release/libhello.a"),
        Path::new("/p/build/hello.elf"),
        Path::new("/p/build/hello.exe.map"),
    );
    let project = Project {
        root: PathBuf::from("/p"),
    };
    let libcalls = b.libcalls().path(&project);
    assert_eq!(
        libcalls,
        PathBuf::from("/p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib")
    );
    let shim = b.shim_archive(&project.root.join("build"));
    let got = b
        .link_args(&[a.into()], Some(&shim), Some(&libcalls), elf, map)
        .unwrap();
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at_shim = got.iter().position(|x| x == &shim.display().to_string());
    let at_libcalls = got
        .iter()
        .position(|x| x == &libcalls.display().to_string());
    assert_eq!(at_shim, archive.map(|i| i + 1));
    assert_eq!(at_libcalls, archive.map(|i| i + 2));

    // With no C++ shim it still follows the application archive directly.
    let got = b
        .link_args(&[a.into()], None, Some(&libcalls), elf, map)
        .unwrap();
    let archive = got.iter().position(|x| x == &a.display().to_string());
    let at_libcalls = got
        .iter()
        .position(|x| x == &libcalls.display().to_string());
    assert_eq!(at_libcalls, archive.map(|i| i + 1));
}

/// The libcall crate is built by its own cargo invocation, under a profile whose only
/// job is to turn LTO off: under the workspace's `lto = true` the rlib holds LLVM
/// bitcode, which `ld` cannot read.
#[test]
fn the_libcall_crate_is_built_without_lto() {
    let build = rust();
    let args = build.libcalls().cargo_args();
    assert!(args.contains(&"--profile".to_string()));
    assert!(args.contains(&RustSdk::LIBCALLS_PROFILE.to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(args.contains(&RustSdk::LIBCALLS_CRATE.to_string()));
    assert!(
        args.iter()
            .any(|a| a.ends_with("symbian-libcalls/Cargo.toml"))
    );
    assert!(args.contains(&"-Zbuild-std=core,alloc".to_string()));
}

/// `cargo rustc … -- -Zdefault-visibility=hidden`: the flag goes to the one crate that is
/// linked, and no `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` or config `rustflags` of the
/// developer's can replace it, as they replaced `--config build.rustflags` (experiment 111).
/// `--lib`, because cargo refuses `--` arguments for more than one target, should the crate
/// ever gain another (review 0.2.0, minor 9).
#[test]
fn the_libcall_crate_s_visibility_flag_is_an_argument_of_cargo_rustc() {
    let b = rust();
    let manifest = b.sdk.libcalls_manifest().display().to_string();
    let spec = b.sdk.target_spec().display().to_string();
    assert_eq!(
        b.libcalls().cargo_args(),
        s(&[
            "/rustup/bin/cargo",
            "rustc",
            "--profile",
            "libcalls",
            "-p",
            "symbian-libcalls",
            "--lib",
            "--manifest-path",
            &manifest,
            "--target",
            &spec,
            "-Zbuild-std=core,alloc",
            "-Zbuild-std-features=optimize_for_size",
            "-Zjson-target-spec",
            "--target-dir",
            "build/cargo",
            "--",
            "-Zdefault-visibility=hidden",
        ])
    );
}
