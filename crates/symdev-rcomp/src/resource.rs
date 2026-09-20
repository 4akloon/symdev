//! The application registration resource symdev writes when a project has no
//! `_reg.rss` of its own (experiments 41-43): the same bytes `rcomp` produces for
//! `APP_REGISTRATION_INFO`, built through the normal resource writer.

use super::RscUid;
use crate::compiler::{RscCompiled, RscCompiledResource, RscResourceData};
use symdev_core::{Error, Result};

/// An `LText16` value: a length byte and UTF-16 characters.
pub struct RscLtext16 {
    chars: String,
}

impl RscLtext16 {
    pub fn new(text: impl Into<String>) -> Result<Self> {
        let chars = text.into();
        if chars.encode_utf16().count() > 255 {
            return Err(Error::Other("LText16 longer than 255".into()));
        }
        Ok(Self { chars })
    }

    pub fn empty() -> Self {
        Self {
            chars: String::new(),
        }
    }

    fn units(&self) -> Vec<u16> {
        self.chars.encode_utf16().collect()
    }

    /// The `LTEXT16` layout on its own: length byte, alignment pad, UTF-16.
    pub fn bytes(&self) -> Vec<u8> {
        let mut data = RscResourceData::default();
        self.push(&mut data);
        data.uncompressed()
    }

    /// Length byte, then the text with its alignment pad (the `LTEXT16` layout).
    fn push(&self, data: &mut RscResourceData) {
        let units = self.units();
        data.raw(&[units.len() as u8]);
        data.text16(&units);
    }
}

/// `APP_REGISTRATION_INFO` as the SDK examples declare it (experiment 42).
pub struct RscAppRegistration {
    pub app_file: RscLtext16,
    pub localisable_resource_file: RscLtext16,
    pub localisable_resource_id: u32,
}

impl RscAppRegistration {
    pub fn new(
        app_file: RscLtext16,
        localisable_resource_file: RscLtext16,
        localisable_resource_id: u32,
    ) -> Self {
        Self {
            app_file,
            localisable_resource_file,
            localisable_resource_id,
        }
    }

    /// The resource image, uncompressed (what the `.rsc` header counts).
    pub fn bytes(&self) -> Vec<u8> {
        self.data().uncompressed()
    }

    pub fn data(&self) -> RscResourceData {
        let mut data = RscResourceData::default();
        data.raw(&[0; 8]);
        self.app_file.push(&mut data);
        data.raw(&[0; 4]);
        self.localisable_resource_file.push(&mut data);
        data.raw(&self.localisable_resource_id.to_le_bytes());
        data.raw(&[0; 4]);
        RscLtext16::empty().push(&mut data);
        data.raw(&[0; 11]);
        data
    }
}

/// A resource file symdev writes itself.
pub struct Rsc {
    pub uid: RscUid,
    resources: Vec<RscResourceData>,
}

impl Rsc {
    pub fn new(uid: RscUid, resources: Vec<RscResourceData>) -> Self {
        Self { uid, resources }
    }

    pub fn registration(uid3: u32, app_file: impl Into<String>) -> Result<Self> {
        let app_file = RscLtext16::new(app_file)?;
        if app_file.units().is_empty() {
            return Err(Error::Other("APP_REGISTRATION_INFO app_file empty".into()));
        }
        Ok(Self::new(
            RscUid::registration(uid3),
            vec![RscAppRegistration::new(app_file, RscLtext16::empty(), 1).data()],
        ))
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        RscCompiled {
            uid2: self.uid.uid2,
            uid3: self.uid.uid3,
            uid3_from_name: false,
            named: false,
            resources: self
                .resources
                .iter()
                .enumerate()
                .map(|(i, data)| RscCompiledResource {
                    name: None,
                    id: i as u32 + 1,
                    data: data.clone(),
                })
                .collect(),
        }
        .rsc_bytes()
    }
}

#[cfg(test)]
mod tests;
