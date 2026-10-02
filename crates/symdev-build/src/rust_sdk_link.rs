//! `RustSdkLink`: `build/rust-sdk`, the one way a Rust project names its Rust SDK
//! (experiment 110).
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};
use symdev_sdk::RustSdkPackage;

use crate::rust_sdk::RustSdk;

/// The link `build/rust-sdk` in a project → the directory that holds the SDK the build
/// resolved: the `symbian-rs/` beside `crates/symdev-locale` and the root `Cargo.toml`, as
/// a checkout and the `rust-sdk` package both lay them out.
///
/// The project names the SDK only through it (`build/rust-sdk/symbian-rs/crates/…`,
/// `build/rust-sdk/symbian-rs/targets/…`), so an upgrade, another `SYMDEV_RUST_SDK` or an
/// uninstalled package never leaves a version's path in the project's files.
///
/// It points **above** `symbian-rs` and not at it, which is measured: cargo joins a path
/// dependency's `..` lexically, so `symbian-macros`' `../../../crates/symdev-locale`
/// climbs out of a link to `symbian-rs` itself (`build/crates/symdev-locale`, not found),
/// while through this one it lands on `build/rust-sdk/crates/symdev-locale`.
pub struct RustSdkLink {
    link: PathBuf,
    cargo_dir: PathBuf,
}

impl RustSdkLink {
    /// Where the link lives, relative to the project root.
    pub const PATH: &'static str = "build/rust-sdk";

    /// The link of the project at `root`, whose cargo output is in `build/cargo`.
    pub fn of(root: &Path) -> RustSdkLink {
        RustSdkLink {
            link: root.join(Self::PATH),
            cargo_dir: root.join("build/cargo"),
        }
    }

    /// The SDK crate `name` as the project names it: `build/rust-sdk/symbian-rs/crates/<name>`.
    pub fn crate_dir(name: &str) -> String {
        format!("{}/{}/crates/{name}", Self::PATH, RustSdkPackage::SDK_DIR)
    }

    /// The target spec as the project names it, in `.cargo/config.toml`'s `build.target`.
    /// cargo resolves a relative path there against the directory that holds `.cargo/`,
    /// wherever it is run from (experiment 110 c).
    pub fn target_spec() -> String {
        format!(
            "{}/{}/targets/{}.json",
            Self::PATH,
            RustSdkPackage::SDK_DIR,
            RustSdk::TARGET
        )
    }

    /// Makes the link name `sdk`'s tree, replacing the link in one step.
    ///
    /// When it named another tree, `build/cargo` is removed **first**: cargo would see the
    /// same `build/rust-sdk/…` package paths, compare mtimes, and keep what it built from
    /// the other SDK (experiment 110 e). Removed before the link moves, so a build killed
    /// in between still finds the old link and removes it again.
    pub fn point_at(&self, sdk: &RustSdk) -> Result<()> {
        let tree = Self::tree_of(sdk)?;
        let before = self.current()?;
        if before.as_deref() == Some(tree) {
            return Ok(());
        }
        if before.is_some() && self.cargo_dir.exists() {
            std::fs::remove_dir_all(&self.cargo_dir).map_err(|e| Self::io(&self.cargo_dir, e))?;
        }
        let parent = self.link.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|e| Self::io(parent, e))?;
        let fresh = parent.join(format!("rust-sdk.{}.tmp", std::process::id()));
        // A leftover of a build that was killed here; there is nothing else at this name.
        let _ = std::fs::remove_file(&fresh);
        std::os::unix::fs::symlink(tree, &fresh).map_err(|e| Self::io(&fresh, e))?;
        std::fs::rename(&fresh, &self.link).map_err(|e| Self::io(&self.link, e))
    }

    /// The directory the link must name: the one above `sdk`, which must be `symbian-rs`
    /// for the project's `build/rust-sdk/symbian-rs/…` to reach it.
    fn tree_of(sdk: &RustSdk) -> Result<&Path> {
        let root = sdk.root();
        let named = root
            .file_name()
            .is_some_and(|n| n == RustSdkPackage::SDK_DIR);
        match root.parent() {
            Some(tree) if named => Ok(tree),
            _ => Err(Error::Other(format!(
                "the Rust SDK at {} is not a directory named {sdk_dir}, so a project's \
                 {}/{sdk_dir}/… cannot reach it; use the {sdk_dir} directory of a symdev \
                 checkout or of the rust-sdk package",
                root.display(),
                Self::PATH,
                sdk_dir = RustSdkPackage::SDK_DIR
            ))),
        }
    }

    /// What the link names now; `None` when there is none, an error when something other
    /// than a link is in its place (it is not symdev's to replace).
    fn current(&self) -> Result<Option<PathBuf>> {
        match std::fs::symlink_metadata(&self.link) {
            Ok(meta) if meta.file_type().is_symlink() => std::fs::read_link(&self.link)
                .map(Some)
                .map_err(|e| Self::io(&self.link, e)),
            Ok(_) => Err(Error::Other(format!(
                "{} is not a link: symdev keeps a link there to the Rust SDK it builds \
                 against; move it away and build again",
                self.link.display()
            ))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Self::io(&self.link, e)),
        }
    }

    fn io(path: &Path, e: std::io::Error) -> Error {
        Error::Other(format!("{}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests;
