//! Source character sets (spec §5.6): CP1252 by default.

use symdev_core::{Error, Result};

/// How source bytes become characters (spec §5.6): CP1252 unless `CHARACTER_SET` says
/// otherwise; `ISOLATIN1`, `ASCII` and `CP850` all behave as ISO 8859-1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RssCharset {
    #[default]
    Cp1252,
    Latin1,
    Utf8,
}

impl RssCharset {
    /// CP1252 differs from ISO 8859-1 only in 0x80..=0x9F.
    const CP1252_HIGH: [u16; 32] = [
        0x20ac, 0x0081, 0x201a, 0x0192, 0x201e, 0x2026, 0x2020, 0x2021, 0x02c6, 0x2030, 0x0160,
        0x2039, 0x0152, 0x008d, 0x017d, 0x008f, 0x0090, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022,
        0x2013, 0x2014, 0x02dc, 0x2122, 0x0161, 0x203a, 0x0153, 0x009d, 0x017e, 0x0178,
    ];

    /// The charset a `CHARACTER_SET <name>` statement selects.
    pub fn from_name(name: &str) -> Result<Self> {
        match name.to_ascii_uppercase().as_str() {
            "CP1252" => Ok(Self::Cp1252),
            "ISOLATIN1" | "ASCII" | "CP850" => Ok(Self::Latin1),
            "UTF8" => Ok(Self::Utf8),
            other => Err(Error::Other(format!(
                "TODO: CHARACTER_SET {other} (rcomp rejects UNICODE and we did not observe SHIFTJIS)"
            ))),
        }
    }

    /// Characters of literal source bytes.
    pub(super) fn decode(self, bytes: &[u8]) -> Vec<u32> {
        match self {
            Self::Utf8 => String::from_utf8_lossy(bytes)
                .chars()
                .map(u32::from)
                .collect(),
            Self::Latin1 => bytes.iter().map(|&b| u32::from(b)).collect(),
            Self::Cp1252 => bytes
                .iter()
                .map(|&b| match b {
                    0x80..=0x9f => u32::from(Self::CP1252_HIGH[usize::from(b) - 0x80]),
                    _ => u32::from(b),
                })
                .collect(),
        }
    }
}
