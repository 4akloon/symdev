use std::path::PathBuf;

use symdev_core::{Error, Result};
use symdev_sdk::{Gcce, Pins};

use super::ToolchainOverrides;

/// What a build takes from GCCE: the compiler, the linker (and its archiver), and the two
/// directories the GCC runtime (`-lsupc++ -lgcc`) is found in. A C++ build needs all of
/// them; a Rust build needs them unless rust-lld links it with the Rust SDK's prebuilt
/// shims (experiment 113).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcceTools {
    pub gxx: PathBuf,
    pub ld: PathBuf,
    /// `SYMDEV_AR`; `None` → the `ar` beside `ld` (see [`GcceTools::ar`]).
    pub ar: Option<PathBuf>,
    pub gcc_lib: PathBuf,
    pub gcc_target_lib: PathBuf,
}

impl GcceTools {
    /// Field by field: the override when it is set (an error names the variable if its
    /// path does not exist), else the installed package's file.
    pub fn resolve(o: &ToolchainOverrides, gcce: Option<&Gcce>) -> Result<GcceTools> {
        Ok(GcceTools {
            gxx: Self::field("SYMDEV_GXX", &o.gxx, gcce.map(Gcce::gxx))?,
            ld: Self::field("SYMDEV_LD", &o.ld, gcce.map(Gcce::ld))?,
            ar: ToolchainOverrides::existing("SYMDEV_AR", &o.ar)?,
            gcc_lib: Self::field("SYMDEV_GCC_LIB", &o.gcc_lib, gcce.map(Gcce::gcc_lib))?,
            gcc_target_lib: Self::field(
                "SYMDEV_GCC_TARGET_LIB",
                &o.gcc_target_lib,
                gcce.map(Gcce::gcc_target_lib),
            )?,
        })
    }

    /// One GCCE field: the override, else the package's path.
    fn field(variable: &str, set: &Option<PathBuf>, package: Option<PathBuf>) -> Result<PathBuf> {
        match ToolchainOverrides::existing(variable, set)?.or(package) {
            Some(path) => Ok(path),
            None => Err(Error::Other(format!(
                "{variable} is not set and {gcce} is not installed; run `symdev sdk install \
                 {word}`, or set {variable}",
                gcce = Pins::gcce(),
                word = Pins::gcce().shell_word()
            ))),
        }
    }

    /// The archiver, for the SDK's C++ shim: `SYMDEV_AR`, else the `ar` that sits beside
    /// the linker in the same binutils build (`arm-none-symbianelf-ld` →
    /// `arm-none-symbianelf-ar`).
    ///
    /// Derived rather than required, because the two always ship together and a Rust
    /// build must keep working for an environment that predates the shim.
    pub fn ar(&self) -> Result<PathBuf> {
        if let Some(ar) = &self.ar {
            return Ok(ar.clone());
        }
        let name = self.ld.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match name.strip_suffix("ld") {
            Some(prefix) => Ok(self.ld.with_file_name(format!("{prefix}ar"))),
            None => Err(Error::Other(format!(
                "cannot find the archiver beside the linker ({}): its file name does not \
                 end in `ld`, so set SYMDEV_AR to the matching `ar`",
                self.ld.display()
            ))),
        }
    }
}
