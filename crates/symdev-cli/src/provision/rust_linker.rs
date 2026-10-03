//! How a Rust project is linked (experiment 113): `SYMDEV_RUST_LINKER`, `SYMDEV_RUST_LLD`.

use symdev_build::{RustLinker, RustLld, RustPrebuilt, RustSdk, SdkLldCache};
use symdev_core::Error;

use super::Provision;

impl Provision {
    /// rust-lld unless `SYMDEV_RUST_LINKER=gnu`; for rust-lld, `SYMDEV_RUST_LLD` if set
    /// (it must be a file), and the SDK fix cache under `SYMDEV_HOME`.
    pub fn rust_linker(&self) -> Result<RustLinker, Error> {
        let value = (self.lookup)(RustLinker::VARIABLE);
        let value = match &value {
            Some(v) => Some(v.to_str().ok_or_else(|| {
                Error::Other(format!("{} is not valid UTF-8", RustLinker::VARIABLE))
            })?),
            None => None,
        };
        if RustLinker::wants_gnu(value)? {
            return Ok(RustLinker::Gnu);
        }
        let rust_lld = self.var(RustLld::VARIABLE);
        if let Some(path) = rust_lld.as_ref().filter(|p| !p.is_file()) {
            return Err(Error::Other(format!(
                "{} is set to {}, which does not exist; fix the path or unset {0} to use the \
                 rust-lld of the project's Rust toolchain",
                RustLld::VARIABLE,
                path.display()
            )));
        }
        // The fix cache is the one thing rust-lld needs a home for, even when every
        // toolchain path is set and no package is.
        let home = self.home_dir().map_err(|e| {
            Error::Other(format!(
                "rust-lld links with fixed copies of SDK files that symdev keeps in \
                 <SYMDEV_HOME>/{}, and {e}; or set SYMDEV_RUST_LINKER=gnu to link with \
                 GCCE's GNU ld",
                SdkLldCache::DIR
            ))
        })?;
        Ok(RustLinker::Lld {
            rust_lld,
            cache: SdkLldCache::at(home.join(SdkLldCache::DIR)),
        })
    }

    /// A one-line note when rust-lld links the prebuilt set of a local Rust SDK tree (the
    /// checkout, or `SYMDEV_RUST_SDK`): that set, not the tree's `shims/` sources, goes into
    /// the program, so an edited shim changes nothing until `prebuilt/` is deleted. The
    /// installed package's set is the release's own and needs no note. Modification times
    /// are not compared: a copied or checked-out tree makes them say nothing.
    pub fn prebuilt_note(
        &self,
        sdk: &RustSdk,
        linker: &RustLinker,
    ) -> Result<Option<String>, Error> {
        if self.needed_rust_sdk().is_some() {
            return Ok(None);
        }
        Ok(linker.prebuilt(sdk)?.map(|p| {
            format!(
                "note: linking the prebuilt shims in {}, not {}/shims/; delete {} to link \
                 edited shims",
                p.lib_dir().display(),
                sdk.root().display(),
                sdk.root().join(RustPrebuilt::DIR).display()
            )
        }))
    }
}
