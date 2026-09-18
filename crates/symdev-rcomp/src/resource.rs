use super::RscUid;
use symdev_core::{Error, Result};

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

    pub fn bytes(&self) -> Vec<u8> {
        let units: Vec<u16> = self.chars.encode_utf16().collect();
        if units.is_empty() {
            return vec![0];
        }
        let mut out = vec![units.len() as u8, 0];
        for u in units {
            out.extend_from_slice(&u.to_le_bytes());
        }
        out
    }
}

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

    pub fn bytes(&self) -> Result<Vec<u8>> {
        Ok(self.resource()?.uncompressed().to_vec())
    }

    pub fn resource(&self) -> Result<RscResource> {
        let mut packed = RscPacked::new();
        packed.push_slice(&[0; 8])?;
        packed.push_ltext16(&self.app_file)?;
        packed.push_slice(&[0; 4])?;
        packed.push_ltext16(&self.localisable_resource_file)?;
        packed.push_slice(&self.localisable_resource_id.to_le_bytes())?;
        packed.push_slice(&[0; 4])?;
        packed.push_ltext16(&RscLtext16::empty())?;
        packed.push_slice(&[0])?;
        packed.push_slice(&[0; 6])?;
        packed.push_slice(&[0; 4])?;
        packed.finish()
    }
}

pub struct RscResource {
    packed: Vec<u8>,
    uncompressed: Vec<u8>,
}

impl RscResource {
    pub fn packed_bytes(&self) -> &[u8] {
        &self.packed
    }

    pub fn uncompressed(&self) -> &[u8] {
        &self.uncompressed
    }

    pub fn uncompressed_len(&self) -> Result<u16> {
        u16::try_from(self.uncompressed.len())
            .map_err(|_| Error::Other("RSC uncompressed overflow".into()))
    }
}

pub struct Rsc {
    pub uid: RscUid,
    resources: Vec<RscResource>,
}

impl Rsc {
    pub fn new(uid: RscUid, resources: Vec<RscResource>) -> Self {
        Self { uid, resources }
    }

    pub fn bytes(&self) -> Result<Vec<u8>> {
        let largest = self
            .resources
            .iter()
            .map(RscResource::uncompressed_len)
            .try_fold(0u16, |acc, n| n.map(|n| acc.max(n)))?;
        let largest = u8::try_from(largest)
            .map_err(|_| Error::Other("RSC uncompressed larger than 255".into()))?;
        let mut body = Vec::new();
        body.extend_from_slice(&self.uid.bytes());
        body.extend_from_slice(&[0, largest, 0, 1]);
        let mut offsets = vec![
            u16::try_from(body.len())
                .map_err(|_| Error::Other("RSC index offset overflow".into()))?,
        ];
        for resource in &self.resources {
            body.extend_from_slice(resource.packed_bytes());
            offsets.push(
                u16::try_from(body.len())
                    .map_err(|_| Error::Other("RSC index offset overflow".into()))?,
            );
        }
        for offset in offsets {
            body.extend_from_slice(&offset.to_le_bytes());
        }
        Ok(body)
    }
}

struct RscPacked {
    packed: Vec<u8>,
    literals: Vec<u8>,
    uncompressed: Vec<u8>,
}

impl RscPacked {
    fn new() -> Self {
        Self {
            packed: vec![0],
            literals: Vec::new(),
            uncompressed: Vec::new(),
        }
    }

    fn push_slice(&mut self, bytes: &[u8]) -> Result<()> {
        u16::try_from(self.uncompressed.len() + bytes.len())
            .map_err(|_| Error::Other("RSC uncompressed overflow".into()))?;
        self.literals.extend_from_slice(bytes);
        self.uncompressed.extend_from_slice(bytes);
        Ok(())
    }

    fn flush_literals(&mut self) {
        let mut rest = self.literals.as_slice();
        while !rest.is_empty() {
            let n = rest.len().min(255);
            self.packed.push(n as u8);
            self.packed.extend_from_slice(&rest[..n]);
            rest = &rest[n..];
        }
        self.literals.clear();
    }

    fn push_ltext16(&mut self, text: &RscLtext16) -> Result<()> {
        let raw = text.bytes();
        if raw.len() == 1 {
            return self.push_slice(&raw);
        }
        if text.chars.encode_utf16().any(|u| u > 0xff) {
            return Err(Error::Other("LText16 packed form requires Latin-1".into()));
        }
        let n = raw[0];
        self.push_slice(&[n])?;
        self.flush_literals();
        self.packed.push(n);
        self.packed.extend(raw[2..].iter().step_by(2).copied());
        u16::try_from(self.uncompressed.len() + raw.len() - 1)
            .map_err(|_| Error::Other("RSC uncompressed overflow".into()))?;
        self.uncompressed.extend_from_slice(&raw[1..]);
        Ok(())
    }

    fn finish(mut self) -> Result<RscResource> {
        self.flush_literals();
        Ok(RscResource {
            packed: self.packed,
            uncompressed: self.uncompressed,
        })
    }
}
