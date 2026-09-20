//! `E32Time`: Symbian time (microseconds since 0001-01-01 UTC).
use symdev_core::{Error, Result};

/// Symbian time: microseconds since 0001-01-01 UTC (`iTimeLo`/`iTimeHi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E32Time(pub u64);

impl E32Time {
    /// Microseconds from 0001-01-01 to 1970-01-01 (experiment 44: header time equals the
    /// file's mtime).
    const UNIX_EPOCH: u64 = 0x00dc_ddb3_0f2f_8000;

    pub fn from_system(t: std::time::SystemTime) -> Result<Self> {
        let since = t
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::Other(format!("time before 1970: {e}")))?;
        Ok(Self(Self::UNIX_EPOCH + since.as_micros() as u64))
    }

    pub fn lo(&self) -> u32 {
        self.0 as u32
    }

    pub fn hi(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}
