//! `RustLld`: the rust-lld of the project's Rust toolchain.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// `rust-lld`, which rustup ships in every toolchain's `rustc` component (listed in
/// `lib/rustlib/manifest-rustc-<host>`; experiment 112 §8): no extra component and no
/// GCCE. symdev runs it as `rust-lld -flavor gnu`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustLld {
    path: PathBuf,
}

impl RustLld {
    /// The variable that names a rust-lld and wins over the toolchain's.
    pub const VARIABLE: &'static str = "SYMDEV_RUST_LLD";

    /// `<sysroot>/lib/rustlib/<host>/bin/rust-lld`, from the output of `rustc --print
    /// sysroot` and `rustc -vV` (its `host:` line), run in the project so that its
    /// `rust-toolchain.toml` picks the nightly. Whether the file is there is the caller's
    /// check, which can name the variables that replace it.
    pub fn in_sysroot(sysroot: &str, verbose_version: &str) -> Result<Self> {
        let sysroot = sysroot.trim();
        if sysroot.is_empty() {
            return Err(Error::Other(
                "`rustc --print sysroot` printed no sysroot".into(),
            ));
        }
        let host = verbose_version
            .lines()
            .find_map(|l| l.strip_prefix("host:"))
            .map(str::trim)
            .filter(|h| !h.is_empty())
            .ok_or_else(|| Error::Other("`rustc -vV` printed no `host:` line".into()))?;
        Ok(Self {
            path: Path::new(sysroot)
                .join("lib/rustlib")
                .join(host)
                .join("bin/rust-lld"),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests;
