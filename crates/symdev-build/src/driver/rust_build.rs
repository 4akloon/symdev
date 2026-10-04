//! `RustBuild`: the link of a `language = "rust"` project (experiment 65), which cargo runs
//! through `symdev-ld` (design spec §4; [`super::RustcLink`]). The link is rust-lld's by
//! default and GCCE's GNU ld on request ([`RustLinker`], experiment 113), both on the
//! recorded GCCE line with `-u _Z7E32Mainv`; the native post-linker and the packaging are
//! `GcceBuild`'s.
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Error, RemotePath, Result};

use super::{GcceBuild, LibcallArchive};
use crate::RustLinker;
use crate::foreign_sdk_paths::ForeignSdkPaths;
use crate::rust_sdk::RustSdk;
use crate::rust_sdk_link::RustSdkLink;
use crate::rust_toolchain_file::RustToolchainFile;
use crate::ui_resources::UiResources;

/// The C++-mangled `E32Main()` that `eexe.lib`'s startup calls. The reference to it comes
/// from `usrt2_2.lib` in the `-( -)` group *after* the object position, so the Rust
/// archive would not be searched for it; `-u` makes the linker pull the member that
/// defines it (experiment 65a).
pub const E32MAIN: &str = "_Z7E32Mainv";

/// The first of the `symrs_app_*` functions the Avkon shim imports from the Rust side
/// (`#[symbian_std::main(gui)]` exports them). It needs a `-u` of its own: the shim
/// archive is searched *after* the Rust archive, because that is the direction the
/// `symrs_*` references usually run, and `ld` does not go back. Naming it here makes
/// the Rust archive's member be pulled on the first pass — the application is one
/// codegen unit, so that member defines all eight — and the shim's references are
/// already defined by the time the shim archive is reached.
pub const APP_CREATE: &str = "symrs_app_create";

pub struct RustBuild {
    pub gcce: GcceBuild,
    pub sdk: RustSdk,
    /// `SYMDEV_CARGO`, else `cargo` on `PATH` (the rustup proxy, which honours the
    /// project's `rust-toolchain.toml`).
    pub cargo: PathBuf,
    /// `SYMDEV_RUSTC`, else `rustc` on `PATH`. Only a `std` build uses it, to find the
    /// `rust-src` component the patched standard library is copied from.
    pub rustc: PathBuf,
    /// The manifest's `package.name`: the Cargo package, its `[[bin]]`, and
    /// the E32 `<name>.exe`.
    pub name: String,
    /// `language = "rust-std"`: build a real `std` for this target from the patched
    /// source (`StdSysroot`) instead of just `core` and `alloc`. Everything after cargo
    /// — the link line, the post-linker, the packaging — is identical.
    pub std: bool,
    /// rust-lld (the default) or GNU ld (`SYMDEV_RUST_LINKER=gnu`), experiment 113.
    pub linker: RustLinker,
    /// `[ui]`: present makes this an Avkon application — the `shims/s60` subclasses,
    /// the five extra import libraries, and the `.rsc`/`_reg.rsc`/`.mif` a captioned
    /// application needs. Absent, nothing of the UI is linked or generated.
    pub ui: Option<UiResources>,
}

impl RustBuild {
    pub fn cargo_from_env() -> PathBuf {
        Self::tool_from_env("SYMDEV_CARGO", "cargo")
    }

    pub fn rustc_from_env() -> PathBuf {
        Self::tool_from_env("SYMDEV_RUSTC", "rustc")
    }

    fn tool_from_env(variable: &str, default: &str) -> PathBuf {
        match std::env::var_os(variable) {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from(default),
        }
    }

    /// The SDK's compiler-runtime archive: the `__atomic_*` family, `__sync_synchronize`,
    /// `memcmp` and `bcmp`, built separately so a program that uses none of them
    /// carries none of them ([`LibcallArchive`]).
    pub fn libcalls(&self) -> LibcallArchive<'_> {
        LibcallArchive::new(&self.cargo, &self.sdk)
    }

    /// What a build checks and sets up before cargo runs, in the project at `root`: its
    /// `rust-toolchain.toml`, if it has one, names the SDK's nightly; nothing in it names
    /// another SDK by absolute path; and `build/rust-sdk` links to this one, the only way
    /// the scaffold names it (experiment 110), unless the project lies inside the SDK's
    /// tree ([`RustSdkLink`]). The checks come first, so a refused project's link is left
    /// as it was, and a project that fails both (a 0.1.0 scaffold) hears of both at once.
    pub fn prepare(&self, root: &Path) -> Result<()> {
        let nightly = match RustToolchainFile::read(root)? {
            Some(own) => own.check_against(&self.sdk.toolchain()?).err(),
            None => None,
        };
        let paths = ForeignSdkPaths::find(root, &self.sdk)?.check().err();
        let refusals: Vec<String> = nightly
            .into_iter()
            .chain(paths)
            .map(|e| e.to_string())
            .collect();
        if !refusals.is_empty() {
            return Err(Error::Other(refusals.join("\n")));
        }
        RustSdkLink::of(root).point_at(&self.sdk)
    }

    /// Runs one cargo invocation (`args[0]` is cargo) in `cwd`: `symdev-ld` runs the
    /// libcall archive's `cargo rustc` this way, nested in cargo's own build (experiment
    /// 114 §1.3).
    pub(super) fn run_cargo_args(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        // A symdev started through a rustup proxy carries the host toolchain in
        // `RUSTUP_TOOLCHAIN`, which would override the project's pinned nightly.
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        let out = self.gcce.env.run_blocking(cmd, cwd)?;
        if out.status != 0 {
            return Err(Error::Other(format!(
                "{} failed (status {}): {}",
                args.join(" "),
                out.status,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(())
    }
}
