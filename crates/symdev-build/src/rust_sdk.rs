//! `RustSdk`: where the Rust SDK for the phone (`symbian-rs/`) lives on this host.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

/// The `symbian-rs/` workspace: the target JSON, the SDK crates a project depends on by
/// path, and the pinned toolchain. Found through `SYMDEV_RUST_SDK`, else the checkout
/// this `symdev` was built from (a stopgap until an installed SDK layout exists; the
/// scaffold writes the absolute paths into the project, experiment 65).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustSdk {
    root: PathBuf,
}

impl RustSdk {
    /// The one Rust target (design spec §3): `targets/arm-symbian-e32.json`.
    pub const TARGET: &'static str = "arm-symbian-e32";
    /// `symbian-rs/rust-toolchain.toml`, copied verbatim into every scaffolded project so
    /// the project builds with the same pinned nightly as the SDK.
    pub const TOOLCHAIN_FILE: &'static str =
        include_str!("../../../symbian-rs/rust-toolchain.toml");
    /// The hello application (`symbian-rs/examples/hello`), the scaffold's `src/main.rs`.
    pub const HELLO_MAIN: &'static str =
        include_str!("../../../symbian-rs/examples/hello/src/main.rs");
    /// The import libraries the SDK's own crates and C++ shim need beyond the runtime
    /// set of the recorded link line: `efsrv.dso` for `RFs`, `bafl.dso` for the trapped
    /// `BaflUtils::EnsurePathExistsL`.
    ///
    /// They are named for every Rust application, because the SDK and not the project
    /// decides what the shim calls (design spec §7, step 70). An unused one costs
    /// nothing: the post-linker emits an import only for a symbol that is actually
    /// referenced, which is why the recorded C++ line has carried four unused DSOs since
    /// experiment 5 and still produces a 746-byte hello.
    pub const LIBRARIES: &'static [&'static str] = &["efsrv.dso", "bafl.dso"];

    pub fn from_env() -> Result<Self> {
        let root = match std::env::var_os("SYMDEV_RUST_SDK") {
            Some(v) if !v.is_empty() => PathBuf::from(v),
            _ => PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../symbian-rs")),
        };
        Self::at(&root)
    }

    /// `root` must hold the target JSON; the path is canonicalised so the scaffold can
    /// write it into a project anywhere.
    pub fn at(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|e| {
            Error::Other(format!(
                "Rust SDK not found at {} ({e}); set SYMDEV_RUST_SDK to the symbian-rs directory",
                root.display()
            ))
        })?;
        let sdk = Self { root };
        if !sdk.target_spec().is_file() {
            return Err(Error::Other(format!(
                "Rust SDK at {} has no targets/{}.json; set SYMDEV_RUST_SDK to the symbian-rs directory",
                sdk.root.display(),
                Self::TARGET
            )));
        }
        Ok(sdk)
    }

    pub fn target_spec(&self) -> PathBuf {
        self.root
            .join("targets")
            .join(format!("{}.json", Self::TARGET))
    }

    /// `crates/<name>` inside the SDK, for a path dependency.
    pub fn crate_dir(&self, name: &str) -> PathBuf {
        self.root.join("crates").join(name)
    }

    /// `shims/common`: the C++ the SDK compiles into every Rust application so that a
    /// leaving Symbian call is `TRAP`ped before it can reach a Rust frame (design spec
    /// §7, step 70). `shims/s60` is reserved for the Avkon subclasses of step 75, which
    /// need their own libraries and cannot be forced onto a console application.
    pub fn shim_dir(&self) -> PathBuf {
        self.root.join("shims").join("common")
    }

    /// Every `.cpp` under [`Self::shim_dir`], sorted, so the object list and therefore
    /// the link line are the same on every host.
    ///
    /// The application names none of them: the shim is part of the SDK.
    pub fn shim_sources(&self) -> Result<Vec<PathBuf>> {
        let dir = self.shim_dir();
        let mut sources: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map_err(|e| {
                Error::Other(format!(
                    "Rust SDK at {} has no readable {} ({e}); the C++ shim is part of the SDK",
                    self.root.display(),
                    dir.display()
                ))
            })?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "cpp"))
            .collect();
        sources.sort();
        Ok(sources)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkout_sdk_is_found_and_has_the_target() {
        let sdk = RustSdk::from_env().unwrap();
        assert!(sdk.target_spec().ends_with("targets/arm-symbian-e32.json"));
        assert!(
            sdk.crate_dir("symbian-runtime")
                .join("Cargo.toml")
                .is_file()
        );
        assert!(RustSdk::TOOLCHAIN_FILE.contains("channel = \"nightly-"));
        assert!(RustSdk::HELLO_MAIN.contains("symbian_runtime::entry!(main);"));
    }

    #[test]
    fn missing_sdk_names_the_env_var() {
        let err = RustSdk::at(Path::new("/nonexistent/symbian-rs")).unwrap_err();
        assert!(err.to_string().contains("SYMDEV_RUST_SDK"), "{err}");
    }
}
