//! `SdkLldCopy`: one set of SDK files fixed for rust-lld.
use std::path::PathBuf;

/// A directory of [`crate::SdkLldCache`]: `lib/` mirrors the SDK's
/// `epoc32/release/armv5/lib`, `urel/` its `urel`, each holding only the files one kind of
/// link names. A rust-lld line searches these instead of the SDK's own directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SdkLldCopy {
    dir: PathBuf,
}

impl SdkLldCopy {
    pub(crate) fn at(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// The fixed import libraries (`.dso`).
    pub fn lib(&self) -> PathBuf {
        self.dir.join("lib")
    }

    /// The fixed static libraries (`eexe.lib`, `usrt2_2.lib`).
    pub fn urel(&self) -> PathBuf {
        self.dir.join("urel")
    }
}
