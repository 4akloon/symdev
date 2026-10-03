//! `RustLinker`: which linker links a Rust program (experiment 113).
use std::path::PathBuf;

use symdev_core::{Error, Result};

use crate::{RustPrebuilt, SdkLldCache};

/// How a Rust program is linked. C++ projects are not asked: their line is the SDK's,
/// byte-verified, and they need GCCE to compile anyway.
pub enum RustLinker {
    /// rust-lld in its GNU flavour, the default (experiments 109, 112, 113): two links
    /// around the 8-byte import stubs, the SDK's DSOs and libraries through `cache`, and
    /// with the Rust SDK's prebuilt shims no GCCE at all.
    Lld {
        /// `SYMDEV_RUST_LLD`; unset, the rust-lld of the project's toolchain.
        rust_lld: Option<PathBuf>,
        cache: SdkLldCache,
    },
    /// GCCE's GNU ld 2.29.1 with the shims compiled per application
    /// (`SYMDEV_RUST_LINKER=gnu`): the link every Rust build made before rust-lld, argv for
    /// argv.
    Gnu,
}

impl RustLinker {
    /// The variable that picks GNU ld.
    pub const VARIABLE: &'static str = "SYMDEV_RUST_LINKER";

    /// Whether `value`, the variable's, asks for GNU ld: unset, empty and `lld` mean
    /// rust-lld, `gnu` GNU ld; anything else is refused.
    pub fn wants_gnu(value: Option<&str>) -> Result<bool> {
        match value.unwrap_or("") {
            "" | "lld" => Ok(false),
            "gnu" => Ok(true),
            other => Err(Error::Other(format!(
                "{} is `{other}`; set it to `gnu` for GCCE's GNU ld, or to `lld` (the same as \
                 unset) for rust-lld",
                Self::VARIABLE
            ))),
        }
    }

    /// The prebuilt set this link uses, given the Rust SDK's (`RustSdk::prebuilt`):
    /// rust-lld takes it; GNU ld never does, so its line and its per-application shims
    /// stay as they were.
    pub fn prebuilt<'a>(&self, sdk: Option<&'a RustPrebuilt>) -> Option<&'a RustPrebuilt> {
        match self {
            Self::Lld { .. } => sdk,
            Self::Gnu => None,
        }
    }

    /// Whether the build needs GCCE: unless rust-lld links with the prebuilt set, the
    /// shims are compiled, and GNU ld is GCCE's.
    pub fn needs_gcce(&self, sdk: Option<&RustPrebuilt>) -> bool {
        self.prebuilt(sdk).is_none()
    }
}

#[cfg(test)]
mod tests;
