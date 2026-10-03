//! `DeviceChoice`: which device `cargo run` and `cargo test` use, by `flutter run`'s rules
//! (design spec §5): `SYMDEV_DEVICE` wins; one running device is it; with none running and
//! one profile, that profile is started; otherwise the user is asked, or without a terminal
//! told how to choose.
use super::{DeviceId, RegistryEntry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceChoice {
    /// `SYMDEV_DEVICE`: a device id or a profile name.
    pub requested: Option<String>,
    /// The live registry entries, in id order.
    pub running: Vec<RegistryEntry>,
    /// The emulator profiles there are.
    pub profiles: Vec<String>,
    /// Whether a person can answer a prompt.
    pub terminal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Choice {
    Use(DeviceId),
    Start(String),
    Ask(Vec<Offer>),
    Refuse(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    Running { id: DeviceId, name: String },
    Profile(String),
}

impl DeviceChoice {
    pub fn decide(&self) -> Choice {
        if let Some(wanted) = &self.requested {
            return self.requested_one(wanted);
        }
        match (self.running.as_slice(), self.profiles.as_slice()) {
            ([one], _) => Choice::Use(one.id),
            ([], [profile]) => Choice::Start(profile.clone()),
            ([], []) => Choice::Refuse(
                "no emulator profile and no firmware installed in EKA2L1 to make one from; \
                 install a firmware in EKA2L1 first"
                    .into(),
            ),
            _ if self.terminal => Choice::Ask(self.offers()),
            _ => Choice::Refuse(format!(
                "several devices: {}; set SYMDEV_DEVICE to one of them",
                self.everything()
            )),
        }
    }

    fn requested_one(&self, wanted: &str) -> Choice {
        if let Some(id) = DeviceId::parse(wanted)
            && self.running.iter().any(|e| e.id == id)
        {
            return Choice::Use(id);
        }
        if let Some(entry) = self.running.iter().find(|e| e.profile == wanted) {
            return Choice::Use(entry.id);
        }
        if self.profiles.iter().any(|p| p == wanted) {
            return Choice::Start(wanted.to_string());
        }
        Choice::Refuse(format!(
            "SYMDEV_DEVICE={wanted} is neither a running emulator nor a profile; there are: {}",
            match self.everything() {
                all if all.is_empty() => "none".to_string(),
                all => all,
            }
        ))
    }

    fn offers(&self) -> Vec<Offer> {
        let running = self.running.iter().map(|e| Offer::Running {
            id: e.id,
            name: e.name.clone(),
        });
        running
            .chain(self.profiles.iter().cloned().map(Offer::Profile))
            .collect()
    }

    /// `emulator-1, emulator-2, profile rm-469`.
    fn everything(&self) -> String {
        let running = self.running.iter().map(|e| e.id.to_string());
        let profiles = self.profiles.iter().map(|p| format!("profile {p}"));
        running.chain(profiles).collect::<Vec<_>>().join(", ")
    }
}
