//! `StdSrc`: the patched `rust-src` a `language = "rust-std"` project is built from.
//!
//! `-Zbuild-std=std` compiles `std` from the source in the toolchain's `rust-src`
//! component. Symbian's platform layer is not in that source, and the component must
//! not be edited in place — it is shared between every project on the host and rustup
//! overwrites it on the next update. So each build materialises a copy:
//!
//! 1. the toolchain's `<sysroot>/lib/rustlib/src/rust/library`, copied whole;
//! 2. `symbian-rs/crates/symbian-sys` copied in as `library/symbian-sys`, because a
//!    crate outside the sysroot build graph gets no `core` and `std` cannot depend on
//!    one that is (see `symbian-rs/rust-src/README.md`);
//! 3. `symbian-rs/rust-src/overlay/library`, copied over the top.
//!
//! Cargo is then pointed at the copy with `__CARGO_TESTS_ONLY_SRC_ROOT`.
//!
//! # Why the overlay files carry a hash
//!
//! Most of the overlay is new files — `sys/pal/symbian/`, `sys/fs/symbian/` and the
//! rest — which no toolchain will ever conflict with. A handful are **replacements**
//! for one of `std`'s own dispatch files, each with one `cfg_select!` arm added. A
//! replacement goes stale silently when the nightly moves, and a stale copy of
//! `sys/sync/mod.rs` would quietly undo whatever upstream changed in it.
//!
//! So `overlay.toml` records the SHA-1 of the toolchain file each replacement was
//! derived from, and materialising checks it. A changed original stops the build and
//! names the file, which is the only way this stays honest across a toolchain bump.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use sha1::{Digest, Sha1};
use symdev_core::{Error, Result};

use crate::rust_sdk::RustSdk;

/// A materialised copy of the standard library source, with the Symbian platform layer
/// patched in.
pub struct StdSrc {
    /// The directory that holds `library/`.
    root: PathBuf,
}

impl StdSrc {
    /// The environment variable cargo reads the `build-std` source root from. It is
    /// cargo's own test hook and the only supported way to build `std` from somewhere
    /// other than the installed component; the toolchain is pinned in
    /// `rust-toolchain.toml`, so the cargo that reads it is pinned too.
    pub const SRC_ROOT_ENV: &'static str = "__CARGO_TESTS_ONLY_SRC_ROOT";

    /// Where the copy goes, under the project's own `build/`.
    ///
    /// `SYMDEV_RUST_STD_SRC` moves it, for a host that would rather keep one copy for
    /// every project than 82 MB in each.
    pub fn dir_for(project_root: &Path) -> PathBuf {
        match std::env::var_os("SYMDEV_RUST_STD_SRC") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => project_root.join("build/rust-src"),
        }
    }

    /// What cargo's `--target-dir` sibling needs: `<root>/library`.
    pub fn src_root(&self) -> PathBuf {
        self.root.join("library")
    }

    /// Builds the copy, replacing whatever was there.
    ///
    /// `rustc` is the compiler whose `rust-src` is copied — the pinned nightly, found
    /// the same way `RustBuild` finds cargo.
    pub fn materialise(sdk: &RustSdk, rustc: &Path, project_root: &Path) -> Result<Self> {
        let library = Self::toolchain_library(rustc, project_root)?;
        let root = Self::dir_for(project_root);
        let this = Self { root };
        if this.root.exists() {
            std::fs::remove_dir_all(&this.root).map_err(io)?;
        }
        copy_tree(&library, &this.src_root())?;
        copy_tree(
            &sdk.crate_dir(RustSdk::SYS_CRATE),
            &this.src_root().join(RustSdk::SYS_CRATE),
        )?;
        let overlay = sdk.std_overlay_dir().join("overlay/library");
        Overlay::read(&sdk.std_overlay_dir().join("overlay.toml"))?.verify(&overlay, &library)?;
        copy_tree(&overlay, &this.src_root())?;
        Ok(this)
    }

    /// `<sysroot>/lib/rustlib/src/rust/library` of the pinned toolchain.
    fn toolchain_library(rustc: &Path, project_root: &Path) -> Result<PathBuf> {
        let mut cmd = Command::new(rustc);
        cmd.arg("--print").arg("sysroot").current_dir(project_root);
        // As `RustBuild::run_cargo_args`: a symdev started through a rustup proxy
        // carries the host toolchain in `RUSTUP_TOOLCHAIN`, which would answer for the
        // wrong nightly.
        cmd.env_remove("RUSTUP_TOOLCHAIN");
        let out = cmd.output().map_err(|e| {
            Error::Other(format!(
                "could not run {} to find the toolchain's rust-src ({e}); set SYMDEV_RUSTC",
                rustc.display()
            ))
        })?;
        if !out.status.success() {
            return Err(Error::Other(format!(
                "{} --print sysroot failed: {}",
                rustc.display(),
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        let sysroot = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
        let library = sysroot.join("lib/rustlib/src/rust/library");
        if !library.join("std/Cargo.toml").is_file() {
            return Err(Error::Other(format!(
                "{} has no std source; run `rustup component add rust-src` for the \
                 toolchain in rust-toolchain.toml",
                library.display()
            )));
        }
        Ok(library)
    }
}

/// The overlay's record of which of `std`'s own files it replaces, and what each one
/// looked like when it was taken.
struct Overlay {
    replaces: BTreeMap<String, String>,
}

impl Overlay {
    fn read(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).map_err(|e| {
            Error::Other(format!(
                "the Rust SDK's std overlay has no {} ({e}); it records the SHA-1 of \
                 every toolchain file the overlay replaces",
                path.display()
            ))
        })?;
        let mut replaces = BTreeMap::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            let Some((name, hash)) = line.split_once('=') else {
                return Err(Error::Other(format!(
                    "{}: `{line}` is not `\"<path>\" = \"<sha1>\"`",
                    path.display()
                )));
            };
            replaces.insert(unquote(name).to_string(), unquote(hash).to_string());
        }
        Ok(Self { replaces })
    }

    /// Every overlay file that replaces a toolchain file must match the hash recorded
    /// for it, and every recorded hash must belong to a file that is still there.
    fn verify(&self, overlay: &Path, library: &Path) -> Result<()> {
        for (name, expected) in &self.replaces {
            if !overlay.join(name).is_file() {
                return Err(Error::Other(format!(
                    "the std overlay records {name} but {} is not there; remove the \
                     line from overlay.toml or restore the file",
                    overlay.join(name).display()
                )));
            }
            let original = library.join(name);
            let actual = sha1_of(&original)?;
            if &actual != expected {
                return Err(Error::Other(format!(
                    "this toolchain's {name} is not the one the Symbian overlay was \
                     taken from (sha1 {actual}, expected {expected}). The overlay \
                     replaces that file whole, so it has to be re-derived: diff \
                     {} against the overlay copy, carry the change across, and record \
                     the new hash in overlay.toml",
                    original.display()
                )));
            }
        }
        Ok(())
    }
}

fn unquote(s: &str) -> &str {
    s.trim().trim_matches('"')
}

fn sha1_of(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).map_err(|e| {
        Error::Other(format!(
            "could not read {} to check it against the std overlay ({e})",
            path.display()
        ))
    })?;
    let mut hasher = Sha1::new();
    hasher.update(&bytes);
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Copies `from` over `to`, creating directories and overwriting files.
///
/// Every file is written afresh rather than hard-linked or `cp -a`-ed, and that is
/// deliberate: cargo decides what to rebuild from modification times, and a copy that
/// kept the source's times would let it call a freshly patched `std` unchanged.
fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(io)?;
    for entry in std::fs::read_dir(from).map_err(io)? {
        let entry = entry.map_err(io)?;
        let target = to.join(entry.file_name());
        if entry.file_type().map_err(io)?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target).map_err(io)?;
        }
    }
    Ok(())
}

fn io(e: std::io::Error) -> Error {
    Error::Other(e.to_string())
}

#[cfg(test)]
mod tests;
