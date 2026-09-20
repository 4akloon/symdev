//! `ElfImage`: raw byte, string and dynamic-tag access shared by the other impls.
use super::image::ElfImage;
use super::types::ElfSection;
use symdev_core::{Error, Result};

impl ElfImage {
    pub(super) fn dynamic(&self, tag: u32) -> Result<Option<u32>> {
        let Some(dynamic) = self.section(Self::SHT_DYNAMIC) else {
            return Ok(None);
        };
        for entry in (dynamic.offset..dynamic.offset + dynamic.size).step_by(8) {
            match self.u32_at(entry)? {
                Self::DT_NULL => break,
                t if t == tag => return Ok(Some(self.u32_at(entry + 4)?)),
                _ => {}
            }
        }
        Ok(None)
    }

    /// File bytes a dynamic tag points at. Non-loaded sections (address 0, as on
    /// experiment-6 `hello.elf`) carry their file offset in the tag.
    pub(super) fn file_range(&self, at: u32, len: u32) -> Result<std::ops::Range<usize>> {
        let section = self
            .sections
            .iter()
            .find(|s| (s.addr != 0 && s.addr == at) || (s.addr == 0 && s.offset == at as usize))
            .ok_or_else(|| Error::Other(format!("ELF dynamic pointer {at:#x} has no section")))?;
        Ok(section.offset..section.offset + len as usize)
    }

    pub(super) fn section(&self, kind: u32) -> Option<ElfSection> {
        self.sections.iter().copied().find(|s| s.kind == kind)
    }

    pub(super) fn string(&self, strtab: usize, at: usize) -> Result<String> {
        let table = self
            .sections
            .get(strtab)
            .ok_or_else(|| Error::Other(format!("ELF string table {strtab} missing")))?;
        let rest = self
            .bytes
            .get(table.offset + at..table.offset + table.size)
            .ok_or_else(|| Error::Other("ELF string out of range".into()))?;
        let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
        String::from_utf8(rest[..end].to_vec())
            .map_err(|_| Error::Other("ELF string is not UTF-8".into()))
    }

    pub(super) fn u16_at(&self, off: usize) -> Result<u16> {
        self.bytes
            .get(off..off + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| Error::Other(format!("ELF truncated at {off:#x}")))
    }

    pub(super) fn u32_at(&self, off: usize) -> Result<u32> {
        self.bytes
            .get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| Error::Other(format!("ELF truncated at {off:#x}")))
    }
}
