use std::path::PathBuf;

use crate::{PackageId, Result, SdkError};

/// An installed `gcce;<version>` package: GCC and binutils for `arm-none-symbianelf` in
/// one prefix.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gcce {
    root: PathBuf,
    gcc_version: String,
}

impl Gcce {
    /// The installed prefix of `id` (`gcce;<version>`, where the GCC version is
    /// `<version>` up to its first `-`, so `gcce;12.1.0-2` is a repackaged GCC 12.1.0).
    /// Checks that every path the toolchain uses exists, so a package whose files were
    /// deleted by hand is reported with the command that repairs it.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<Gcce> {
        let version = match id.segments().collect::<Vec<_>>()[..] {
            ["gcce", version] => version,
            _ => {
                return Err(SdkError::Other(format!(
                    "{id} is not a GCCE package id (`gcce;<version>`)"
                )));
            }
        };
        let gcce = Gcce {
            root,
            gcc_version: version.split('-').next().unwrap_or(version).to_string(),
        };
        for path in [gcce.gxx(), gcce.ld(), gcce.gcc_lib(), gcce.gcc_target_lib()] {
            if !path.exists() {
                return Err(SdkError::Other(format!(
                    "{} is missing from installed {id}; run `symdev sdk uninstall {word} && \
                     symdev sdk install {word}`",
                    path.display(),
                    word = id.shell_word()
                )));
            }
        }
        Ok(gcce)
    }

    /// `bin/arm-none-symbianelf-g++`
    pub fn gxx(&self) -> PathBuf {
        self.root.join("bin/arm-none-symbianelf-g++")
    }

    /// `bin/arm-none-symbianelf-ld`; `ar` sits beside it.
    pub fn ld(&self) -> PathBuf {
        self.root.join("bin/arm-none-symbianelf-ld")
    }

    /// `lib/gcc/arm-none-symbianelf/<gcc version>`: `libgcc.a` and friends.
    pub fn gcc_lib(&self) -> PathBuf {
        self.root
            .join("lib/gcc/arm-none-symbianelf")
            .join(&self.gcc_version)
    }

    /// `arm-none-symbianelf/lib`: `libsupc++.a` and the linker scripts.
    pub fn gcc_target_lib(&self) -> PathBuf {
        self.root.join("arm-none-symbianelf/lib")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::Gcce;
    use crate::PackageId;

    /// The files of an installed GCCE prefix, as `~/gcc-builds/gcc-12.1.0` holds them.
    fn prefix(root: &Path, gcc_version: &str) {
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::write(root.join("bin/arm-none-symbianelf-g++"), b"").unwrap();
        fs::write(root.join("bin/arm-none-symbianelf-ld"), b"").unwrap();
        fs::create_dir_all(root.join("lib/gcc/arm-none-symbianelf").join(gcc_version)).unwrap();
        fs::create_dir_all(root.join("arm-none-symbianelf/lib")).unwrap();
    }

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    #[test]
    fn knows_the_files_of_the_prefix() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("gcce/12.1.0");
        prefix(&root, "12.1.0");
        let gcce = Gcce::at(root.clone(), &id("gcce;12.1.0")).unwrap();
        assert_eq!(gcce.gxx(), root.join("bin/arm-none-symbianelf-g++"));
        assert_eq!(gcce.ld(), root.join("bin/arm-none-symbianelf-ld"));
        assert_eq!(
            gcce.gcc_lib(),
            root.join("lib/gcc/arm-none-symbianelf/12.1.0")
        );
        assert_eq!(gcce.gcc_target_lib(), root.join("arm-none-symbianelf/lib"));
    }

    #[test]
    fn a_package_revision_is_not_part_of_the_gcc_version() {
        let tmp = tempfile::tempdir().unwrap();
        prefix(tmp.path(), "12.1.0");
        let gcce = Gcce::at(tmp.path().to_path_buf(), &id("gcce;12.1.0-2")).unwrap();
        assert_eq!(
            gcce.gcc_lib(),
            tmp.path().join("lib/gcc/arm-none-symbianelf/12.1.0")
        );
    }

    #[test]
    fn a_file_deleted_by_hand_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        prefix(tmp.path(), "12.1.0");
        let gxx = tmp.path().join("bin/arm-none-symbianelf-g++");
        fs::remove_file(&gxx).unwrap();
        let e = Gcce::at(tmp.path().to_path_buf(), &id("gcce;12.1.0"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            e,
            format!(
                "{} is missing from installed gcce;12.1.0; run `symdev sdk uninstall 'gcce;12.1.0' \
                 && symdev sdk install 'gcce;12.1.0'`",
                gxx.display()
            )
        );
    }

    #[test]
    fn every_layout_path_is_checked() {
        for missing in [
            "bin/arm-none-symbianelf-ld",
            "lib/gcc/arm-none-symbianelf/12.1.0",
            "arm-none-symbianelf/lib",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            prefix(tmp.path(), "12.1.0");
            let path = tmp.path().join(missing);
            if path.is_dir() {
                fs::remove_dir(&path).unwrap();
            } else {
                fs::remove_file(&path).unwrap();
            }
            let e = Gcce::at(tmp.path().to_path_buf(), &id("gcce;12.1.0")).unwrap_err();
            assert!(
                e.to_string().starts_with(&path.display().to_string()),
                "{e}"
            );
        }
    }

    #[test]
    fn refuses_an_id_that_is_not_gcce() {
        let tmp = tempfile::tempdir().unwrap();
        prefix(tmp.path(), "12.1.0");
        for other in ["sdk;s60-3rd-fp2;1.1", "gcce;12.1.0;x"] {
            let e = Gcce::at(tmp.path().to_path_buf(), &id(other)).unwrap_err();
            assert!(e.to_string().contains(other), "{e}");
        }
    }
}
