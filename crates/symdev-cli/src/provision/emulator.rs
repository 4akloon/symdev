//! The EKA2L1 and the firmware an emulator runs (emulator packages spec §5): the user's
//! own through `SYMDEV_EKA2L1` / `SYMDEV_EKA2L1_DATA` first, the pinned packages second.

use symdev_core::Error;
use symdev_emulator::EmulatorData;
use symdev_emulator::device::{Eka2l1, Firmware};
use symdev_manifest::Device;
use symdev_sdk::{EmulatorPackage, FirmwarePackage, PackageId, Pins};

use super::Provision;

const EKA2L1: &str = "SYMDEV_EKA2L1";
const DATA: &str = "SYMDEV_EKA2L1_DATA";

impl Provision {
    /// `SYMDEV_EKA2L1` as it is when set; else the pinned `emulator` package's program,
    /// installed now if missing, by the rules of every package (`--offline`, keyless and
    /// unreadable sources, no source read once it is installed).
    pub fn eka2l1(&self) -> Result<Eka2l1, Error> {
        if let Some(program) = self.var(EKA2L1) {
            return Ok(Eka2l1::User(program));
        }
        let id = Pins::emulator();
        let home = self.install_missing(std::slice::from_ref(&id))?;
        let package = EmulatorPackage::at(home.package_dir(&id), &id)?;
        Ok(Eka2l1::Package(package.program()))
    }

    /// What emulator profiles are made from: with `SYMDEV_EKA2L1_DATA`, every firmware
    /// installed in that EKA2L1 data folder; else the pinned firmware package of every
    /// supported device, installed now when a configured source has it. A failed lookup
    /// names both ways (the catalog adds `SYMDEV_EKA2L1_DATA` for a firmware id).
    pub fn firmwares(&self) -> Result<Vec<Firmware>, Error> {
        let ids: Vec<PackageId> = Device::ALL.into_iter().map(Pins::firmware).collect();
        if let Some(dir) = self.var(DATA) {
            let found = Firmware::in_user_data(&EmulatorData::at(&dir));
            if found.is_empty() {
                let names: Vec<&str> = ids.iter().map(PackageId::as_str).collect();
                return Err(Error::Other(format!(
                    "{DATA} is {}, which has no firmware in data/roms/: install one in EKA2L1, \
                     or unset {DATA} to use the {} package",
                    dir.display(),
                    names.join(" and ")
                )));
            }
            return Ok(found);
        }
        let home = self.install_missing(&ids)?;
        ids.iter()
            .map(|id| {
                let package = FirmwarePackage::at(home.package_dir(id), id)?;
                Ok(Firmware::Package {
                    root: package.root().to_path_buf(),
                    name: package.name().to_string(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
