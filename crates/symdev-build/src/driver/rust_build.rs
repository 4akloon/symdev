//! `RustBuild`: the build backend for `language = "rust"` projects (experiment 65).
//!
//! cargo compiles the project to a static library for `arm-symbian-e32` (rustc never
//! links, design spec §3); the recorded GCCE link line, the native post-linker and the
//! packaging are `GcceBuild`'s, unchanged but for `-u _Z7E32Mainv`.
use std::path::PathBuf;
use std::process::Command;

use symdev_core::{Artifact, BuildBackend, Error, Project, RemotePath, Result};

use super::{GcceBuild, LibcallArchive, arg, io, produced};
use crate::required_capability::RequiredCapability;
use crate::rust_sdk::RustSdk;
use crate::std_src::StdSrc;
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
    /// The manifest's `package.name`: the Cargo package, the archive `lib<name>.a`, and
    /// the E32 `build/<name>.exe`.
    pub name: String,
    /// `language = "rust-std"`: build a real `std` for this target from the patched
    /// source ([`StdSrc`]) instead of just `core` and `alloc`. Everything after cargo
    /// — the link line, the post-linker, the packaging — is identical.
    pub std: bool,
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

    /// Which crates `-Zbuild-std` builds.
    ///
    /// `core,alloc` for a `#![no_std]` application, `std,panic_abort` for one with a
    /// real `std` — `panic_abort` is named explicitly because the target's
    /// `panic-strategy` is `abort` and `std` would otherwise ask for the unwinding
    /// runtime, which this link line has no unwinder for.
    fn build_std(&self) -> &'static str {
        if self.std {
            "-Zbuild-std=std,panic_abort"
        } else {
            "-Zbuild-std=core,alloc"
        }
    }

    /// `core`'s own size/speed switch, on for a `#![no_std]` application.
    ///
    /// It picks the small algorithm wherever `core` keeps two: integer `Display`
    /// without the 200-byte two-digit lookup table, the small sort, the short
    /// `str` padding path. Measured on the sixteen `no_std` examples it takes
    /// 600–1 450 bytes of `.text` and exactly 200 bytes of `.rodata` off each one that
    /// formats anything, and grows none of them; a 369 MHz ARM9 has bytes to spare
    /// less than it has cycles, and nothing observable changes.
    ///
    /// Only the `no_std` path names it. Naming `-Zbuild-std-features` at all replaces
    /// cargo's default set, which for a real `std` is `panic-unwind`, and the `std`
    /// examples are not what this was measured on.
    const BUILD_STD_FEATURES: &'static str = "-Zbuild-std-features=optimize_for_size";

    /// The one recorded cargo invocation, run in the project root. `-Zbuild-std` builds
    /// `core` and `alloc` for the target; `-Zjson-target-spec` is what this nightly's
    /// cargo demands for a `.json` target; `--target-dir build/cargo` keeps every output
    /// under `build/`. The scaffolded `.cargo/config.toml` repeats these so a hand
    /// `cargo build` matches.
    pub fn cargo_args(&self) -> Vec<String> {
        let mut args = vec![
            arg(&self.cargo),
            "build".into(),
            "--release".into(),
            "--target".into(),
            arg(&self.sdk.target_spec()),
            self.build_std().into(),
        ];
        if !self.std {
            args.push(Self::BUILD_STD_FEATURES.into());
        }
        args.extend([
            "-Zjson-target-spec".into(),
            "--target-dir".into(),
            "build/cargo".into(),
        ]);
        args
    }

    /// The SDK's compiler-runtime archive: the `__atomic_*` family, `__sync_synchronize`,
    /// `memcmp` and `bcmp`, built separately so a program that uses none of them
    /// carries none of them ([`LibcallArchive`]).
    pub fn libcalls(&self) -> LibcallArchive<'_> {
        LibcallArchive::new(&self.cargo, &self.sdk)
    }

    /// Where cargo leaves the static library.
    pub fn archive(&self, project: &Project) -> PathBuf {
        project
            .root
            .join("build/cargo")
            .join(RustSdk::TARGET)
            .join("release")
            .join(format!("lib{}.a", self.name))
    }

    fn run_cargo(&self, project: &Project, cwd: &RemotePath) -> Result<()> {
        let src = match self.std {
            true => Some(StdSrc::materialise(&self.sdk, &self.rustc, &project.root)?),
            false => None,
        };
        self.run_cargo_in(&self.cargo_args(), cwd, src.as_ref())
    }

    fn run_cargo_args(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        self.run_cargo_in(args, cwd, None)
    }

    fn run_cargo_in(&self, args: &[String], cwd: &RemotePath, src: Option<&StdSrc>) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
        if let Some(src) = src {
            cmd.env(StdSrc::SRC_ROOT_ENV, src.src_root());
        }
        // A symdev started through a rustup proxy carries the host toolchain in
        // `RUSTUP_TOOLCHAIN`, which would override the project's pinned nightly.
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        // The application's own UID3, so a crate can name a per-application path
        // without the manifest value being written a second time in the source, where
        // the two then drift apart (it cost a confused emulator run to find that out).
        cmd.env("SYMDEV_UID3", format!("0x{:08x}", self.gcce.uid3));
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

impl BuildBackend for RustBuild {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>> {
        let build_dir = project.root.join("build");
        std::fs::create_dir_all(&build_dir).map_err(io)?;
        let cwd = RemotePath::new(arg(&project.root));
        self.run_cargo(project, &cwd)?;
        let archive = produced(
            self.archive(project),
            &format!(
                "the Cargo package must be named `{}` with `crate-type = [\"staticlib\"]`, \
                 as `symdev new --language rust` writes it",
                self.name
            ),
        )?;
        let shim = self.build_shims(project, &cwd)?;
        self.run_cargo_args(&self.libcalls().cargo_args(), &cwd)?;
        let libcalls = produced(
            self.libcalls().path(project),
            &format!(
                "the Rust SDK's {} crate is what defines the __atomic_* family and memcmp \
                 for this target",
                RustSdk::LIBCALLS_CRATE
            ),
        )?;
        let elf = build_dir.join(format!("{}.elf", self.name));
        let map = build_dir.join(format!("{}.exe.map", self.name));
        self.gcce.run_tool(
            &self.link_args(&archive, shim.as_deref(), Some(&libcalls), &elf, &map),
            &cwd,
        )?;
        RequiredCapability::check(&elf, &self.gcce.capabilities, &format!("{}.exe", self.name))?;
        let out = build_dir.join(format!("{}.exe", self.name));
        self.gcce
            .run_elf2e32(&self.gcce.elf2e32_args(&self.name, &elf, &out), &cwd)?;
        let mut artifacts = vec![Artifact::exe(out)];
        artifacts.extend(self.build_ui(&build_dir)?);
        artifacts.extend(self.build_strings(&project.root, &build_dir)?);
        Ok(artifacts)
    }
}
