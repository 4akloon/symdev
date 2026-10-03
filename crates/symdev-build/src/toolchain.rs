mod epocroot;
mod gcce_tools;
mod overrides;

use std::path::PathBuf;

use symdev_core::{Error, Result};
use symdev_sdk::{Gcce, PlatformSdk};

pub use epocroot::Epocroot;
pub use gcce_tools::GcceTools;
pub use overrides::ToolchainOverrides;

pub struct Toolchain {
    pub epocroot: PathBuf,
    /// External post-linker (`SYMDEV_ELF2E32`). `None` → native `symdev-elf2e32`
    /// runs on the same recorded argv (experiments 45–46).
    pub elf2e32: Option<PathBuf>,
    /// `None` for a build that uses no GCCE: a Rust build linked by rust-lld with the
    /// Rust SDK's prebuilt shims (experiment 113). Every other build has it.
    pub gcce: Option<GcceTools>,
}

impl Toolchain {
    /// A toolchain with GCCE, field by field: the override when it is set (an error names
    /// the variable if its path does not exist), else the installed package's file.
    pub fn resolve(
        o: &ToolchainOverrides,
        gcce: Option<&Gcce>,
        sdk: Option<&PlatformSdk>,
    ) -> Result<Toolchain> {
        let base = Self::without_gcce(o, sdk)?;
        Ok(Toolchain {
            gcce: Some(GcceTools::resolve(o, gcce)?),
            ..base
        })
    }

    /// A toolchain with no GCCE: the SDK root and the post-linker only.
    pub fn without_gcce(o: &ToolchainOverrides, sdk: Option<&PlatformSdk>) -> Result<Toolchain> {
        Ok(Toolchain {
            epocroot: Epocroot::resolve(o, sdk)?.0,
            elf2e32: ToolchainOverrides::existing("SYMDEV_ELF2E32", &o.elf2e32)?,
            gcce: None,
        })
    }

    /// GCCE, for a build that compiles C++ or links with GNU ld. A toolchain set up without
    /// it reaching this is symdev's error, not the environment's: only a rust-lld build with
    /// the prebuilt set is set up so, and such a build compiles and links no GCCE.
    pub fn gcce(&self) -> Result<&GcceTools> {
        self.gcce.as_ref().ok_or_else(|| {
            Error::Other(
                "internal error: a step needs GCCE, and this build was set up without GCCE (a \
                 Rust build that rust-lld links with the Rust SDK's prebuilt shims); set \
                 SYMDEV_RUST_LINKER=gnu to build with GCCE, and report it"
                    .into(),
            )
        })
    }
}

#[cfg(test)]
mod tests;
