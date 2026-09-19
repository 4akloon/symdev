//! `.rsc` (`rcomp -u`) and `.rsg` output for a compiled resource file (experiment 56).

use symdev_core::{Error, Result};
use symdev_uidcrc::UidCrc;

use crate::compiler::{RscCompiled, RscResourceData, RscSegment};
use crate::scsu::RscScsu;

impl RscCompiled {
    pub const UID1: u32 = 0x101f_4a6b;
    /// Header flag: UID3 is the `NAME` value (no `UID3` statement).
    pub const FLAG_UID3_FROM_NAME: u8 = 0x01;

    /// UIDs, flags, largest uncompressed resource (`u16`), one bit per resource that is
    /// packed, the resources, then `u16` start offsets and the index offset.
    pub fn rsc_bytes(&self) -> Result<Vec<u8>> {
        let mut out = UidCrc::new(Self::UID1, self.uid2, self.uid3)
            .bytes()
            .to_vec();
        out.push(if self.uid3_from_name {
            Self::FLAG_UID3_FROM_NAME
        } else {
            0
        });
        let largest = self
            .resources
            .iter()
            .map(|r| r.data.len())
            .max()
            .unwrap_or(0);
        out.extend_from_slice(
            &u16::try_from(largest)
                .map_err(|_| Error::Other(format!("resource of {largest} bytes")))?
                .to_le_bytes(),
        );
        let mut bits = vec![0u8; self.resources.len().div_ceil(8)];
        let mut bodies = Vec::new();
        for (i, r) in self.resources.iter().enumerate() {
            match Self::packed(&r.data)? {
                Some(packed) => {
                    bits[i / 8] |= 1 << (i % 8);
                    bodies.push(packed);
                }
                None => bodies.push(r.data.uncompressed()),
            }
        }
        out.extend_from_slice(&bits);
        let mut index = Vec::new();
        for body in bodies {
            index.push(out.len());
            out.extend_from_slice(&body);
        }
        index.push(out.len());
        for at in index {
            out.extend_from_slice(
                &u16::try_from(at)
                    .map_err(|_| Error::Other(format!("resource file over 64 KB ({at})")))?
                    .to_le_bytes(),
            );
        }
        Ok(out)
    }

    /// Runs alternate compressed / raw, starting with a compressed run (possibly
    /// empty); pads are dropped. `None` when the resource has no 16-bit text.
    fn packed(data: &RscResourceData) -> Result<Option<Vec<u8>>> {
        if !data
            .segments
            .iter()
            .any(|s| matches!(s, RscSegment::Text(_)))
        {
            return Ok(None);
        }
        // (compressed, bytes)
        let mut runs: Vec<(bool, Vec<u8>)> = vec![(true, Vec::new())];
        for seg in &data.segments {
            match seg {
                RscSegment::Pad => {}
                RscSegment::Raw(b) => match runs.last_mut() {
                    Some((false, last)) => last.extend_from_slice(b),
                    _ => runs.push((false, b.clone())),
                },
                RscSegment::Text(t) => {
                    let enc = RscScsu::encode(t)?;
                    match runs.last_mut() {
                        Some((true, last)) if last.is_empty() => *last = enc,
                        Some((true, _)) => {
                            runs.push((false, Vec::new()));
                            runs.push((true, enc));
                        }
                        _ => runs.push((true, enc)),
                    }
                }
            }
        }
        let mut out = Vec::new();
        for (_, bytes) in runs {
            let n = bytes.len();
            match n {
                0..=0x7f => out.push(n as u8),
                0x80..=0x7fff => out.extend_from_slice(&[0x80 | (n >> 8) as u8, n as u8]),
                _ => {
                    return Err(Error::Other(format!(
                        "TODO: run of {n} bytes (not observed)"
                    )));
                }
            }
            out.extend_from_slice(&bytes);
        }
        Ok(Some(out))
    }

    /// `rcomp -h`: `#define <NAME>` padded to column 50 (at least one space), the id
    /// as `0x%x` (decimal without a `NAME` statement), CRLF, for each named resource
    /// (experiment 56).
    pub fn rsg_text(&self) -> String {
        let mut out = String::new();
        for r in &self.resources {
            if let Some(name) = &r.name {
                let name = name.to_ascii_uppercase();
                let id = if self.named {
                    format!("0x{:x}", r.id)
                } else {
                    r.id.to_string()
                };
                out.push_str(&format!("#define {name:<41} {id}\r\n"));
            }
        }
        out
    }
}
