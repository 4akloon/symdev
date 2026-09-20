//! Parses a `--uid1`/`--uid2`/`--uid3`/`--sid` hex value.
use symdev_core::{Error, Result};

pub(super) fn parse_uid(s: &str) -> Result<u32> {
    let hex = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    u32::from_str_radix(hex, 16).map_err(|_| Error::Other(format!("invalid UID: {s}")))
}
