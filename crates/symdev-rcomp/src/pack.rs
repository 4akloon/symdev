//! Packing a resource image: which texts are stored compressed and how the runs are
//! laid out ([rcomp-spec.md](../../../docs/research/rcomp-spec.md) §1).

use symdev_core::{Error, Result};

use crate::compiler::{RscResourceData, RscSegment};
use crate::scsu::RscScsu;

/// One compressible text of a resource, as the profitability pass sees it.
struct PackedText {
    /// Index in `segments`; `None` for the empty text the format starts with.
    segment: Option<usize>,
    /// Bytes it takes in the uncompressed image (`pad + 2 * n`).
    raw_size: usize,
    /// Compressed bytes.
    compressed: Vec<u8>,
    /// Raw bytes that follow it, up to the next compressible text or the end.
    trailing: Vec<u8>,
}

/// A resource image with its text compression decided.
pub struct RscPacker;

impl RscPacker {
    /// Alignment byte before 16-bit text that is stored raw, packed or not (spec §3.6).
    pub const PAD: u8 = 0xab;

    /// `Some(bytes)` when the resource is packed, `None` when every text ended up raw
    /// and the plain uncompressed image is written instead.
    pub fn pack(data: &RscResourceData) -> Result<Option<Vec<u8>>> {
        // Level one: compress each text under its `2 * n` budget; what does not fit is
        // raw for good (spec §1.1).
        let mut compressed: Vec<Option<Vec<u8>>> = data
            .segments
            .iter()
            .map(|s| match s {
                RscSegment::Text(t) if !t.is_empty() => RscScsu::encode(t),
                _ => None,
            })
            .collect();
        // The leading empty compressed run, dropped for good once it loses its test.
        let mut leading_empty = true;
        loop {
            let texts = Self::layout(data, &compressed, leading_empty);
            if texts.is_empty() {
                return Ok(None);
            }
            match Self::unprofitable(&texts) {
                None => return Self::emit(&texts).map(Some),
                Some(at) => match texts[at].segment {
                    Some(seg) => compressed[seg] = None,
                    None => leading_empty = false,
                },
            }
        }
    }

    /// Walk the image, collecting the compressible texts with their sizes and the raw
    /// bytes between them (spec §1.3 pass 0).
    fn layout(
        data: &RscResourceData,
        compressed: &[Option<Vec<u8>>],
        leading_empty: bool,
    ) -> Vec<PackedText> {
        let mut texts: Vec<PackedText> = Vec::new();
        let mut raw: Vec<u8> = Vec::new();
        for (i, seg) in data.segments.iter().enumerate() {
            match seg {
                RscSegment::Pad => {}
                RscSegment::Raw(b) => {
                    raw.extend_from_slice(b);
                }
                RscSegment::Text(t) => {
                    let pad =
                        usize::from(data.segments.get(i.wrapping_sub(1)) == Some(&RscSegment::Pad));
                    match &compressed[i] {
                        Some(bytes) => {
                            if texts.is_empty() && leading_empty && !raw.is_empty() {
                                texts.push(PackedText {
                                    segment: None,
                                    raw_size: 0,
                                    compressed: Vec::new(),
                                    trailing: Vec::new(),
                                });
                            }
                            if let Some(last) = texts.last_mut() {
                                last.trailing = std::mem::take(&mut raw);
                            }
                            raw.clear();
                            texts.push(PackedText {
                                segment: Some(i),
                                raw_size: pad + 2 * t.len(),
                                compressed: bytes.clone(),
                                trailing: Vec::new(),
                            });
                        }
                        None => {
                            if pad == 1 {
                                raw.push(Self::PAD);
                            }
                            t.iter()
                                .for_each(|u| raw.extend_from_slice(&u.to_le_bytes()));
                        }
                    }
                }
            }
        }
        if let Some(last) = texts.last_mut() {
            last.trailing = raw;
        }
        texts
    }

    /// The first text whose compressed form does not pay for itself (spec §1.3 pass 1).
    /// The first text is free when it is not also the last: the image always starts with
    /// a compressed run.
    fn unprofitable(texts: &[PackedText]) -> Option<usize> {
        let header = |n: usize| if n <= 0x7f { 1 } else { 2 };
        for (i, t) in texts.iter().enumerate() {
            let last = i + 1 == texts.len();
            if i == 0 && !last {
                continue;
            }
            let mut cost = t.compressed.len() + header(t.compressed.len());
            if !(last && t.trailing.is_empty()) {
                cost += header(t.trailing.len());
            }
            if cost >= t.raw_size {
                return Some(i);
            }
        }
        None
    }

    /// Runs in image order: each compressed run, then the header and bytes of the raw
    /// run that follows it (spec §1.5 pass 2).
    fn emit(texts: &[PackedText]) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        for (i, t) in texts.iter().enumerate() {
            let last = i + 1 == texts.len();
            Self::run(&mut out, &t.compressed)?;
            if !(last && t.trailing.is_empty()) {
                Self::run(&mut out, &t.trailing)?;
            }
        }
        Ok(out)
    }

    fn run(out: &mut Vec<u8>, bytes: &[u8]) -> Result<()> {
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
        out.extend_from_slice(bytes);
        Ok(())
    }
}
