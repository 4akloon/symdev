//! Compiled resources: segments (raw bytes, alignment pads, 16-bit text) and the
//! compiled file.

/// One piece of a resource: bytes stored as they are, the alignment byte before 16-bit
/// text (dropped when the resource is packed), or 16-bit text (compressed when packed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RscSegment {
    Raw(Vec<u8>),
    Pad,
    Text(Vec<u16>),
}

/// A compiled resource: its segments in order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RscResourceData {
    pub segments: Vec<RscSegment>,
}

impl RscResourceData {
    /// Length when completely uncompressed (pads and UTF-16 included).
    pub fn len(&self) -> usize {
        self.segments
            .iter()
            .map(|s| match s {
                RscSegment::Raw(b) => b.len(),
                RscSegment::Pad => 1,
                RscSegment::Text(t) => 2 * t.len(),
            })
            .sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn uncompressed(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.len());
        for s in &self.segments {
            match s {
                RscSegment::Raw(b) => out.extend_from_slice(b),
                RscSegment::Pad => out.push(0),
                RscSegment::Text(t) => t
                    .iter()
                    .for_each(|u| out.extend_from_slice(&u.to_le_bytes())),
            }
        }
        out
    }

    pub(super) fn raw(&mut self, bytes: &[u8]) {
        if let Some(RscSegment::Raw(last)) = self.segments.last_mut() {
            last.extend_from_slice(bytes);
        } else {
            self.segments.push(RscSegment::Raw(bytes.to_vec()));
        }
    }

    /// 16-bit text starts at an even offset within the resource (experiment 56:
    /// `LTEXT` alone is `len, 0x00, UTF-16`; after an odd number of bytes no pad).
    pub(super) fn text16(&mut self, units: &[u16]) {
        if units.is_empty() {
            return;
        }
        if self.len() % 2 == 1 {
            self.segments.push(RscSegment::Pad);
        }
        self.segments.push(RscSegment::Text(units.to_vec()));
    }
}

/// The compiled file: UIDs and resources in source order, with their names.
#[derive(Debug, Clone, PartialEq)]
pub struct RscCompiled {
    pub uid2: u32,
    pub uid3: u32,
    /// No `UID3` statement: UID3 is the `NAME` value (header flag, experiment 56).
    pub uid3_from_name: bool,
    /// A `NAME` statement was given (ids carry it; `.rsg` prints them in hex).
    pub named: bool,
    pub resources: Vec<RscCompiledResource>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RscCompiledResource {
    pub name: Option<String>,
    pub id: u32,
    pub data: RscResourceData,
}
