//! `LinkInputs`: what a Rust program's link takes besides the SDK's own files.
use std::path::{Path, PathBuf};

/// The Rust archive cargo made, the shim archives in link order, and the libcall archive.
pub struct LinkInputs<'a> {
    pub archive: &'a Path,
    pub shims: &'a [PathBuf],
    pub libcalls: &'a Path,
}
