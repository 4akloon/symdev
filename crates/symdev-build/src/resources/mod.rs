//! Resource and build-output helpers: `bld.inf`/MMP loading, install destinations,
//! and the SDK include case-fold overlay.

mod app_target;
mod build_outputs;
mod casefold;
mod generated_casefold;
mod mmp_ext;
mod mmp_path;
mod mmp_resource_ext;
mod project_exports;
mod project_mmps;

pub use app_target::AppTarget;
pub use build_outputs::BuildOutputs;
pub use casefold::SdkIncludeCaseFold;
pub use generated_casefold::GeneratedCaseFold;
pub use mmp_path::MmpPath;
pub(crate) use project_exports::ProjectExports;
pub(crate) use project_mmps::ProjectMmps;

#[cfg(test)]
mod tests;
