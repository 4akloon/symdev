//! `RustBuild`: the build backend for `language = "rust"` projects (experiment 65).
//!
//! cargo compiles the project to a static library for `arm-symbian-e32` (rustc never
//! links, design spec §3); the recorded GCCE link line, the native post-linker and the
//! packaging are `GcceBuild`'s, unchanged but for `-u _Z7E32Mainv`.
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_core::{Artifact, BuildBackend, Error, Project, RemotePath, Result};

use super::{CompileIncludes, GcceBuild, LibcallArchive, arg, io};
use crate::required_capability::RequiredCapability;
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

    /// Where the shim object for `source` goes: `build/shims/<stem>.o`, under the
    /// project's `build/` like everything else cargo and the linker produce.
    pub fn shim_object(&self, project: &Project, source: &Path) -> PathBuf {
        let stem = source.file_stem().unwrap_or_default();
        project
            .root
            .join("build/shims")
            .join(Path::new(stem).with_extension("o"))
    }

    /// The shim is compiled with **`GcceBuild`'s own C++ argv**, so it sees `gcce.h`,
    /// the GCC-12 varargs repair, `-D__PRODUCT_INCLUDE__` and every define a C++ project
    /// gets. That is the whole point: the shim is ordinary S60 C++, and the day the
    /// recorded compile line changes the shim moves with it.
    ///
    /// The source directory is the shim directory, so `#include "symrs_shim.h"` finds
    /// its neighbour, and nothing of the user's project is on the include path — the
    /// shim belongs to the SDK.
    pub fn shim_compile_args(&self, source: &Path, obj: &Path) -> Result<Vec<String>> {
        self.gcce.compile_args(
            &self.sdk.shim_dir(),
            &CompileIncludes::default(),
            source,
            obj,
        )
    }

    /// `build/shims/libsymrs.a`: the shim as a static library.
    ///
    /// An **archive**, not a list of objects, and for one measured reason. Objects are
    /// linked whole, so an unused wrapper's reference to its DLL still makes ld record a
    /// `DT_NEEDED` — `--gc-sections` then removes the code but not the dependency,
    /// because as-needed is decided during symbol resolution and garbage collection
    /// happens after it. A `hello` that calls nothing came out at 3219 bytes and loaded
    /// `bafl.dll` for no reason. From an archive a member nobody references is never
    /// pulled and the question does not arise.
    pub fn shim_archive(&self, project: &Project) -> PathBuf {
        project.root.join("build/shims/libsymrs.a")
    }

    /// `ar cr <archive> <objects…>`, the one archiver invocation.
    pub fn ar_args(&self, archive: &Path, objects: &[PathBuf]) -> Result<Vec<String>> {
        let mut args = vec![arg(&self.gcce.tools.ar()?), "cr".into(), arg(archive)];
        args.extend(objects.iter().map(|o| arg(o)));
        Ok(args)
    }

    /// Compiles every SDK shim source into `build/shims/` and archives the objects.
    fn build_shims(&self, project: &Project, cwd: &RemotePath) -> Result<Option<PathBuf>> {
        let sources = self.sdk.shim_sources()?;
        if sources.is_empty() {
            return Ok(None);
        }
        std::fs::create_dir_all(project.root.join("build/shims")).map_err(io)?;
        let mut objects = Vec::new();
        for source in sources {
            let obj = self.shim_object(project, &source);
            self.gcce
                .run_tool(&self.shim_compile_args(&source, &obj)?, cwd)?;
            objects.push(obj);
        }
        let archive = self.shim_archive(project);
        // `ar cr` updates in place, so a stale member from an earlier build would
        // survive a renamed source; the archive is rebuilt from scratch every time.
        if archive.exists() {
            std::fs::remove_file(&archive).map_err(io)?;
        }
        self.gcce
            .run_tool(&self.ar_args(&archive, &objects)?, cwd)?;
        Ok(Some(archive))
    }

    /// `GcceBuild::link_args` with `-u _Z7E32Mainv` after `-u _E32Startup`,
    /// `--gc-sections`, and the two runtime DSOs that define the compiler's helpers
    /// placed *before* the Rust archive.
    ///
    /// Both additions exist to keep a Rust program small, and only the second one gets
    /// at the cause. `compiler_builtins` is built as one codegen unit, so pulling any
    /// member of it pulls all of it: soft-float, 64-bit division, `mem*`, about 0x2b338
    /// bytes of `.text` for a program whose own code is under a kilobyte. What pulls it
    /// is a single reference — `__aeabi_memclr4`, which appears as soon as a function
    /// has a local array (experiment 77 read it out of the link map).
    ///
    /// But the phone already has those routines: `euser.dso` exports `memcpy`, `memset`,
    /// `memmove` and `memclr`, and `drtaeabi.dso` the whole `__aeabi_mem*` family. The
    /// linker was reaching for the archive only because the archive came first. Naming
    /// those two DSOs ahead of it resolves the helpers from ROM, the archive member is
    /// never pulled, and the code is the platform's own rather than a second copy
    /// shipped in every application.
    ///
    /// `--gc-sections` stays: it still drops what a Rust program does not reach, and it
    /// covers helpers the DSOs do not define. Both are added for Rust only; the recorded
    /// C++ link line, byte-verified against the SDK's own, is untouched.
    ///
    /// The shim archive follows the Rust archive, because that is where the undefined
    /// `symrs_*` references come from and an archive is searched only for what is
    /// undefined at the point it appears. The euser and drtaeabi lines stay in front of
    /// the Rust archive. [`RustSdk::LIBRARIES`] adds the DSOs the SDK's own code imports.
    pub fn link_args(
        &self,
        archive: &Path,
        shim: Option<&Path>,
        libcalls: Option<&Path>,
        elf: &Path,
        map: &Path,
    ) -> Vec<String> {
        let mut args = self.gcce.link_args(&self.name, archive, elf, map, &[]);
        let after = args
            .windows(2)
            .position(|w| w[0] == "-u" && w[1] == "_E32Startup")
            .map_or(args.len(), |i| i + 2);
        args.insert(after, E32MAIN.into());
        args.insert(after, "-u".into());
        args.insert(after, "--gc-sections".into());
        let lib = self.gcce.tools.epocroot.join("epoc32/release/armv5/lib");
        let at = args
            .iter()
            .position(|a| a == &arg(archive))
            .unwrap_or(args.len());
        for (i, flag) in [
            format!("-L{}", lib.display()),
            "-l:euser.dso".into(),
            "-l:drtaeabi.dso".into(),
        ]
        .into_iter()
        .enumerate()
        {
            args.insert(at + i, flag);
        }
        // The archives follow the Rust archive in the order their references run:
        // the application refers to the shim, and both may refer to a compiler-runtime
        // routine, so the libcall archive is searched last. An archive is searched only
        // for what is still undefined where it appears.
        let after = args
            .iter()
            .position(|a| a == &arg(archive))
            .map_or(args.len(), |i| i + 1);
        let extras: Vec<String> = [shim, libcalls].into_iter().flatten().map(arg).collect();
        args.splice(after..after, extras);
        let at = args
            .iter()
            .position(|a| a == "-lsupc++")
            .unwrap_or(args.len());
        args.splice(at..at, Self::sdk_libraries());
        args
    }

    /// The DSOs the SDK's own crates and shim import, under `--as-needed` so that an
    /// application which calls neither keeps the dependency list of a pure-euser
    /// program: `--gc-sections` drops the unused shim function (its symbols are hidden,
    /// so they are not collection roots), and `--as-needed` then drops the `DT_NEEDED`
    /// the removed code would have justified. Without it ld 2.29.1 records every `-l`
    /// shared library and a hello would load `bafl.dll` to call nothing.
    fn sdk_libraries() -> Vec<String> {
        let mut out = vec!["--as-needed".to_string()];
        out.extend(RustSdk::LIBRARIES.iter().map(|l| format!("-l:{l}")));
        out.push("--no-as-needed".into());
        out
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
        let archive = self.archive(project);
        if !archive.is_file() {
            return Err(Error::Other(format!(
                "cargo produced no {}: the Cargo package must be named `{}` with \
                 `crate-type = [\"staticlib\"]` (as `symdev new --language rust` writes it)",
                archive.display(),
                self.name
            )));
        }
        let shim = self.build_shims(project, &cwd)?;
        self.run_cargo_args(&self.libcalls().cargo_args(), &cwd)?;
        let libcalls = self.libcalls().path(project);
        if !libcalls.is_file() {
            return Err(Error::Other(format!(
                "cargo produced no {}: the Rust SDK's {} crate is what defines the \
                 __atomic_* family and memcmp for this target",
                libcalls.display(),
                RustSdk::LIBCALLS_CRATE
            )));
        }
        let elf = build_dir.join(format!("{}.elf", self.name));
        let map = build_dir.join(format!("{}.exe.map", self.name));
        self.gcce.run_tool(
            &self.link_args(&archive, shim.as_deref(), Some(&libcalls), &elf, &map),
            &cwd,
        )?;
        // Before the E32 exists, so a missing capability is a build error naming the
        // manifest key rather than a bare -46 on a phone with no console.
        RequiredCapability::check(&elf, &self.gcce.capabilities, &format!("{}.exe", self.name))?;
        let out = build_dir.join(format!("{}.exe", self.name));
        self.gcce
            .run_elf2e32(&self.gcce.elf2e32_args(&self.name, &elf, &out), &cwd)?;
        Ok(vec![Artifact::exe(out)])
    }
}
