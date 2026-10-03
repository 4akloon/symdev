//! `CargoBuild`: `symdev build` of a Rust project is `cargo build --release` (spec §3), in
//! the project root, with everything else in the project's `.cargo/config.toml`: one build
//! path, the one plain cargo takes.
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use symdev_build::RustBuild;
use symdev_core::{Error, Result};

pub(crate) struct CargoBuild;

impl CargoBuild {
    pub fn args() -> Vec<String> {
        vec!["build".into(), "--release".into()]
    }

    /// `symdev-ld` the way cargo finds the linker the config names: the first `PATH`
    /// directory that has it.
    pub fn linker_on_path(path_var: &OsStr) -> Option<PathBuf> {
        std::env::split_paths(path_var)
            .map(|dir| dir.join("symdev-ld"))
            .find(|p| p.is_file())
    }

    /// Runs cargo in `root`, its output the user's to see. `RUSTUP_TOOLCHAIN` is dropped so
    /// the project's `rust-toolchain.toml` picks the nightly, as `RustBuild` always did.
    pub fn run(root: &Path) -> Result<()> {
        let path = std::env::var_os("PATH").unwrap_or_default();
        if Self::linker_on_path(&path).is_none() {
            return Err(Error::Other(
                "symdev-ld is not on PATH: cargo links through it; run symdev setup-linker \
                 (or install.sh)"
                    .into(),
            ));
        }
        let cargo = RustBuild::cargo_from_env();
        let status = Command::new(&cargo)
            .args(Self::args())
            .current_dir(root)
            .env_remove("RUSTUP_TOOLCHAIN")
            .status()
            .map_err(|e| Error::Other(format!("run {}: {e}", cargo.display())))?;
        if !status.success() {
            return Err(Error::Other(format!(
                "{} build --release failed ({status})",
                cargo.display()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn symdev_build_runs_plain_cargo_build_release() {
        assert_eq!(super::CargoBuild::args(), ["build", "--release"]);
    }

    #[test]
    fn the_linker_is_looked_up_on_path_like_cargo_does() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            super::CargoBuild::linker_on_path(dir.path().as_os_str()),
            None
        );
        std::os::unix::fs::symlink("/bin/true", dir.path().join("symdev-ld")).unwrap();
        let path =
            std::env::join_paths([std::path::Path::new("/nonexistent"), dir.path()]).unwrap();
        assert_eq!(
            super::CargoBuild::linker_on_path(&path),
            Some(dir.path().join("symdev-ld"))
        );
    }
}
