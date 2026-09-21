//! `LibcallArchive`: the SDK's compiler-runtime crate, built and linked as its own
//! archive rather than as a dependency of the application.
use std::path::{Path, PathBuf};

use symdev_core::Project;

use super::arg;
use crate::rust_sdk::RustSdk;

/// `crates/symbian-libcalls` on the link line.
///
/// That crate defines what rustc's own code generation calls and Symbian 9.3 on
/// ARMv5TE does not provide: the `__atomic_*` family over one process-wide
/// `RFastLock`, `__sync_synchronize`, `memcmp` and `bcmp` (experiment 80). The
/// application names none of it.
///
/// **It has to be a separate archive, and that is measured.** Its entry points are
/// `#[unsafe(no_mangle)]`, so they are global symbols, and every global symbol is a
/// `--gc-sections` root in a `-shared` link: as an ordinary dependency of
/// `symbian-runtime` the 32 of them survived into every program and cost `hello` 756
/// bytes for code it never calls (`-Zdefault-visibility=hidden` does not help —
/// `no_mangle` items stay `GLOBAL DEFAULT`). From an archive the member is pulled only
/// by a program that really performs an atomic operation or compares two byte slices,
/// and `hello` is 3 187 bytes again.
pub struct LibcallArchive<'a> {
    cargo: &'a Path,
    sdk: &'a RustSdk,
}

impl<'a> LibcallArchive<'a> {
    pub fn new(cargo: &'a Path, sdk: &'a RustSdk) -> Self {
        Self { cargo, sdk }
    }

    /// The second cargo invocation of a Rust build.
    ///
    /// `--profile libcalls` does two things the workspace's `release` cannot. It turns
    /// LTO **off**, because under `lto = true` an rlib holds LLVM bitcode and `ld`
    /// cannot read it; and it raises `codegen-units`, so the archive has one member per
    /// module. With one member for everything, `examples/files` — which uses `memcmp`
    /// and no atomic — pulled the atomics too and grew 714 bytes.
    pub fn cargo_args(&self) -> Vec<String> {
        vec![
            arg(self.cargo),
            "build".into(),
            "--profile".into(),
            RustSdk::LIBCALLS_PROFILE.into(),
            "-p".into(),
            RustSdk::LIBCALLS_CRATE.into(),
            "--manifest-path".into(),
            arg(&self.sdk.libcalls_manifest()),
            "--target".into(),
            arg(&self.sdk.target_spec()),
            "-Zbuild-std=core,alloc".into(),
            // The same `core` switch the application is built with, so the two
            // halves of one program agree on which `core` they saw.
            "-Zbuild-std-features=optimize_for_size".into(),
            "-Zjson-target-spec".into(),
            "--target-dir".into(),
            "build/cargo".into(),
        ]
    }

    /// Where cargo leaves it, under the project's `build/` like everything else.
    pub fn path(&self, project: &Project) -> PathBuf {
        project
            .root
            .join("build/cargo")
            .join(RustSdk::TARGET)
            .join(RustSdk::LIBCALLS_PROFILE)
            .join(format!(
                "lib{}.rlib",
                RustSdk::LIBCALLS_CRATE.replace('-', "_")
            ))
    }
}
