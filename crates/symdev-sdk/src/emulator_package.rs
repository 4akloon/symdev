use std::path::PathBuf;

use crate::{PackageId, Result, SdkError};

/// An installed `emulator;<version>` package: the tree of the fork CI's EKA2L1 AppImage,
/// extracted (emulator packages spec §3). symdev starts [`Self::PROGRAM`] itself: `AppRun`
/// is only a link to it, and a process started through the link is named `AppRun`, which
/// the device registry does not take for an EKA2L1 (experiment 115 §1.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmulatorPackage {
    root: PathBuf,
}

impl EmulatorPackage {
    /// The program, relative to the package root. Its `RUNPATH` and `qt.conf` find the
    /// bundled libraries and Qt plugins without any environment.
    pub const PROGRAM: &'static str = "usr/bin/eka2l1_qt";

    /// The installed package of `id` (`emulator;…`); checks that the program is there.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<EmulatorPackage> {
        if id.kind() != "emulator" {
            return Err(SdkError::Other(format!(
                "{id} is not an emulator package id (`emulator;<version>`)"
            )));
        }
        let program = root.join(Self::PROGRAM);
        if !program.is_file() {
            return Err(SdkError::Other(format!(
                "{} is missing from installed {id}; run `symdev sdk uninstall {word} && symdev \
                 sdk install {word}`",
                program.display(),
                word = id.shell_word()
            )));
        }
        Ok(EmulatorPackage { root })
    }

    pub fn program(&self) -> PathBuf {
        self.root.join(Self::PROGRAM)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::EmulatorPackage;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    #[test]
    fn the_program_is_the_binary_apprun_links_to() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("usr/bin")).unwrap();
        fs::write(tmp.path().join("usr/bin/eka2l1_qt"), b"").unwrap();
        let p = EmulatorPackage::at(tmp.path().to_path_buf(), &id("emulator;2026.10.04")).unwrap();
        assert_eq!(p.program(), tmp.path().join("usr/bin/eka2l1_qt"));
    }

    #[test]
    fn a_missing_program_is_named_with_the_reinstall_command() {
        let tmp = tempfile::tempdir().unwrap();
        let e = EmulatorPackage::at(tmp.path().to_path_buf(), &id("emulator;2026.10.04"))
            .unwrap_err()
            .to_string();
        assert!(
            e.starts_with(&tmp.path().join("usr/bin/eka2l1_qt").display().to_string()),
            "{e}"
        );
        assert!(e.contains("symdev sdk uninstall 'emulator;2026.10.04' && symdev sdk install 'emulator;2026.10.04'"), "{e}");
    }

    #[test]
    fn refuses_an_id_that_is_not_an_emulator() {
        let tmp = tempfile::tempdir().unwrap();
        let e = EmulatorPackage::at(tmp.path().to_path_buf(), &id("gcce;12.1.0")).unwrap_err();
        assert!(e.to_string().contains("gcce;12.1.0"), "{e}");
    }
}
