//! The Rust SDK a Rust project builds against (spec §12).

use symdev_build::RustSdk;
use symdev_core::Error;
use symdev_sdk::{PackageId, Pins, RustSdkPackage};

use super::Provision;

/// The variable that names a Rust SDK and wins over the checkout and the package.
const VARIABLE: &str = "SYMDEV_RUST_SDK";

impl Provision {
    /// `SYMDEV_RUST_SDK` when it is set (and then it must be a Rust SDK); else the checkout
    /// symdev was built from, while it is still a Rust SDK, so a developer keeps using
    /// their tree; else the installed `rust-sdk` package of this release, installed now if
    /// it is missing — the same rules as GCCE's: `--offline`, keyless and unreadable
    /// sources, and no source read while it is installed.
    pub fn rust_sdk(&self) -> Result<RustSdk, Error> {
        if let Some(sdk) = self.local_rust_sdk()? {
            return Ok(sdk);
        }
        let id = Pins::rust_sdk();
        let home = self
            .install_missing(std::slice::from_ref(&id))
            .map_err(|e| {
                Error::Other(format!("{e}; or set {VARIABLE} to a symbian-rs directory"))
            })?;
        let package = RustSdkPackage::at(home.package_dir(&id), &id)?;
        RustSdk::at(&package.symbian_rs())
    }

    /// The `rust-sdk` package, when a Rust project would take its SDK from it: neither
    /// `SYMDEV_RUST_SDK` nor the checkout provides one.
    pub(super) fn needed_rust_sdk(&self) -> Option<PackageId> {
        let set = self.var(VARIABLE).is_some();
        (!set && self.checkout_sdk().is_none()).then(Pins::rust_sdk)
    }

    /// The SDK that `SYMDEV_RUST_SDK` or the checkout names, if either does.
    fn local_rust_sdk(&self) -> Result<Option<RustSdk>, Error> {
        match self.var(VARIABLE) {
            Some(set) => RustSdk::at(&set).map(Some).map_err(|e| {
                Error::Other(format!(
                    "{VARIABLE} is set, but {e}; point it at a symbian-rs directory, or unset \
                     it to use the {} package",
                    Pins::rust_sdk()
                ))
            }),
            None => Ok(self.checkout_sdk()),
        }
    }

    /// The checkout symdev was built from, if it is still there and still a Rust SDK.
    fn checkout_sdk(&self) -> Option<RustSdk> {
        self.checkout.as_deref().and_then(|c| RustSdk::at(c).ok())
    }
}

#[cfg(test)]
mod tests;
