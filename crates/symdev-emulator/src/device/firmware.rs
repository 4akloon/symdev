//! `Firmware`: what an emulator profile is made from (emulator packages spec §5): a
//! firmware installed in the user's EKA2L1 (reached through `SYMDEV_EKA2L1_DATA`), or an
//! installed `firmware;<name>;<n>` package.
use std::path::PathBuf;

use crate::EmulatorData;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Firmware {
    /// `data/roms/<name>` of the user's EKA2L1 data folder; drive Z, drive C,
    /// `devices.yml` and `config.yml` are that folder's.
    UserData { data: EmulatorData, name: String },
    /// An installed package: `roms/<name>/`, `drives/z/<name>/` and `device.yml` under `root`.
    Package { root: PathBuf, name: String },
}

impl Firmware {
    /// The firmware's folder name, which is also its profile's name (`rm-469`).
    pub fn name(&self) -> &str {
        match self {
            Firmware::UserData { name, .. } | Firmware::Package { name, .. } => name,
        }
    }

    /// Every firmware installed in the user's data folder `data` (`data/roms/<name>/`), in
    /// name order; none when the folder has no `data/roms`.
    pub fn in_user_data(data: &EmulatorData) -> Vec<Firmware> {
        let roms = data.root().join("data/roms");
        let mut names: Vec<String> = std::fs::read_dir(&roms)
            .map(|dir| {
                dir.flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
            .into_iter()
            .map(|name| Firmware::UserData {
                data: data.clone(),
                name,
            })
            .collect()
    }
}
