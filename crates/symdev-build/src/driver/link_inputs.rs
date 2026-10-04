//! `LinkInputs`: what a Rust program's link takes besides the SDK's own files.
use std::path::{Path, PathBuf};

/// rustc's objects and rlibs (or 0.3.0's one staticlib), the shim archives in link order,
/// and the libcall archive.
pub struct LinkInputs<'a> {
    pub rust: &'a [PathBuf],
    pub shims: &'a [PathBuf],
    pub libcalls: &'a Path,
}
