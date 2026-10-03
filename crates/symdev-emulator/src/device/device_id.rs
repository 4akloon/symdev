//! `DeviceId`: `emulator-<n>`, the name a running device is chosen by (design spec §5).
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceId(u32);

impl DeviceId {
    const PREFIX: &'static str = "emulator-";

    /// `emulator-<n>` with `n ≥ 1` in decimal, no sign or leading zero; anything else is
    /// not a device id (it may be a profile name).
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.strip_prefix(Self::PREFIX)?;
        if digits.is_empty()
            || digits.starts_with('0')
            || !digits.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        digits.parse().ok().map(Self)
    }

    /// The `n`-th emulator, `n ≥ 1`.
    pub fn nth(n: u32) -> Option<Self> {
        (n >= 1).then_some(Self(n))
    }

    pub fn number(self) -> u32 {
        self.0
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::PREFIX, self.0)
    }
}
