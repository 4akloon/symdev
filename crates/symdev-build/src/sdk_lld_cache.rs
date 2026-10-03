//! `SdkLldCache`: SDK files fixed for rust-lld, kept on this machine only.
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use symdev_core::{Error, Result};
use symdev_elf2e32::{StrtabPadding, Target2Rewrite};

use crate::sdk_lld_copy::SdkLldCopy;

/// Copies of the SDK files a rust-lld link names, fixed so lld takes them (experiment 109
/// §2): every `.dso` with its string tables' padding zeroed ([`StrtabPadding`]), every
/// `.lib` with its `R_ARM_TARGET2` relocations rewritten ([`Target2Rewrite`]; only
/// `usrt2_2.lib` has one). They are SDK bytes, so they are made here and never shipped.
///
/// **Made at the first link that needs them, not at `symdev sdk install`.** An SDK named by
/// `SYMDEV_EPOCROOT` never passes through `sdk install`, an SDK installed by an older
/// symdev would have no copies, and the directory's name is a hash of the SDK's path, of
/// each file's name and contents and of the fix rules, so a copy can never be stale: a
/// changed SDK file, another SDK or a new rule gives a new directory. Hashing the ~20 files
/// a link names costs a few milliseconds per build.
///
/// A copy is written into a staging directory and renamed into place, so a half-made copy
/// is never seen; when two builds race, the second rename fails and its copy is dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkLldCache {
    root: PathBuf,
}

impl SdkLldCache {
    /// Where the cache lives, relative to `SYMDEV_HOME`.
    pub const DIR: &'static str = "cache/sdk-lld";
    /// Part of every key: change it when a fix rule changes.
    const RULES: &'static str = "symdev sdk-lld 1: dso strtab padding, lib TARGET2";
    /// The SDK directories a Rust link line names, under `epoc32/release/armv5`.
    const DIRS: [&'static str; 2] = ["lib", "urel"];

    pub fn at(root: PathBuf) -> Self {
        Self { root }
    }

    /// The fixed copies of `names` — the `-l:` files of a link line, each found in the
    /// SDK's `epoc32/release/armv5/lib` or `urel` — made now if this SDK has none yet.
    pub fn ensure(&self, epocroot: &Path, names: &[String]) -> Result<SdkLldCopy> {
        let armv5 = epocroot.join("epoc32/release/armv5");
        let mut files = BTreeMap::new();
        for name in names {
            let (dir, path) = Self::DIRS
                .iter()
                .map(|d| (*d, armv5.join(d).join(name)))
                .find(|(_, path)| path.is_file())
                .ok_or_else(|| {
                    Error::Other(format!(
                        "the SDK at {} has no {name} in epoc32/release/armv5/lib or \
                         epoc32/release/armv5/urel, which the rust-lld link names",
                        epocroot.display()
                    ))
                })?;
            let bytes = fs::read(&path).map_err(|e| at(&path, e))?;
            files.insert((dir, name.clone()), (path, bytes));
        }
        let dir = self.root.join(Self::key(epocroot, &files)?);
        if !dir.is_dir() {
            self.make(&dir, &files)?;
        }
        Ok(SdkLldCopy::at(dir))
    }

    fn make(&self, dir: &Path, files: &BTreeMap<(&str, String), (PathBuf, Vec<u8>)>) -> Result<()> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let staging = self
            .root
            .join(format!(".staging-{}-{nanos}", std::process::id()));
        let made = Self::write(&staging, files).and_then(|()| {
            fs::rename(&staging, dir).or_else(|e| match dir.is_dir() {
                true => Ok(()),
                false => Err(at(dir, e)),
            })
        });
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        made
    }

    fn write(staging: &Path, files: &BTreeMap<(&str, String), (PathBuf, Vec<u8>)>) -> Result<()> {
        for d in Self::DIRS {
            fs::create_dir_all(staging.join(d)).map_err(|e| at(staging, e))?;
        }
        for ((d, name), (source, bytes)) in files {
            let fixed = Self::fix(name, bytes)
                .map_err(|e| Error::Other(format!("{}: {e}", source.display())))?;
            let path = staging.join(d).join(name);
            fs::write(&path, fixed).map_err(|e| at(&path, e))?;
        }
        Ok(())
    }

    fn fix(name: &str, bytes: &[u8]) -> Result<Vec<u8>> {
        match Path::new(name).extension().and_then(|e| e.to_str()) {
            Some("dso") => Ok(StrtabPadding::zero(bytes)?.into_bytes()),
            Some("lib") => Ok(Target2Rewrite::archive(bytes)?.into_bytes()),
            _ => Err(Error::Other(format!(
                "TODO: a rust-lld link naming {name}, neither a .dso nor a .lib (not observed)"
            ))),
        }
    }

    /// 32 hex digits of SHA-256 over the rules, the SDK's canonical path, and each file's
    /// place, name and SHA-256.
    fn key(
        epocroot: &Path,
        files: &BTreeMap<(&str, String), (PathBuf, Vec<u8>)>,
    ) -> Result<String> {
        let root = epocroot.canonicalize().map_err(|e| at(epocroot, e))?;
        let mut h = Sha256::new();
        h.update(Self::RULES.as_bytes());
        h.update(b"\n");
        h.update(root.as_os_str().as_encoded_bytes());
        h.update(b"\n");
        for ((d, name), (_, bytes)) in files {
            h.update(format!("{d}/{name} {}\n", hex(&Sha256::digest(bytes))).as_bytes());
        }
        Ok(hex(&h.finalize()[..16]))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn at(path: &Path, e: std::io::Error) -> Error {
    Error::Other(format!("{}: {e}", path.display()))
}

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;
