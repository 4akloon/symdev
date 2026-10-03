mod bld;
mod driver;
mod exports;
mod foreign_sdk_paths;
mod icons;
mod mmp;
mod model;
mod package;
mod project;
mod required_capability;
mod resources;
mod rust_linker;
mod rust_lld;
mod rust_prebuilt;
mod rust_sdk;
mod rust_sdk_link;
mod rust_toolchain_file;
mod sdk_lld_cache;
mod sdk_lld_copy;
mod std_src;
mod std_sysroot;
mod strings_resources;
mod toolchain;
mod ui_resources;

pub use bld::ParseError;
pub use driver::{APP_CREATE, E32MAIN, GcceBuild, RustBuild, RustcLink};
pub use driver::{CompileFlags, CompileIncludes, GcceCompat, Module, SourceLanguage};
pub use exports::{DllExports, FrozenExports};
pub use icons::{AppIcon, IconOutputs};
pub use model::{BldExport, BldInf, Mmp, MmpBitmap, MmpBitmapSource, MmpOption, MmpResource};
pub use package::SisPackage;
pub use resources::{AppTarget, BuildOutputs, GeneratedCaseFold, MmpPath, SdkIncludeCaseFold};
pub use rust_linker::RustLinker;
pub use rust_lld::RustLld;
pub use rust_prebuilt::RustPrebuilt;
pub use rust_sdk::RustSdk;
pub use rust_sdk_link::RustSdkLink;
pub use rust_toolchain_file::RustToolchainFile;
pub use sdk_lld_cache::SdkLldCache;
pub use std_src::StdSrc;
pub use std_sysroot::StdSysroot;
pub use strings_resources::StringsResources;
pub use toolchain::{Epocroot, GcceTools, Toolchain, ToolchainOverrides};
pub use ui_resources::UiResources;

/// `error`, prefixed with the file it concerns: every file error of the rust-lld link and
/// its SDK cache reads `<path>: <what>`.
pub(crate) fn file_error(
    path: &std::path::Path,
    error: impl std::fmt::Display,
) -> symdev_core::Error {
    symdev_core::Error::Other(format!("{}: {error}", path.display()))
}
