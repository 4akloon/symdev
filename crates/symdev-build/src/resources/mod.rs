//! Resource and build-output helpers: `bld.inf`/MMP loading, install destinations,
//! and the SDK include case-fold overlay.
use std::path::Path;

use symdev_core::{Error, Result};

mod app_target;
mod build_outputs;
mod casefold;
mod mmp_ext;
mod mmp_resource_ext;
mod project_mmps;

pub use app_target::AppTarget;
pub use build_outputs::BuildOutputs;
pub use casefold::SdkIncludeCaseFold;
pub(crate) use project_mmps::ProjectMmps;

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| Error::Other(format!("read {path:?}: {e}")))
}

#[cfg(test)]
mod tests;
