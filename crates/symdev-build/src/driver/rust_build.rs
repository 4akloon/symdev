//! `RustBuild`: the build backend for `language = "rust"` projects (experiment 65).
//!
//! cargo compiles the project to a static library for `arm-symbian-e32` (rustc never
//! links, design spec §3); the recorded GCCE link line, the native post-linker and the
//! packaging are `GcceBuild`'s, unchanged but for `-u _Z7E32Mainv`.
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Artifact, BuildBackend, Error, Project, RemotePath, Result};

use super::{GcceBuild, arg, io};
use crate::rust_sdk::RustSdk;

/// The C++-mangled `E32Main()` that `eexe.lib`'s startup calls. The reference to it comes
/// from `usrt2_2.lib` in the `-( -)` group *after* the object position, so the Rust
/// archive would not be searched for it; `-u` makes the linker pull the member that
/// defines it (experiment 65a).
pub const E32MAIN: &str = "_Z7E32Mainv";

pub struct RustBuild {
    pub gcce: GcceBuild,
    pub sdk: RustSdk,
    /// `SYMDEV_CARGO`, else `cargo` on `PATH` (the rustup proxy, which honours the
    /// project's `rust-toolchain.toml`).
    pub cargo: PathBuf,
    /// The manifest's `package.name`: the Cargo package, the archive `lib<name>.a`, and
    /// the E32 `build/<name>.exe`.
    pub name: String,
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

    /// Where cargo leaves the static library.
    pub fn archive(&self, project: &Project) -> PathBuf {
        project
            .root
            .join("build/cargo")
            .join(RustSdk::TARGET)
            .join("release")
            .join(format!("lib{}.a", self.name))
    }

    /// `GcceBuild::link_args` with `-u _Z7E32Mainv` right after `-u _E32Startup`, and
    /// `--gc-sections`.
    ///
    /// The garbage collection is what keeps a Rust program small. `compiler_builtins` is
    /// built as **one** codegen unit, so the first reference to any `__aeabi_*` helper —
    /// and `__aeabi_memclr4` appears as soon as a program has a local array — pulls the
    /// whole crate into the link: experiment 68 measured 0x2b338 bytes of `.text` and a
    /// 104 560-byte E32 for an example whose own code is under a kilobyte. rustc gives
    /// every function its own `.text.<symbol>` section, so `--gc-sections` drops what
    /// nothing reaches. It is added for Rust only; the recorded C++ link line, which is
    /// byte-verified against the SDK's own, is untouched.
    pub fn link_args(&self, archive: &Path, elf: &Path, map: &Path) -> Vec<String> {
        let mut args = self.gcce.link_args(&self.name, archive, elf, map, &[]);
        let after = args
            .windows(2)
            .position(|w| w[0] == "-u" && w[1] == "_E32Startup")
            .map_or(args.len(), |i| i + 2);
        args.insert(after, E32MAIN.into());
        args.insert(after, "-u".into());
        args.insert(after, "--gc-sections".into());
        args
    }

    fn run_cargo(&self, cwd: &RemotePath) -> Result<()> {
        let args = self.cargo_args();
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

impl BuildBackend for RustBuild {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>> {
        let build_dir = project.root.join("build");
        std::fs::create_dir_all(&build_dir).map_err(io)?;
        let cwd = RemotePath::new(arg(&project.root));
        self.run_cargo(&cwd)?;
        let archive = self.archive(project);
        if !archive.is_file() {
            return Err(Error::Other(format!(
                "cargo produced no {}: the Cargo package must be named `{}` with \
                 `crate-type = [\"staticlib\"]` (as `symdev new --language rust` writes it)",
                archive.display(),
                self.name
            )));
        }
        let elf = build_dir.join(format!("{}.elf", self.name));
        let map = build_dir.join(format!("{}.exe.map", self.name));
        self.gcce
            .run_tool(&self.link_args(&archive, &elf, &map), &cwd)?;
        let out = build_dir.join(format!("{}.exe", self.name));
        self.gcce
            .run_elf2e32(&self.gcce.elf2e32_args(&self.name, &elf, &out), &cwd)?;
        Ok(vec![Artifact::exe(out)])
    }
}
