//! `RustBuild`: the link line, and the two orderings that decide how big a Rust
//! program is and whether a GUI one links at all.
use std::path::Path;

use super::rust_build::APP_VTBL;
use super::{E32MAIN, RustBuild, arg};
use crate::rust_sdk::RustSdk;

impl RustBuild {
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
    ///
    /// A GUI application changes two things and disturbs nothing else. It names
    /// [`APP_VTBL`] with a `-u` as well, because the reference to it runs the other
    /// way — from the shim archive back into the Rust one, which `ld` has already
    /// passed. And it puts [`RustSdk::UI_LIBRARIES`] on, through the recorded line's
    /// own `libraries` slot, so they sit after the runtime DSOs exactly where an
    /// MMP's `LIBRARY` list would have put them.
    pub fn link_args(
        &self,
        archive: &Path,
        shim: Option<&Path>,
        libcalls: Option<&Path>,
        elf: &Path,
        map: &Path,
    ) -> Vec<String> {
        let ui_libraries: Vec<String> = match self.ui {
            Some(_) => RustSdk::UI_LIBRARIES.iter().map(|l| (*l).into()).collect(),
            None => Vec::new(),
        };
        let mut args = self
            .gcce
            .link_args(&self.name, archive, elf, map, &ui_libraries);
        let after = args
            .windows(2)
            .position(|w| w[0] == "-u" && w[1] == "_E32Startup")
            .map_or(args.len(), |i| i + 2);
        if self.ui.is_some() {
            args.insert(after, APP_VTBL.into());
            args.insert(after, "-u".into());
        }
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
}
