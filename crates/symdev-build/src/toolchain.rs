mod epocroot;
mod overrides;

use std::path::PathBuf;

use symdev_core::{Error, Result};
use symdev_sdk::{Gcce, Pins, PlatformSdk};

pub use epocroot::Epocroot;
pub use overrides::ToolchainOverrides;

pub struct Toolchain {
    pub epocroot: PathBuf,
    pub gxx: PathBuf,
    pub ld: PathBuf,
    /// `SYMDEV_AR`; `None` → the `ar` beside `ld` (see [`Toolchain::ar`]).
    pub ar: Option<PathBuf>,
    /// External post-linker (`SYMDEV_ELF2E32`). `None` → native `symdev-elf2e32`
    /// runs on the same recorded argv (experiments 45–46).
    pub elf2e32: Option<PathBuf>,
    pub gcc_lib: PathBuf,
    pub gcc_target_lib: PathBuf,
}

impl Toolchain {
    /// Field by field: the override when it is set (an error names the variable if its
    /// path does not exist), else the installed package's file.
    pub fn resolve(
        o: &ToolchainOverrides,
        gcce: Option<&Gcce>,
        sdk: Option<&PlatformSdk>,
    ) -> Result<Toolchain> {
        Ok(Toolchain {
            epocroot: Epocroot::resolve(o, sdk)?.0,
            gxx: Self::field("SYMDEV_GXX", &o.gxx, gcce.map(Gcce::gxx))?,
            ld: Self::field("SYMDEV_LD", &o.ld, gcce.map(Gcce::ld))?,
            ar: ToolchainOverrides::existing("SYMDEV_AR", &o.ar)?,
            elf2e32: ToolchainOverrides::existing("SYMDEV_ELF2E32", &o.elf2e32)?,
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

#[cfg(test)]
mod tests;
