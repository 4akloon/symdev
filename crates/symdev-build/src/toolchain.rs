use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

pub struct Toolchain {
    pub epocroot: PathBuf,
    pub gxx: PathBuf,
    pub ld: PathBuf,
    /// External post-linker (`SYMDEV_ELF2E32`). `None` → native `symdev-elf2e32`
    /// runs on the same recorded argv (experiments 45–46).
    pub elf2e32: Option<PathBuf>,
    pub gcc_lib: PathBuf,
    pub gcc_target_lib: PathBuf,
}

/// The SDK root alone (`SYMDEV_EPOCROOT`). Reading a project's `bld.inf` needs it — the
/// preprocessor's include path and the variant header live under it — while packaging
/// needs neither the compiler nor the linker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epocroot(PathBuf);

impl Epocroot {
    pub fn from_env() -> Result<Self> {
        Ok(Self(Toolchain::required("SYMDEV_EPOCROOT")?))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Toolchain {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            epocroot: Self::required("SYMDEV_EPOCROOT")?,
            gxx: Self::required("SYMDEV_GXX")?,
            ld: Self::required("SYMDEV_LD")?,
            elf2e32: Self::optional("SYMDEV_ELF2E32"),
            gcc_lib: Self::required("SYMDEV_GCC_LIB")?,
            gcc_target_lib: Self::required("SYMDEV_GCC_TARGET_LIB")?,
        })
    }

    fn optional(key: &str) -> Option<PathBuf> {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }

    fn required(key: &str) -> Result<PathBuf> {
        match std::env::var(key) {
            Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
            _ => Err(Error::Other(format!("missing toolchain: {key}"))),
        }
    }
}
