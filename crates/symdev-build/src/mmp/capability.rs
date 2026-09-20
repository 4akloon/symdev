//! `MmpCapabilities`: the `CAPABILITY` line of one `.mmp`
//! ([mmp-frontend-spec.md](../../../../docs/research/mmp-frontend-spec.md) §10.3).
use symdev_core::{Capabilities, Error, Result};

use crate::Mmp;

/// Names the SDK still recognises and that grant nothing on this release (§10.3).
const OBSOLETE: &[&str] = &[
    "ROOT",
    "MEDIADD",
    "READSYSTEMDATA",
    "WRITESYSTEMDATA",
    "SOUNDDD",
    "UIDD",
    "KILLANYPROCESS",
    "DEVMAN",
    "PHONENETWORK",
    "LOCALNETWORK",
];

/// The capabilities one `.mmp` asks for, canonically spelled and in bit order.
///
/// symdev keeps a single source of truth for them: `symdev.toml`. The E32 image and the
/// SIS controller are built from the manifest list, and this type's job is to make sure
/// the `.mmp` does not quietly ask for something else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MmpCapabilities {
    /// The canonical names, in bit order.
    pub names: Vec<&'static str>,
    pub warnings: Vec<String>,
}

impl MmpCapabilities {
    pub fn of(mmp: &Mmp) -> Result<Self> {
        let mut names: Vec<&'static str> = Vec::new();
        let mut warnings = Vec::new();
        for token in &mmp.capability {
            let (clear, name) = match token.strip_prefix('-') {
                Some(rest) => (true, rest),
                None => (false, token.as_str()),
            };
            if name.eq_ignore_ascii_case("NONE") || name.eq_ignore_ascii_case("ALL") {
                if clear {
                    warnings.push(format!("{}: CAPABILITY {token} is rejected", mmp.target));
                    continue;
                }
                if name.eq_ignore_ascii_case("ALL") {
                    names = Capabilities::all();
                }
                continue;
            }
            if OBSOLETE.iter().any(|o| o.eq_ignore_ascii_case(name)) {
                warnings.push(format!(
                    "{}: CAPABILITY {name} is an old name and grants nothing",
                    mmp.target
                ));
                continue;
            }
            let canonical = Capabilities::canonical(name).ok_or_else(|| {
                Error::Other(format!(
                    "{}: CAPABILITY {name} is not a capability of this release",
                    mmp.target
                ))
            })?;
            names.retain(|n| *n != canonical);
            if !clear {
                names.push(canonical);
            }
        }
        let order = Capabilities::all();
        names.sort_by_key(|n| order.iter().position(|o| o == n).unwrap_or(usize::MAX));
        Ok(Self { names, warnings })
    }

    /// The manifest is what the image and the package are built from, so a `.mmp` that
    /// asks for a different set has to be reconciled by hand rather than silently lose.
    pub fn check(&self, manifest: &[String], target: &str) -> Result<()> {
        if self.names.is_empty() {
            return Ok(());
        }
        let mut granted: Vec<String> = manifest
            .iter()
            .map(|n| Capabilities::canonical(n).unwrap_or(n.as_str()).to_string())
            .collect();
        granted.sort();
        let mut asked: Vec<String> = self.names.iter().map(|n| (*n).to_string()).collect();
        asked.sort();
        if granted == asked {
            return Ok(());
        }
        Err(Error::Other(format!(
            "{target}: CAPABILITY asks for [{}] but symdev.toml grants [{}]; \
             `[symbian] capabilities` is the one list the E32 image and the SIS are both \
             built from, so make them agree",
            asked.join(" "),
            granted.join(" ")
        )))
    }
}
