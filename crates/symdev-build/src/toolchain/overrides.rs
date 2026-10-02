use std::ffi::OsString;
use std::path::PathBuf;

use symdev_core::{Error, Result};

/// The `SYMDEV_*` toolchain variables that are set. Each one wins over the installed
/// packages, field by field (spec §3), so every existing environment keeps working; the
/// packages fill whatever is left.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ToolchainOverrides {
    /// `SYMDEV_EPOCROOT`
    pub epocroot: Option<PathBuf>,
    /// `SYMDEV_GXX`
    pub gxx: Option<PathBuf>,
    /// `SYMDEV_LD`
    pub ld: Option<PathBuf>,
    /// `SYMDEV_AR`; unset, the `ar` beside the linker.
    pub ar: Option<PathBuf>,
    /// `SYMDEV_ELF2E32`; unset, the native post-linker.
    pub elf2e32: Option<PathBuf>,
    /// `SYMDEV_GCC_LIB`
    pub gcc_lib: Option<PathBuf>,
    /// `SYMDEV_GCC_TARGET_LIB`
    pub gcc_target_lib: Option<PathBuf>,
}

impl ToolchainOverrides {
    /// The variables of this process's environment.
    pub fn from_env() -> ToolchainOverrides {
        Self::from_lookup(|key| std::env::var_os(key))
    }

    /// The variables as `get` reports them; an empty value counts as unset.
    pub(crate) fn from_lookup(get: impl Fn(&str) -> Option<OsString>) -> ToolchainOverrides {
        let var = |key: &str| get(key).filter(|v| !v.is_empty()).map(PathBuf::from);
        ToolchainOverrides {
            epocroot: var("SYMDEV_EPOCROOT"),
            gxx: var("SYMDEV_GXX"),
            ld: var("SYMDEV_LD"),
            ar: var("SYMDEV_AR"),
            elf2e32: var("SYMDEV_ELF2E32"),
            gcc_lib: var("SYMDEV_GCC_LIB"),
            gcc_target_lib: var("SYMDEV_GCC_TARGET_LIB"),
        }
    }

    /// Whether any field the GCCE package provides is left to it.
    pub fn needs_gcce(&self) -> bool {
        [&self.gxx, &self.ld, &self.gcc_lib, &self.gcc_target_lib]
            .iter()
            .any(|field| field.is_none())
    }

    /// Whether the EPOCROOT is left to the platform SDK package.
    pub fn needs_sdk(&self) -> bool {
        self.epocroot.is_none()
    }

    /// `set` when the variable is set and its path exists; `None` when it is unset.
    pub(crate) fn existing(variable: &str, set: &Option<PathBuf>) -> Result<Option<PathBuf>> {
        match set {
            Some(path) if !path.exists() => Err(Error::Other(format!(
                "{variable} is set to {}, which does not exist; fix the path or unset \
                 {variable}",
                path.display()
            ))),
            other => Ok(other.clone()),
        }
    }
}
