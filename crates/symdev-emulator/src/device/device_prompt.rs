//! `DevicePrompt`: the numbered list `flutter run` shows when several devices could run
//! the app (design spec §5): a digit picks, `q` quits.
use symdev_core::{Error, Result};

use super::Offer;

pub struct DevicePrompt;

impl DevicePrompt {
    /// `[1]: Nokia N00 (RM-469) (emulator-1)`, `[2]: rm-469 (start a new emulator)`.
    pub fn lines(offers: &[Offer]) -> Vec<String> {
        offers
            .iter()
            .enumerate()
            .map(|(i, offer)| match offer {
                Offer::Running { id, name } => format!("[{}]: {name} ({id})", i + 1),
                Offer::Profile(p) => format!("[{}]: {p} (start a new emulator)", i + 1),
            })
            .collect()
    }

    /// The offer a typed line picks, `None` for `q`; anything else is an error, and the
    /// caller asks again.
    pub fn answer(line: &str, offers: &[Offer]) -> Result<Option<Offer>> {
        let line = line.trim();
        if line == "q" {
            return Ok(None);
        }
        line.parse::<usize>()
            .ok()
            .and_then(|n| n.checked_sub(1))
            .and_then(|i| offers.get(i))
            .map(|o| Some(o.clone()))
            .ok_or_else(|| {
                Error::Other(format!(
                    "`{line}`: type a number from 1 to {}, or q to quit",
                    offers.len()
                ))
            })
    }
}
