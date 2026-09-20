//! `Module`: what one MMP builds (its E32 kind and UIDs).
use symdev_core::{Error, Result};

use crate::Mmp;

/// What one MMP builds: its E32 kind and UIDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Module {
    pub dll: bool,
    pub uid2: u32,
    pub uid3: u32,
    /// `EPOCALLOWDLLDATA`: writable static data in a DLL (`--dlldata`).
    pub allow_data: bool,
    /// `SECUREID`, when the `.mmp` set one; it defaults to UID3 (§10.2).
    pub secureid: Option<u32>,
}

impl Module {
    /// DLL UIDs come from the MMP `UID <uid2> <uid3>` line; EXEs keep the recorded
    /// experiment-6 UIDs (UID2 omitted, UID3 from the manifest).
    pub fn of(mmp: &Mmp, manifest_uid3: u32) -> Result<Self> {
        if mmp.is_dll() {
            let [uid2, uid3] = mmp.uid[..] else {
                return Err(Error::Other(format!(
                    "{}: TARGETTYPE DLL needs `UID <uid2> <uid3>`",
                    mmp.target
                )));
            };
            return Ok(Self {
                dll: true,
                uid2,
                uid3,
                allow_data: mmp.epocallowdlldata,
                secureid: mmp.secureid,
            });
        }
        Ok(Self {
            dll: false,
            uid2: 0,
            uid3: manifest_uid3,
            allow_data: false,
            secureid: mmp.secureid,
        })
    }

    pub(super) fn ext(&self) -> &'static str {
        if self.dll { "dll" } else { "exe" }
    }
}
