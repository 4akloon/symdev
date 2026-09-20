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

    /// The archiver, for the SDK's C++ shim: `SYMDEV_AR`, else the `ar` that sits beside
    /// `SYMDEV_LD` in the same binutils build (`arm-none-symbianelf-ld` →
    /// `arm-none-symbianelf-ar`).
    ///
    /// Derived rather than required, because the two always ship together and a Rust
    /// build must keep working for an environment that predates the shim.
    pub fn ar(&self) -> Result<PathBuf> {
        if let Some(ar) = Self::optional("SYMDEV_AR") {
            return Ok(ar);
        }
        let name = self.ld.file_name().and_then(|n| n.to_str()).unwrap_or("");
        match name.strip_suffix("ld") {
            Some(prefix) => Ok(self.ld.with_file_name(format!("{prefix}ar"))),
            None => Err(Error::Other(format!(
                "cannot find the archiver beside SYMDEV_LD ({}): its file name does not \
                 end in `ld`, so set SYMDEV_AR to the matching `ar`",
                self.ld.display()
            ))),
        }
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
