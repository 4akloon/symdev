//! `CargoOutput`: rustc's `-o` in cargo's build-directory layout (experiment 114 §1.1):
//! `<target-dir>/<triple>/<profile>/build/<package>/<16 hex>/out/<file>`.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CargoOutput {
    path: PathBuf,
    profile_dir: PathBuf,
}

impl CargoOutput {
    pub fn of(path: &Path) -> Result<Self> {
        let up: Vec<&Path> = path.ancestors().take(6).collect();
        let name = |i: usize| {
            up.get(i)
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
        };
        let hash_ok =
            name(2).is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()));
        if name(1) != Some("out") || !hash_ok || name(4) != Some("build") || up.len() < 6 {
            return Err(Error::Other(format!(
                "symdev-ld: rustc's output {} is not in cargo's \
                 <profile>/build/<package>/<hash>/out/ layout; TODO: another cargo layout \
                 (not observed)",
                path.display()
            )));
        }
        Ok(Self {
            path: path.to_path_buf(),
            profile_dir: up[5].to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Where cargo hard-links the main binary, and so where `cargo run`'s path points.
    pub fn profile_dir(&self) -> &Path {
        &self.profile_dir
    }

    pub fn work_dir(&self) -> PathBuf {
        self.with_suffix(".symdev")
    }
    pub fn sisx(&self) -> PathBuf {
        self.with_suffix(".sisx")
    }
    pub fn record(&self) -> PathBuf {
        self.with_suffix(".symdev.toml")
    }

    fn with_suffix(&self, suffix: &str) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(suffix);
        PathBuf::from(s)
    }
}
