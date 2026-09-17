use std::path::PathBuf;

use symdev_core::{Error, Result};

pub struct Toolchain {
    pub epocroot: PathBuf,
    pub gxx: PathBuf,
    pub ld: PathBuf,
    pub elf2e32: PathBuf,
    pub gcc_lib: PathBuf,
    pub gcc_target_lib: PathBuf,
}

impl Toolchain {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            epocroot: required("SYMDEV_EPOCROOT")?,
            gxx: required("SYMDEV_GXX")?,
            ld: required("SYMDEV_LD")?,
            elf2e32: required("SYMDEV_ELF2E32")?,
            gcc_lib: required("SYMDEV_GCC_LIB")?,
            gcc_target_lib: required("SYMDEV_GCC_TARGET_LIB")?,
        })
    }
}

fn required(key: &str) -> Result<PathBuf> {
    match std::env::var(key) {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => Err(Error::Other(format!("missing toolchain: {key}"))),
    }
}
