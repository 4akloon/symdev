//! Compressed Unicode in packed resources: SCSU (Unicode TR #6) as `rcomp` writes it.

use symdev_core::{Error, Result};

/// SCSU encoder state for one compressed run.
pub struct RscScsu;

impl RscScsu {
    /// Single-byte mode with the default windows: ASCII and Latin-1 pass through
    /// (dynamic window 0 starts at U+0080); control characters that collide with tags
    /// are quoted from static window 0.
    pub fn encode(units: &[u16]) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(units.len());
        for &u in units {
            match u {
                0x00 | 0x09 | 0x0a | 0x0d | 0x20..=0x7f => out.push(u as u8),
                // Tag bytes: quote from static window 0 (SQ0), e.g. U+000C → 01 0c
                // (experiment 56, richtexteditor).
                0x01..=0x08 | 0x0b | 0x0c | 0x0e..=0x1f => out.extend_from_slice(&[0x01, u as u8]),
                0x80..=0xff => out.push(u as u8),
                _ => {
                    return Err(Error::Other(format!(
                        "TODO: compressed U+{u:04X} (outside Latin-1, see rcomp-spec.md)"
                    )));
                }
            }
        }
        Ok(out)
    }
}
