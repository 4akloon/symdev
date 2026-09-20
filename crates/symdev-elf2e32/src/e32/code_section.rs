//! `E32CodeSection`: the E32 code section (import slots and relocations rewritten).
use super::fixups::E32Fixups;
use super::imports::E32ImportSection;
use super::layout::E32Layout;
use super::ordinals::E32Ordinals;
use crate::ElfImage;
use symdev_core::{Error, Result};

/// E32 code section: the ELF code segment with import slots and absolute
/// relocations rewritten (experiment 44).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32CodeSection {
    pub bytes: Vec<u8>,
}

impl E32CodeSection {
    /// Import slot word = `addend << 16 | ordinal`; `R_ARM_ABS32` word = `S + A`;
    /// `R_ARM_RELATIVE` words stay as linked.
    pub fn from_elf(elf: &ElfImage, layout: &E32Layout, ordinals: &E32Ordinals) -> Result<Self> {
        let mut bytes = elf.segment_bytes(elf.code_segment()?)?.to_vec();
        let base = layout.code_base;
        let end = base + bytes.len() as u32;
        for imp in elf.import_relocs()? {
            if imp.symbol == E32ImportSection::PURE_VIRTUAL {
                continue;
            }
            if !(base..end).contains(&imp.vaddr) {
                return Err(Error::Other(format!(
                    "TODO: import {} at {:#x} outside code (not observed)",
                    imp.symbol, imp.vaddr
                )));
            }
            let ordinal = ordinals.get(&imp.dll, &imp.symbol).ok_or_else(|| {
                Error::Other(format!("no ordinal for {} in {}", imp.symbol, imp.dll))
            })?;
            let addend = E32Fixups::word(&bytes, base, imp.vaddr)?;
            if addend > 0xffff || ordinal > 0xffff {
                return Err(Error::Other(format!(
                    "import {} addend {addend:#x} / ordinal {ordinal:#x} exceed 16 bits",
                    imp.symbol
                )));
            }
            E32Fixups::set_word(&mut bytes, base, imp.vaddr, (addend << 16) | ordinal)?;
        }
        E32Fixups::absolute(elf, &mut bytes, base)?;
        Ok(Self { bytes })
    }
}
