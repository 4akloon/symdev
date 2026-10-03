//! How a Rust project is linked (experiment 113): `SYMDEV_RUST_LINKER`, `SYMDEV_RUST_LLD`.

use symdev_build::{RustLinker, RustLld, SdkLldCache};
use symdev_core::Error;

use super::Provision;

impl Provision {
    /// rust-lld unless `SYMDEV_RUST_LINKER=gnu`; for rust-lld, `SYMDEV_RUST_LLD` if set
    /// (it must exist), and the SDK fix cache under `SYMDEV_HOME`.
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
        if let Some(path) = rust_lld.as_ref().filter(|p| !p.exists()) {
            return Err(Error::Other(format!(
                "{} is set to {}, which does not exist; fix the path or unset {0} to use the \
                 rust-lld of the project's Rust toolchain",
                RustLld::VARIABLE,
                path.display()
            )));
        }
        Ok(RustLinker::Lld {
            rust_lld,
            cache: SdkLldCache::at(self.home_dir()?.join(SdkLldCache::DIR)),
        })
    }
}
