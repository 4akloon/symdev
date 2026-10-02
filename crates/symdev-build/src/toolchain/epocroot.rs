use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};
use symdev_sdk::PlatformSdk;

use super::ToolchainOverrides;

/// The SDK root alone (`SYMDEV_EPOCROOT`, else the installed platform SDK). Reading a
/// project's `bld.inf` needs it — the preprocessor's include path and the variant header
/// live under it — while packaging needs neither the compiler nor the linker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Epocroot(pub(super) PathBuf);

impl Epocroot {
    /// `SYMDEV_EPOCROOT` when set, else the installed platform SDK.
    pub fn resolve(o: &ToolchainOverrides, sdk: Option<&PlatformSdk>) -> Result<Epocroot> {
        let set = ToolchainOverrides::existing("SYMDEV_EPOCROOT", &o.epocroot)?;
        match set.or_else(|| sdk.map(|s| s.epocroot().to_path_buf())) {
            Some(root) => Ok(Epocroot(root)),
            None => Err(Error::Other(
                "SYMDEV_EPOCROOT is not set and no platform SDK package is installed; run \
                 `symdev sdk install` in the project, or set SYMDEV_EPOCROOT to your SDK"
                    .into(),
            )),
        }
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}
