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
use crate::ui_resources::UiResources;

/// The C++-mangled `E32Main()` that `eexe.lib`'s startup calls. The reference to it comes
/// from `usrt2_2.lib` in the `-( -)` group *after* the object position, so the Rust
/// archive would not be searched for it; `-u` makes the linker pull the member that
/// defines it (experiment 65a).
pub const E32MAIN: &str = "_Z7E32Mainv";

/// The symbol the Avkon shim imports from the Rust side (`#[symbian_std::main(gui)]`
/// exports it). It needs a `-u` of its own: the shim archive is searched *after* the
/// Rust archive, because that is the direction the `symrs_*` references usually run,
/// and `ld` does not go back. Naming the vtable here makes the Rust archive's member
/// be pulled on the first pass, so the shim's reference to it is already defined by
/// the time the shim archive is reached.
pub const APP_VTBL: &str = "symrs_app_vtbl";

pub struct RustBuild {
    pub gcce: GcceBuild,
    pub sdk: RustSdk,
    /// `SYMDEV_CARGO`, else `cargo` on `PATH` (the rustup proxy, which honours the
    /// project's `rust-toolchain.toml`).
    pub cargo: PathBuf,
    /// The manifest's `package.name`: the Cargo package, the archive `lib<name>.a`, and
    /// the E32 `build/<name>.exe`.
    pub name: String,
    /// `[ui]`: present makes this an Avkon application — the `shims/s60` subclasses,
    /// the five extra import libraries, and the `.rsc`/`_reg.rsc`/`.mif` a captioned
    /// application needs. Absent, nothing of the UI is linked or generated.
    pub ui: Option<UiResources>,
}

impl RustBuild {
    pub fn cargo_from_env() -> PathBuf {
        match std::env::var_os("SYMDEV_CARGO") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from("cargo"),
        }
    }

    /// The one recorded cargo invocation, run in the project root. `-Zbuild-std` builds
    /// `core` and `alloc` for the target; `-Zjson-target-spec` is what this nightly's
    /// cargo demands for a `.json` target; `--target-dir build/cargo` keeps every output
    /// under `build/`. The scaffolded `.cargo/config.toml` repeats these so a hand
    /// `cargo build` matches.
    pub fn cargo_args(&self) -> Vec<String> {
        vec![
            arg(&self.cargo),
            "build".into(),
            "--release".into(),
            "--target".into(),
            arg(&self.sdk.target_spec()),
            "-Zbuild-std=core,alloc".into(),
            "-Zjson-target-spec".into(),
            "--target-dir".into(),
            "build/cargo".into(),
        ]
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

    fn run_cargo(&self, cwd: &RemotePath) -> Result<()> {
        self.run_cargo_args(&self.cargo_args(), cwd)
    }

    fn run_cargo_args(&self, args: &[String], cwd: &RemotePath) -> Result<()> {
        let mut cmd = Command::new(&args[0]);
        cmd.args(&args[1..]);
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
        self.run_cargo(&cwd)?;
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
        Ok(artifacts)
    }
}
