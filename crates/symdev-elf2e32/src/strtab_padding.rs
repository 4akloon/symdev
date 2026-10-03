//! `StrtabPadding`: string tables made NUL-terminated for lld, in place.
use symdev_core::{Error, Result};

use crate::elf_section_headers::ElfSectionHeaders;

/// An ELF file whose string tables end in a NUL (experiment 109 §2, change 2).
///
/// 428 of the 570 `.dso` import libraries of the S60 3rd FP2 SDK pad a string table with
/// one to three spaces after its last NUL. GNU ld 2.29.1 does not care; lld refuses the
/// file ("SHT_STRTAB string table section is non-null terminated"). Every byte after the
/// last NUL of each `SHT_STRTAB` becomes 0: no string an offset can name changes, and no
/// size or offset moves. Bytes in, bytes out: no file is read or written here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrtabPadding {
    bytes: Vec<u8>,
    zeroed: usize,
}

impl StrtabPadding {
    /// Any ELF32 little-endian ARM file. A non-empty string table with no NUL at all is
    /// refused: there is no terminator to keep, and lld would refuse it anyway.
    pub fn zero(bytes: &[u8]) -> Result<Self> {
        let headers = ElfSectionHeaders::read(bytes)?;
        let mut out = bytes.to_vec();
        let mut zeroed = 0;
        for s in &headers.sections {
            if s.kind != ElfSectionHeaders::SHT_STRTAB || s.size == 0 {
                continue;
            }
            let table = &mut out[s.offset..s.offset + s.size];
            let Some(last) = table.iter().rposition(|b| *b == 0) else {
                return Err(Error::Other(format!(
                    "section {}: a string table with no NUL",
                    s.index
                )));
            };
            for b in &mut table[last + 1..] {
                if *b != 0 {
                    *b = 0;
                    zeroed += 1;
                }
            }
        }
        Ok(Self { bytes: out, zeroed })
    }

    /// How many bytes were set to 0.
    pub fn zeroed(&self) -> usize {
        self.zeroed
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

#[cfg(test)]
mod tests;
