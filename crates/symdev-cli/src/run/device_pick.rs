//! `pick_device`: the device of spec §5's choice, started when it is a profile.
use std::io::{BufRead, Write};

use symdev_core::{Error, Result};
use symdev_emulator::device::{
    Choice, DeviceChoice, DeviceId, DevicePrompt, EmulatorInstance, Offer, RegistryEntry,
};

use crate::devices_cmd::{Devices, eka2l1_with_control};
use crate::provision::Provision;

/// `SYMDEV_DEVICE`, else the running emulators and the profiles; a prompt on stderr and
/// stdin when `terminal` and there is more than one.
pub(crate) fn pick_device(terminal: bool, provision: &Provision) -> Result<RegistryEntry> {
    let devices = Devices::from_env()?;
    let running = devices.live()?;
    let requested = std::env::var("SYMDEV_DEVICE")
        .ok()
        .filter(|v| !v.is_empty());
    // A profile is made only when a start needs one: nothing runs, or SYMDEV_DEVICE names
    // a profile rather than an emulator id.
    let wants_profile = running.is_empty()
        || requested
            .as_deref()
            .is_some_and(|r| DeviceId::parse(r).is_none());
    let mut profiles = devices.profiles();
    if profiles.is_empty() && wants_profile {
        profiles = devices.make_profiles(provision.firmwares()?)?;
    }
    let choice = DeviceChoice {
        requested,
        running: running.clone(),
        profiles,
        terminal,
    }
    .decide();
    let offer = match choice {
        Choice::Use(id) => Offer::Running {
            id,
            name: String::new(),
        },
        Choice::Start(profile) => Offer::Profile(profile),
        Choice::Refuse(why) => return Err(Error::Other(why)),
        Choice::Ask(offers) => ask(&offers)?,
    };
    match offer {
        Offer::Running { id, .. } => running
            .into_iter()
            .find(|e| e.id == id)
            .ok_or_else(|| Error::Other(format!("{id} is not running"))),
        Offer::Profile(profile) => {
            let eka2l1 = eka2l1_with_control(provision)?;
            eprintln!("starting an emulator on profile {profile}");
            let entry =
                EmulatorInstance::start(&eka2l1, &devices.profile(&profile), devices.registry())?;
            eprintln!("{} is {}", entry.id, entry.name);
            Ok(entry)
        }
    }
}

/// `flutter run`'s prompt: the numbered devices, a digit picks, `q` quits.
fn ask(offers: &[Offer]) -> Result<Offer> {
    let stdin = std::io::stdin();
    loop {
        eprintln!("Several devices can run this:");
        for line in DevicePrompt::lines(offers) {
            eprintln!("{line}");
        }
        eprint!("Please choose one (or \"q\" to quit): ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        if stdin
            .lock()
            .read_line(&mut line)
            .map_err(|e| Error::Other(e.to_string()))?
            == 0
        {
            return Err(Error::Other("no device chosen".into()));
        }
        match DevicePrompt::answer(&line, offers) {
            Ok(Some(offer)) => return Ok(offer),
            Ok(None) => return Err(Error::Other("no device chosen".into())),
            Err(e) => eprintln!("{e}"),
        }
    }
}
