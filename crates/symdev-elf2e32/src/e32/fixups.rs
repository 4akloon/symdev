//! `E32Fixups`: word fixups shared by the code, data and relocation sections.
use crate::ElfImage;
use symdev_core::{Error, Result};

/// Word fixups inside one ELF segment's bytes (`base` = its link address).
pub(super) struct E32Fixups;

impl E32Fixups {
    fn slot(bytes: &[u8], base: u32, vaddr: u32) -> Result<std::ops::Range<usize>> {
        let at = vaddr
            .checked_sub(base)
            .map(|o| o as usize)
            .filter(|&o| o + 4 <= bytes.len())
            .ok_or_else(|| {
                Error::Other(format!("fixup at {vaddr:#x} outside segment {base:#x}"))
            })?;
        Ok(at..at + 4)
    }

    pub(super) fn word(bytes: &[u8], base: u32, vaddr: u32) -> Result<u32> {
        let r = Self::slot(bytes, base, vaddr)?;
        Ok(u32::from_le_bytes([
            bytes[r.start],
            bytes[r.start + 1],
            bytes[r.start + 2],
            bytes[r.start + 3],
        ]))
    }

    pub(super) fn set_word(bytes: &mut [u8], base: u32, vaddr: u32, value: u32) -> Result<()> {
        let r = Self::slot(bytes, base, vaddr)?;
        bytes[r].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// `R_ARM_ABS32` words in `[base, base + len)` become `S + A`, `R_ARM_GLOB_DAT`
    /// words `S`; `R_ARM_RELATIVE` words stay as linked (experiments 44, 49, 51).
    pub(super) fn absolute(elf: &ElfImage, bytes: &mut [u8], base: u32) -> Result<()> {
        let end = base + bytes.len() as u32;
        for rel in elf.local_relocs()? {
            if !rel.absolute || !(base..end).contains(&rel.vaddr) {
                continue;
            }
            let addend = if rel.addend_in_place {
                Self::word(bytes, base, rel.vaddr)?
            } else {
                0
            };
            let value = rel.target.checked_add(addend).ok_or_else(|| {
                Error::Other(format!("absolute relocation at {:#x} overflows", rel.vaddr))
            })?;
            Self::set_word(bytes, base, rel.vaddr, value)?;
        }
        Ok(())
    }
}
