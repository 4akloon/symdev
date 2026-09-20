//! `E32DataSection`: the E32 data section (initialised bytes with relocations rewritten).
use super::fixups::E32Fixups;
use super::layout::E32Layout;
use crate::ElfImage;
use symdev_core::Result;

/// E32 data section: the writable segment's initialised bytes with absolute
/// relocations rewritten (experiment 49). BSS is not stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32DataSection {
    pub bytes: Vec<u8>,
}

impl E32DataSection {
    pub fn from_elf(elf: &ElfImage, layout: &E32Layout) -> Result<Self> {
        let Some(seg) = elf.data_segment() else {
            return Ok(Self { bytes: Vec::new() });
        };
        let mut bytes = elf.segment_bytes(seg)?.to_vec();
        E32Fixups::absolute(elf, &mut bytes, layout.data_base)?;
        Ok(Self { bytes })
    }
}
