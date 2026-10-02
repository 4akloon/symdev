use std::path::PathBuf;

use crate::{PackageId, Result, SdkError};

/// An installed `rust-sdk;<version>` package: the `symbian-rs` tree a Rust project builds
/// against (its crates, target spec, C++ shims and `std` overlay), in the repository's
/// layout. `symbian-rs` is not self-contained: `symbian-macros` depends on
/// `../../../crates/symdev-locale`, which inherits its version and edition from the root
/// `Cargo.toml`, so the package holds those two beside `symbian-rs/` where the relative
/// paths expect them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustSdkPackage {
    root: PathBuf,
}

impl RustSdkPackage {
    /// The directory inside the package that is the Rust SDK.
    pub const SDK_DIR: &'static str = "symbian-rs";

    /// The files a Rust build cannot start without, relative to the package root: the ones
    /// symdev-build's `RustSdk::at` requires (a test there keeps the two lists equal).
    pub const REQUIRED: &'static [&'static str] = &[
        "symbian-rs/targets/arm-symbian-e32.json",
        "crates/symdev-locale/Cargo.toml",
        "Cargo.toml",
    ];

    /// The installed tree of `id` (`rust-sdk;<version>`). Checks [`Self::REQUIRED`], so a
    /// package whose files were deleted by hand is reported with the command that repairs
    /// it rather than as a Rust SDK that is not there.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<RustSdkPackage> {
        if !matches!(id.segments().collect::<Vec<_>>()[..], ["rust-sdk", _]) {
            return Err(SdkError::Other(format!(
                "{id} is not a Rust SDK package id (`rust-sdk;<version>`)"
            )));
        }
        for file in Self::REQUIRED {
            let path = root.join(file);
            if !path.is_file() {
                return Err(SdkError::Other(format!(
                    "{} is missing from installed {id}; run `symdev sdk uninstall {word} && \
                     symdev sdk install {word}`",
                    path.display(),
                    word = id.shell_word()
                )));
            }
        }
        Ok(RustSdkPackage { root })
    }

    /// The `symbian-rs` directory: what `SYMDEV_RUST_SDK` would name.
    pub fn symbian_rs(&self) -> PathBuf {
        self.root.join(Self::SDK_DIR)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::RustSdkPackage;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    /// The files of an installed `rust-sdk` tree that the build checks for.
    fn tree(root: &Path) {
        for file in RustSdkPackage::REQUIRED {
            let path = root.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"{}").unwrap();
        }
    }

    #[test]
    fn the_sdk_is_the_symbian_rs_directory_of_the_package() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let sdk = RustSdkPackage::at(tmp.path().to_path_buf(), &id("rust-sdk;0.1.0")).unwrap();
        assert_eq!(sdk.symbian_rs(), tmp.path().join("symbian-rs"));
    }

    /// `symbian-macros` depends on `../../../crates/symdev-locale`, which inherits its
    /// version and edition from the root `Cargo.toml`: the package keeps the repository's
    /// layout so both resolve.
    #[test]
    fn requires_what_symbian_rs_reaches_outside_itself() {
        for file in [
            "symbian-rs/targets/arm-symbian-e32.json",
            "crates/symdev-locale/Cargo.toml",
            "Cargo.toml",
        ] {
            assert!(RustSdkPackage::REQUIRED.contains(&file), "{file}");
        }
    }

    #[test]
    fn a_file_deleted_by_hand_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let spec = tmp.path().join("symbian-rs/targets/arm-symbian-e32.json");
        fs::remove_file(&spec).unwrap();
        let e = RustSdkPackage::at(tmp.path().to_path_buf(), &id("rust-sdk;0.1.0"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            e,
            format!(
                "{} is missing from installed rust-sdk;0.1.0; run `symdev sdk uninstall \
                 'rust-sdk;0.1.0' && symdev sdk install 'rust-sdk;0.1.0'`",
                spec.display()
            )
        );
    }

    #[test]
    fn refuses_an_id_that_is_not_a_rust_sdk() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        for other in ["gcce;12.1.0", "rust-sdk;0.1.0;x"] {
            let e = RustSdkPackage::at(tmp.path().to_path_buf(), &id(other)).unwrap_err();
            assert!(e.to_string().contains(other), "{e}");
        }
    }
}
