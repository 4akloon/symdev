use std::path::{Path, PathBuf};

use crate::{PackageId, Result, SdkError};

/// An installed `firmware;<firmware>;<n>` package (emulator packages spec §4): one
/// firmware in EKA2L1's data layout, `roms/<firmware>/` and `drives/z/<firmware>/`, and
/// that device's entry of EKA2L1's `devices.yml` as `device.yml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FirmwarePackage {
    root: PathBuf,
    name: String,
}

impl FirmwarePackage {
    /// The installed package of `id`; checks the three parts.
    pub fn at(root: PathBuf, id: &PackageId) -> Result<FirmwarePackage> {
        let segments: Vec<&str> = id.segments().collect();
        let ["firmware", name, _] = segments[..] else {
            return Err(SdkError::Other(format!(
                "{id} is not a firmware package id (`firmware;<firmware>;<n>`)"
            )));
        };
        for part in [
            "device.yml".to_string(),
            format!("roms/{name}"),
            format!("drives/z/{name}"),
        ] {
            let path = root.join(&part);
            if !path.exists() {
                return Err(SdkError::Other(format!(
                    "{} is missing from installed {id}; run `symdev sdk uninstall {word} && \
                     symdev sdk install {word}`",
                    path.display(),
                    word = id.shell_word()
                )));
            }
        }
        Ok(FirmwarePackage {
            root,
            name: name.to_string(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The firmware's folder name in EKA2L1's data (`rm-469`).
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::FirmwarePackage;
    use crate::PackageId;

    fn id(s: &str) -> PackageId {
        PackageId::parse(s).unwrap()
    }

    fn tree(root: &std::path::Path) {
        fs::create_dir_all(root.join("roms/rm-469")).unwrap();
        fs::create_dir_all(root.join("drives/z/rm-469")).unwrap();
        fs::write(root.join("device.yml"), "RM-469:\n  firmcode: RM-469\n").unwrap();
    }

    #[test]
    fn the_firmware_is_named_by_the_ids_second_segment() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        let f = FirmwarePackage::at(tmp.path().to_path_buf(), &id("firmware;rm-469;1")).unwrap();
        assert_eq!(f.name(), "rm-469");
        assert_eq!(f.root(), tmp.path());
    }

    #[test]
    fn each_missing_part_is_named_with_the_reinstall_command() {
        for part in ["device.yml", "roms/rm-469", "drives/z/rm-469"] {
            let tmp = tempfile::tempdir().unwrap();
            tree(tmp.path());
            let gone = tmp.path().join(part);
            if gone.is_dir() {
                fs::remove_dir_all(&gone).unwrap()
            } else {
                fs::remove_file(&gone).unwrap()
            }
            let e = FirmwarePackage::at(tmp.path().to_path_buf(), &id("firmware;rm-469;1"))
                .unwrap_err()
                .to_string();
            assert!(e.starts_with(&gone.display().to_string()), "{part}: {e}");
            assert!(
                e.contains("symdev sdk install 'firmware;rm-469;1'"),
                "{part}: {e}"
            );
        }
    }

    #[test]
    fn refuses_an_id_that_is_not_a_firmware_of_three_segments() {
        let tmp = tempfile::tempdir().unwrap();
        tree(tmp.path());
        for bad in ["sdk;s60-3rd-fp2;1.1", "firmware;1", "firmware;rm-469;1;x"] {
            let e = FirmwarePackage::at(tmp.path().to_path_buf(), &id(bad)).unwrap_err();
            assert!(
                e.to_string().contains("firmware;<firmware>;<n>"),
                "{bad}: {e}"
            );
        }
    }
}
