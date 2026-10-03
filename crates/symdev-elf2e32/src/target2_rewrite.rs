//! `Target2Rewrite`: `R_ARM_TARGET2` → `R_ARM_ABS32` in ELF objects, in place.
use symdev_core::{Error, Result};

use crate::ar_members::ArMembers;
use crate::elf_section_headers::ElfSectionHeaders;

/// An object, or an `ar` archive of objects, whose `R_ARM_TARGET2` relocations now say
/// `R_ARM_ABS32` (experiment 109 §2, change 5).
///
/// On Symbian `R_ARM_TARGET2` means an absolute word: GNU ld's symbianelf target links it
/// as `R_ARM_ABS32`, and libsupc++'s personality routine, built for `__symbian__`, reads
/// the catch typeinfo word of an exception table as an absolute pointer. rust-lld treats
/// it as absolute with `--target2=abs`, but cannot emit a dynamic relocation for it
/// against an imported symbol, which the `typeinfo for XLeaveException` of every `TRAP`
/// needs. With the type rewritten the input means the same to GNU ld, and lld links it.
///
/// Only the type byte of each relocation entry changes (`r_info` keeps its symbol), so no
/// size or offset moves. Bytes in, bytes out: no file is read or written here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target2Rewrite {
    bytes: Vec<u8>,
    rewritten: usize,
}

impl Target2Rewrite {
    pub const R_ARM_ABS32: u8 = 2;
    pub const R_ARM_TARGET2: u8 = 41;

    /// One ELF32 little-endian ARM relocatable object (`ET_REL`), as GCCE and RVCT write
    /// them. Anything else is refused.
    pub fn object(bytes: &[u8]) -> Result<Self> {
        let mut out = bytes.to_vec();
        let rewritten = Self::rewrite(&mut out)?;
        Ok(Self {
            bytes: out,
            rewritten,
        })
    }

    /// A System V / GNU `ar` archive (`ar`, RVCT's `armar`): every member is rewritten as
    /// [`Self::object`] except the archive's own symbol and long-name tables. A member that
    /// is not such an object is refused, with its name.
    pub fn archive(bytes: &[u8]) -> Result<Self> {
        let mut out = bytes.to_vec();
        let mut rewritten = 0;
        for member in ArMembers::read(bytes)?.members {
            if member.is_table() {
                continue;
            }
            rewritten += Self::rewrite(&mut out[member.data.clone()])
                .map_err(|e| Error::Other(format!("archive member `{}`: {e}", member.name)))?;
        }
        Ok(Self {
            bytes: out,
            rewritten,
        })
    }

    /// How many relocations were rewritten.
    pub fn rewritten(&self) -> usize {
        self.rewritten
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// Every relocation section is checked before the first byte changes.
    fn rewrite(object: &mut [u8]) -> Result<usize> {
        let headers = ElfSectionHeaders::read(object)?;
        if headers.e_type != ElfSectionHeaders::ET_REL {
            return Err(Error::Other(format!(
                "ELF type {}, not a relocatable object",
                headers.e_type
            )));
        }
        let mut tables = Vec::new();
        for s in &headers.sections {
            let entry = match s.kind {
                ElfSectionHeaders::SHT_REL => 8,
                ElfSectionHeaders::SHT_RELA => 12,
                _ => continue,
            };
            if s.entsize != entry {
                return Err(Error::Other(format!(
                    "section {}: entry size {}, expected {entry}",
                    s.index, s.entsize
                )));
            }
            if s.size % entry != 0 {
                return Err(Error::Other(format!(
                    "section {}: size {} is not a whole number of entries",
                    s.index, s.size
                )));
            }
            tables.push((s.offset, s.size, entry));
        }
        let mut rewritten = 0;
        for (offset, size, entry) in tables {
            for at in (offset..offset + size).step_by(entry) {
                if object[at + 4] == Self::R_ARM_TARGET2 {
                    object[at + 4] = Self::R_ARM_ABS32;
                    rewritten += 1;
                }
            }
        }
        Ok(rewritten)
    }
}

#[cfg(test)]
mod tests;
