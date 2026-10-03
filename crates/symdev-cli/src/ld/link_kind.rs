//! `LinkKind`: what `symdev-ld` is linking (experiment 114 §1.6).
use symdev_core::{Error, Result};

use super::CargoLinkEnv;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkKind {
    /// The project's `[[bin]]`: `CARGO_BIN_NAME` is the package name.
    Main,
    /// A `tests/<name>.rs` with `harness = false`: no `CARGO_BIN_NAME`, a
    /// `CARGO_TARGET_TMPDIR`. Linked as a console program even in an Avkon project.
    Test { name: String },
}

impl LinkKind {
    pub fn of(env: &CargoLinkEnv, package: &str) -> Result<Self> {
        match (&env.bin_name, &env.target_tmpdir, &env.crate_name) {
            (Some(bin), _, _) if bin == package => Ok(Self::Main),
            (Some(bin), _, _) => Err(Error::Other(format!(
                "symdev-ld: cargo is linking the binary `{bin}`, but symdev.toml's package is \
                 `{package}`: a symdev project has one [[bin]], named after the package; \
                 examples and further binaries are not supported"
            ))),
            (None, Some(_), Some(name)) => Ok(Self::Test { name: name.clone() }),
            _ => Err(Error::Other(
                "symdev-ld: neither CARGO_BIN_NAME (a binary) nor CARGO_TARGET_TMPDIR (a test) \
                 is set; symdev-ld is cargo's linker, run `cargo build`"
                    .into(),
            )),
        }
    }
}
