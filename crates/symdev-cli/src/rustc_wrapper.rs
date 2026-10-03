//! `RustcWrapper`: the `symdev-rustc` role, `[build] rustc` of a `language = "rust-std"`
//! project (experiment 114 §1.4, H4). cargo starts it by the absolute path of the link
//! `<project>/build/symdev-rustc` (observed from the project root and from `src/`), and it
//! runs the pinned rustc with `--sysroot <project>/build/sysroot`, where `StdSysroot`
//! materialised the patched `std` source.
use std::convert::Infallible;
use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_build::{RustBuild, StdSysroot};
use symdev_core::{Error, Result};

pub(crate) struct RustcWrapper {
    sysroot: PathBuf,
    rustc: PathBuf,
}

impl RustcWrapper {
    /// `argv0` is the wrapper's own path: the sysroot is `sysroot/` beside it.
    pub fn of(argv0: &Path, cwd: &Path, rustc: PathBuf) -> Result<Self> {
        let me = cwd.join(argv0);
        let sysroot = me
            .parent()
            .map(|dir| dir.join("sysroot"))
            .ok_or_else(|| Error::Other(format!("{}: no directory", me.display())))?;
        if !sysroot.is_dir() {
            return Err(Error::Other(format!(
                "{}: no {} beside it; `symdev build` materialises the patched std there \
                 (once per SDK and nightly)",
                StdSysroot::WRAPPER,
                sysroot.display()
            )));
        }
        Ok(Self { sysroot, rustc })
    }

    /// Replaces this process with `rustc --sysroot <sysroot> <args…>`.
    pub fn run(argv0: &Path, args: impl Iterator<Item = OsString>) -> Result<Infallible> {
        let cwd = std::env::current_dir()
            .map_err(|e| Error::Other(format!("symdev-rustc: no working directory: {e}")))?;
        let this = Self::of(argv0, &cwd, RustBuild::rustc_from_env())?;
        let e = Command::new(&this.rustc)
            .arg("--sysroot")
            .arg(&this.sysroot)
            .args(args)
            .exec();
        Err(Error::Other(format!(
            "symdev-rustc: could not run {} ({e}); set SYMDEV_RUSTC",
            this.rustc.display()
        )))
    }
}
