//! `RustPrebuilt`: the Rust SDK's C++ shims and GCC runtime, compiled once.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// `symbian-rs/prebuilt/lib` of an installed `rust-sdk` (0.3.0 on): the shims of
/// `shims/common` and `shims/s60` as `libsymrs.a` and `libsymrs_ui.a`, their
/// `R_ARM_TARGET2` already rewritten, and the members of GCCE's `libsupc++.a` and
/// `libgcc.a` they need (experiment 109 §1). With them rust-lld links a Rust program on a
/// machine with no GCCE. A source checkout has no `prebuilt/` (the release recipe makes it,
/// and git ignores it), and there the shims are compiled with GCCE as before; a checkout
/// where someone ran the recipe's `prebuilt.sh` links its output instead, until it is deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustPrebuilt {
    lib: PathBuf,
}

impl RustPrebuilt {
    /// Relative to the SDK root.
    pub const DIR: &'static str = "prebuilt";
    pub const ARCHIVES: [&'static str; 4] =
        ["libsymrs.a", "libsymrs_ui.a", "libsupc++.a", "libgcc.a"];

    /// `None` when `sdk_root` has no `prebuilt/`; with one, every archive of
    /// [`Self::ARCHIVES`] must be in `prebuilt/lib`.
    pub fn in_sdk(sdk_root: &Path) -> Result<Option<Self>> {
        let dir = sdk_root.join(Self::DIR);
        if !dir.exists() {
            return Ok(None);
        }
        let lib = dir.join("lib");
        for archive in Self::ARCHIVES {
            let path = lib.join(archive);
            if !path.is_file() {
                return Err(Error::Other(format!(
                    "Rust SDK at {} has {}/ but no {}: reinstall the rust-sdk package, or \
                     set SYMDEV_RUST_LINKER=gnu to compile the shims with GCCE",
                    sdk_root.display(),
                    Self::DIR,
                    path.display()
                )));
            }
        }
        Ok(Some(Self { lib }))
    }

    /// The directory `-lsupc++ -lgcc` are found in.
    pub fn lib_dir(&self) -> &Path {
        &self.lib
    }

    /// The shim archives a link names, in order: a `[ui]` application's Avkon shim first,
    /// because it refers to the common one and an archive is searched only for what is
    /// undefined where it appears.
    pub fn shims(&self, ui: bool) -> Vec<PathBuf> {
        let ui = ui.then(|| self.lib.join("libsymrs_ui.a"));
        ui.into_iter()
            .chain([self.lib.join("libsymrs.a")])
            .collect()
    }
}

#[cfg(test)]
mod tests;
