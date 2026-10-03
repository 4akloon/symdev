//! `symdev devices`, `symdev emulator start <profile>`, `symdev emulator stop <id>`
//! (design spec §5), and `Devices`, which the runner shares: the registry, the profiles
//! (made from the user's installed firmware when there are none) and their liveness.
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use symdev_core::{Error, Result};
use symdev_emulator::EmulatorData;
use symdev_emulator::control::ControlClient;
use symdev_emulator::device::{
    DeviceId, DeviceRegistry, EmulatorInstance, EmulatorProfile, Firmware, RegistryEntry, is_eka2l1,
};

/// How long a liveness probe waits for `emulator.info`: a wedged emulator must not hang
/// `symdev devices` or a run.
const PROBE: Duration = Duration::from_secs(3);

pub(crate) struct Devices {
    registry: DeviceRegistry,
    profiles_root: PathBuf,
}

impl Devices {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            registry: DeviceRegistry::from_env()?,
            profiles_root: EmulatorProfile::root_from_env()?,
        })
    }

    pub fn registry(&self) -> &DeviceRegistry {
        &self.registry
    }

    pub fn profile(&self, name: &str) -> EmulatorProfile {
        EmulatorProfile::at(&self.profiles_root, name)
    }

    /// The running emulators symdev started: their PID is an EKA2L1 and their socket
    /// answers `emulator.info`. Dead entries are dropped, never signalled.
    pub fn live(&self) -> Result<Vec<RegistryEntry>> {
        self.registry.live(is_eka2l1, |e| {
            ControlClient::connect(&e.socket)
                .and_then(|c| c.with_timeout(PROBE).info())
                .is_ok()
        })
    }

    /// Every emulator symdev started that still runs, answering or not.
    pub fn registered(&self) -> Result<Vec<RegistryEntry>> {
        self.registry.registered(is_eka2l1)
    }

    /// The profiles' names. With none, one is made per firmware in the user's EKA2L1
    /// (`data/roms/<firmware>`), spec §5 phase 1.
    pub fn profiles(&self) -> Result<Vec<String>> {
        let names = Self::dirs(&self.profiles_root);
        if !names.is_empty() {
            return Ok(names);
        }
        let user = match std::env::var_os("SYMDEV_EKA2L1_DATA").filter(|v| !v.is_empty()) {
            Some(dir) => EmulatorData::at(std::path::Path::new(&dir)),
            None => return Ok(Vec::new()),
        };
        for firmware in Firmware::in_user_data(&user) {
            self.profile(firmware.name()).create(&firmware)?;
            eprintln!("created profile {}", firmware.name());
        }
        Ok(Self::dirs(&self.profiles_root))
    }

    fn dirs(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|d| {
                d.flatten()
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

/// `SYMDEV_EKA2L1`, which must have the control server.
pub(crate) fn eka2l1_with_control() -> Result<PathBuf> {
    let eka2l1 = symdev_emulator::Eka2l1Backend::from_env()?.eka2l1;
    if !EmulatorInstance::has_control(&eka2l1)? {
        return Err(Error::Other(format!(
            "SYMDEV_EKA2L1 ({}) has no --control: cargo run needs an EKA2L1 with the control \
             server (EKA2L1#770–#772, our fork's symdev branch)",
            eka2l1.display()
        )));
    }
    Ok(eka2l1)
}

pub(crate) fn list() -> Result<ExitCode> {
    let devices = Devices::from_env()?;
    let profiles = devices.profiles()?;
    let live = devices.live()?;
    for e in devices.registered()? {
        let state = match live.iter().any(|l| l.id == e.id) {
            true => e.name.clone(),
            false => "(not answering)".into(),
        };
        println!("{}  {}  pid {}  profile {}", e.id, state, e.pid, e.profile);
    }
    for p in profiles {
        println!("profile {p}");
    }
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn start(profile: &str) -> Result<ExitCode> {
    let devices = Devices::from_env()?;
    let profiles = devices.profiles()?;
    if !profiles.iter().any(|p| p == profile) {
        return Err(Error::Other(format!(
            "no emulator profile {profile}; there are: {}",
            profiles.join(", ")
        )));
    }
    let eka2l1 = eka2l1_with_control()?;
    let entry = EmulatorInstance::start(&eka2l1, &devices.profile(profile), devices.registry())?;
    println!("{}", entry.id);
    Ok(ExitCode::SUCCESS)
}

pub(crate) fn stop(id: &str) -> Result<ExitCode> {
    let devices = Devices::from_env()?;
    let wanted = DeviceId::parse(id)
        .ok_or_else(|| Error::Other(format!("`{id}` is not a device id like emulator-1")))?;
    let entry = devices
        .registered()?
        .into_iter()
        .find(|e| e.id == wanted)
        .ok_or_else(|| Error::Other(format!("{id} is not running")))?;
    EmulatorInstance::stop(&entry, devices.registry())?;
    Ok(ExitCode::SUCCESS)
}
