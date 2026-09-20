//! `SisTools`: argv for `makesis.exe`/`signsis.exe` under Wine.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

pub struct SisTools {
    pub wine: PathBuf,
    pub makesis: PathBuf,
    pub signsis: PathBuf,
}

impl SisTools {
    pub fn from_env() -> Result<Self> {
        let epocroot = Self::required("SYMDEV_EPOCROOT")?;
        let wine = match std::env::var("SYMDEV_WINE") {
            Ok(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from("/usr/bin/wine"),
        };
        let tools = epocroot.join("epoc32/tools");
        Ok(Self {
            wine,
            makesis: tools.join("makesis.exe"),
            signsis: tools.join("signsis.exe"),
        })
    }

    pub fn makesis_args(&self, pkg: &str, sis: &str) -> Vec<String> {
        vec![
            Self::path_arg(&self.wine),
            Self::path_arg(&self.makesis),
            "-v".into(),
            pkg.into(),
            sis.into(),
        ]
    }

    pub fn signsis_args(
        &self,
        sis: &str,
        sisx: &str,
        cer: &str,
        key: &str,
        password: &str,
    ) -> Vec<String> {
        vec![
            Self::path_arg(&self.wine),
            Self::path_arg(&self.signsis),
            sis.into(),
            sisx.into(),
            cer.into(),
            key.into(),
            password.into(),
        ]
    }

    fn required(key: &str) -> Result<PathBuf> {
        match std::env::var(key) {
            Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
            _ => Err(Error::Other(format!("missing toolchain: {key}"))),
        }
    }

    fn path_arg(path: &Path) -> String {
        path.display().to_string()
    }
}
