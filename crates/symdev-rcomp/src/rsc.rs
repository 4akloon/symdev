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
        let mut raw_open = false;
        for (i, seg) in data.segments.iter().enumerate() {
            match seg {
                // A pad only exists before text kept raw, and then it is the fill byte.
                RscSegment::Pad => {}
                RscSegment::Raw(b) => {
                    Self::push_raw(&mut runs, b);
                    raw_open = true;
                }
                RscSegment::Text(t) => {
                    let enc = RscScsu::encode(t)?;
                    let padded = data.segments.get(i.wrapping_sub(1)) == Some(&RscSegment::Pad);
                    let more = i + 1 < data.segments.len();
                    if Self::compress_is_shorter(enc.len(), t.len(), padded, raw_open, more) {
                        match runs.last_mut() {
                            Some((true, last)) if last.is_empty() => *last = enc,
                            Some((true, _)) => {
                                runs.push((false, Vec::new()));
                                runs.push((true, enc));
                            }
                            _ => runs.push((true, enc)),
                        }
                        raw_open = false;
                    } else {
                        let mut bytes = Vec::with_capacity(2 * t.len() + 1);
                        if padded {
                            bytes.push(Self::RAW_PAD);
                        }
                        t.iter()
                            .for_each(|u| bytes.extend_from_slice(&u.to_le_bytes()));
                        Self::push_raw(&mut runs, &bytes);
                        raw_open = true;
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

    /// Fill byte before 16-bit text that stays raw inside a packed resource
    /// (experiment 56: `imopenapiexample`).
    const RAW_PAD: u8 = 0xab;

    fn push_raw(runs: &mut Vec<(bool, Vec<u8>)>, bytes: &[u8]) {
        match runs.last_mut() {
            Some((false, last)) => last.extend_from_slice(bytes),
            _ => runs.push((false, bytes.to_vec())),
        }
    }

    /// Text is compressed only when that makes the resource shorter; ties stay raw
    /// (experiment 56: nine one-character texts, only the last one — with nothing after
    /// it — is compressed). Compressing costs its run header, plus one byte to reopen a
    /// raw run when anything follows, plus one to close an open compressed run; staying
    /// raw costs the pad and two bytes per character, plus one byte when a raw run has
    /// to be opened (two at the very start, after the empty compressed run).
    fn compress_is_shorter(
        encoded: usize,
        chars: usize,
        padded: bool,
        raw_open: bool,
        more_follows: bool,
    ) -> bool {
        let header = |n: usize| if n < 0x80 { 1 } else { 2 };
        let compressed = header(encoded) + encoded + usize::from(more_follows);
        let raw = usize::from(padded) + 2 * chars + usize::from(!raw_open);
        compressed < raw
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
