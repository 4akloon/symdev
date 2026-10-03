//! `symdev devices`, `symdev emulator start <profile>`, `symdev emulator stop <id>`
//! (design spec §5), and `Devices`, which the runner shares: the registry, the profiles
//! (made from `Provision`'s firmware when a command needs one and there are none) and
//! their liveness.
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use symdev_core::{Error, Result};
use symdev_emulator::control::ControlClient;
use symdev_emulator::device::{
    DeviceId, DeviceRegistry, Eka2l1, EmulatorInstance, EmulatorProfile, Firmware, RegistryEntry,
    is_eka2l1,
};
use symdev_sdk::Pins;

use crate::provision::Provision;

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

    /// The profiles that exist, by name. Makes none.
    pub fn profiles(&self) -> Vec<String> {
        Self::dirs(&self.profiles_root)
    }

    /// A profile per firmware of `firmwares`, made now (spec §5: on first need).
    pub fn make_profiles(&self, firmwares: Vec<Firmware>) -> Result<Vec<String>> {
        for firmware in firmwares {
            self.profile(firmware.name()).create(&firmware)?;
            eprintln!("created profile {}", firmware.name());
        }
        Ok(self.profiles())
    }

    /// The profiles, made from `provision`'s firmware when there are none.
    pub fn profiles_or_make(&self, provision: &Provision) -> Result<Vec<String>> {
        match self.profiles() {
            none if none.is_empty() => self.make_profiles(provision.firmwares()?),
            some => Ok(some),
        }
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

/// The EKA2L1 to start, which must have the control server.
pub(crate) fn eka2l1_with_control(provision: &Provision) -> Result<Eka2l1> {
    let eka2l1 = provision.eka2l1()?;
    if EmulatorInstance::has_control(&eka2l1)? {
        return Ok(eka2l1);
    }
    let fix = match &eka2l1 {
        Eka2l1::User(_) => format!(
            "unset SYMDEV_EKA2L1 to use the {} package, or point it at an EKA2L1 built from \
             our fork's symdev branch",
            Pins::emulator()
        ),
        Eka2l1::Package(_) => format!(
            "the package is damaged: run `symdev sdk uninstall {w} && symdev sdk install {w}`",
            w = Pins::emulator().shell_word()
        ),
    };
    Err(Error::Other(format!(
        "{} has no --control: cargo run needs an EKA2L1 with the control server \
         (EKA2L1#770–#772); {fix}",
        eka2l1.describe()
    )))
}

pub(crate) fn list(provision: &Provision) -> Result<ExitCode> {
    let devices = Devices::from_env()?;
    let profiles = devices.profiles_or_make(provision)?;
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

pub(crate) fn start(profile: &str, provision: &Provision) -> Result<ExitCode> {
    let devices = Devices::from_env()?;
    let profiles = devices.profiles_or_make(provision)?;
    if !profiles.iter().any(|p| p == profile) {
        return Err(Error::Other(format!(
            "no emulator profile {profile}; there are: {}",
            profiles.join(", ")
        )));
    }
    let eka2l1 = eka2l1_with_control(provision)?;
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
