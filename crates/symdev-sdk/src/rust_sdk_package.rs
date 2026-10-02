use std::path::{Path, PathBuf};

use crate::{PackageId, Result, SdkError};

/// An installed `rust-sdk;<version>` package: the `symbian-rs` tree a Rust project builds
/// against (its crates, target spec, C++ shims and `std` overlay).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustSdkPackage {
    root: PathBuf,
}

impl RustSdkPackage {
    /// The files a Rust build cannot start without, relative to the package root: the ones
    /// symdev-build's `RustSdk::at` requires (a test there keeps the two lists equal).
    pub const REQUIRED: &'static [&'static str] = &["targets/arm-symbian-e32.json"];

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
    pub fn root(&self) -> &Path {
        &self.root
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
    fn the_package_root_is_the_symbian_rs_tree() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let sdk = RustSdkPackage::at(tmp.path().to_path_buf(), &id("rust-sdk;0.1.0")).unwrap();
        assert_eq!(sdk.root(), tmp.path());
    }

    #[test]
    fn requires_the_target_spec_the_rust_build_reads() {
        assert!(RustSdkPackage::REQUIRED.contains(&"targets/arm-symbian-e32.json"));
    }

    #[test]
    fn a_file_deleted_by_hand_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let spec = tmp.path().join("targets/arm-symbian-e32.json");
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
