use std::path::{Path, PathBuf};

use crate::{PackageId, Result, SdkError};

/// An installed platform SDK package (`sdk;…`): the package root is the EPOCROOT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformSdk {
    root: PathBuf,
}

impl PlatformSdk {
    /// The installed SDK of `id` (`sdk;…`); checks that `epoc32/include` exists.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<PlatformSdk> {
        if id.kind() != "sdk" {
            return Err(SdkError::Other(format!(
                "{id} is not a platform SDK package id (`sdk;<platform>;<version>`)"
            )));
        }
        let include = root.join("epoc32/include");
        if !include.is_dir() {
            return Err(SdkError::Other(format!(
                "{} is missing from installed {id}; run `symdev sdk uninstall {id} && symdev \
                 sdk install {id}`",
                include.display()
            )));
        }
        Ok(PlatformSdk { root })
    }

    /// The directory that holds `epoc32/`.
    pub fn epocroot(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::PlatformSdk;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    #[test]
    fn the_package_root_is_the_epocroot() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("epoc32/include")).unwrap();
        let sdk = PlatformSdk::at(tmp.path().to_path_buf(), &id("sdk;s60-3rd-fp2;1.1")).unwrap();
        assert_eq!(sdk.epocroot(), tmp.path());
    }

    #[test]
    fn a_missing_include_tree_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        let e = PlatformSdk::at(tmp.path().to_path_buf(), &id("sdk;s60-3rd-fp2;1.1"))
            .unwrap_err()
            .to_string();
        assert!(
            e.starts_with(&tmp.path().join("epoc32/include").display().to_string()),
            "{e}"
        );
        assert!(
            e.contains(
                "symdev sdk uninstall sdk;s60-3rd-fp2;1.1 && symdev sdk install sdk;s60-3rd-fp2;1.1"
            ),
            "{e}"
        );
    }

    #[test]
    fn refuses_an_id_that_is_not_an_sdk() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("epoc32/include")).unwrap();
        let e = PlatformSdk::at(tmp.path().to_path_buf(), &id("gcce;12.1.0")).unwrap_err();
        assert!(e.to_string().contains("gcce;12.1.0"), "{e}");
    }
}
