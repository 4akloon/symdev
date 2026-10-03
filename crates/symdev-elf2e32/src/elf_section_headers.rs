//! `ElfSectionHeaders`: the section header table of an ELF32 little-endian ARM file, read
//! for an in-place patch ([`crate::Target2Rewrite`], [`crate::StrtabPadding`]).
use symdev_core::{Error, Result};

use crate::elf_section_header::ElfSectionHeader;

/// The file type (`e_type`) and every section header, each checked to lie inside the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElfSectionHeaders {
    pub e_type: u16,
    pub sections: Vec<ElfSectionHeader>,
}

impl ElfSectionHeaders {
    pub const ET_REL: u16 = 1;
    pub const SHT_STRTAB: u32 = 3;
    pub const SHT_RELA: u32 = 4;
    pub const SHT_REL: u32 = 9;
    const SHT_NOBITS: u32 = 8;
    const EHDR: usize = 52;
    const SHDR: usize = 40;
    const EM_ARM: u16 = 40;

    /// Refuses anything but an ELF32 little-endian ARM file whose section header table and
    /// every section's contents lie inside `bytes`, so a patch never writes out of bounds.
    pub fn read(bytes: &[u8]) -> Result<Self> {
        let fail = |what: String| Err(Error::Other(what));
        if bytes.get(..4) != Some(b"\x7fELF".as_slice()) {
            return fail("not an ELF file".into());
        }
        if bytes.len() < Self::EHDR {
            return fail("truncated ELF header".into());
        }
        if bytes[4] != 1 {
            return fail(format!("ELF class {}, not ELF32", bytes[4]));
        }
        if bytes[5] != 1 {
            return fail("not a little-endian ELF".into());
        }
        let u16_at = |at: usize| u16::from_le_bytes([bytes[at], bytes[at + 1]]);
        let machine = u16_at(18);
        if machine != Self::EM_ARM {
            return fail(format!("machine {machine}, not ARM ({})", Self::EM_ARM));
        }
        let shoff = Self::u32_at(bytes, 0x20) as usize;
        let (shentsize, shnum) = (u16_at(0x2e) as usize, u16_at(0x30) as usize);
        let mut sections = Vec::new();
        if shnum == 0 && shoff != 0 {
            return fail("extended section numbering is not supported".into());
        }
        if shnum != 0 && shentsize != Self::SHDR {
            return fail(format!(
                "section header size {shentsize}, expected {}",
                Self::SHDR
            ));
        }
        let table_end = (shnum * Self::SHDR).checked_add(shoff);
        if table_end.is_none_or(|end| end > bytes.len()) {
            return fail("section header table past the end of the file".into());
        }
        for index in 0..shnum {
            let at = shoff + index * Self::SHDR;
            let section = ElfSectionHeader {
                index,
                kind: Self::u32_at(bytes, at + 4),
                offset: Self::u32_at(bytes, at + 16) as usize,
                size: Self::u32_at(bytes, at + 20) as usize,
                entsize: Self::u32_at(bytes, at + 36) as usize,
            };
            let end = section.offset.checked_add(section.size);
            if section.kind != Self::SHT_NOBITS && end.is_none_or(|end| end > bytes.len()) {
                return fail(format!(
                    "section {index}: contents past the end of the file"
                ));
            }
            sections.push(section);
        }
        Ok(Self {
            e_type: u16_at(16),
            sections,
        })
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }
}
