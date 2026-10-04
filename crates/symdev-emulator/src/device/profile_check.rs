//! `EmulatorProfile::check`: a profile is whole and its links lead somewhere.
use std::path::Path;

use symdev_core::{Error, Result};

use crate::device::EmulatorProfile;

impl EmulatorProfile {
    /// Refuses, before EKA2L1 starts on it, a profile that is half made (no ROM, no drive
    /// Z) or whose ROM or drive Z is a link to nothing (its firmware package was
    /// uninstalled, or the user's EKA2L1 data moved).
    pub fn check(&self) -> Result<()> {
        let data = self.dir().join("data");
        let z = data.join("drives/z");
        let roms: Vec<_> = std::fs::read_dir(data.join("roms"))
            .map(|d| d.flatten().map(|e| e.path()).collect())
            .unwrap_or_default();
        if roms.is_empty() || z.symlink_metadata().is_err() {
            return Err(Error::Other(format!(
                "emulator profile {} at {} is incomplete: it needs a ROM in data/roms/ and \
                 drive Z in data/drives/z; remove {} and symdev makes the profile again",
                self.name(),
                self.dir().display(),
                self.dir().display()
            )));
        }
        for path in std::iter::once(z).chain(roms) {
            let is_link = path
                .symlink_metadata()
                .is_ok_and(|m| m.file_type().is_symlink());
            if is_link && !path.exists() {
                return Err(self.gone(&path));
            }
        }
        Ok(())
    }

    fn gone(&self, path: &Path) -> Error {
        let target = std::fs::read_link(path).unwrap_or_default();
        let fix = match package_id(&target) {
            Some(id) => format!(
                "Install it again with `symdev sdk install '{id}'`, or remove {} and symdev \
                 makes the profile again",
                self.dir().display()
            ),
            None => format!(
                "Point SYMDEV_EKA2L1_DATA at the EKA2L1 data that has it, or remove {} and \
                 symdev makes the profile again",
                self.dir().display()
            ),
        };
        Error::Other(format!(
            "emulator profile {} links {} to {}, which is gone: its firmware package was \
             uninstalled or its EKA2L1 data moved. {fix}",
            self.name(),
            path.display(),
            target.display()
        ))
    }
}

/// The `firmware;<name>;<version>` id of the package folder a profile's link points into
/// (`<home>/firmware/<name>/<version>/{roms/<name>,drives/z}`).
fn package_id(target: &Path) -> Option<String> {
    let parts: Vec<_> = target
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect();
    let at = parts.iter().rposition(|p| *p == Some("firmware"))?;
    let (name, version) = (parts.get(at + 1)?.as_ref()?, parts.get(at + 2)?.as_ref()?);
    Some(format!("firmware;{name};{version}"))
}
