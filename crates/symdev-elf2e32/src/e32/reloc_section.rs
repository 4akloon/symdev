//! `E32RelocSection`: the E32 code/data relocation section.
use super::imports::E32ImportSection;
use super::layout::E32Layout;
use crate::ElfImage;
use symdev_core::{Error, Result};

/// E32 code relocation section (experiment 44 uncompressed `hello.exe`): `u32 size`
/// (blocks only), `u32 count` (real entries), then per 4 KiB page `u32 page, u32 block
/// size, u16 entries` padded to 4 with a zero entry. Entry = kind << 12 | page offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32RelocSection {
    /// `(code offset, kind)` sorted by offset.
    pub entries: Vec<(u32, u16)>,
}

impl E32RelocSection {
    pub const KIND_TEXT: u16 = 1;
    pub const KIND_DATA: u16 = 2;
    pub const KIND_INFERRED: u16 = 3;
    const PAGE: u32 = 0x1000;

    /// Code relocations: local dynamic relocations whose word lies in the code segment;
    /// kind by which segment the target lies in (experiment 44).
    pub fn code_from_elf(elf: &ElfImage, layout: &E32Layout) -> Result<Self> {
        Self::from_elf(elf, layout, layout.code_base, layout.code_size)
    }

    /// Data relocations: the same for words in the initialised data (experiment 49).
    pub fn data_from_elf(elf: &ElfImage, layout: &E32Layout) -> Result<Self> {
        Self::from_elf(elf, layout, layout.data_base, layout.data_size)
    }

    fn from_elf(elf: &ElfImage, layout: &E32Layout, base: u32, len: u32) -> Result<Self> {
        let code = layout.code_base..layout.code_base + layout.code_size;
        let data = layout.data_base..layout.data_base + layout.data_size + layout.bss_size;
        let mut entries = Vec::new();
        for rel in elf.local_relocs()? {
            if !code.contains(&rel.vaddr) && !data.contains(&rel.vaddr) {
                return Err(Error::Other(format!(
                    "relocation at {:#x} outside code and data",
                    rel.vaddr
                )));
            }
            if !(base..base + len).contains(&rel.vaddr) {
                continue;
            }
            let kind = if code.contains(&rel.target) {
                Self::KIND_TEXT
            } else if data.contains(&rel.target) {
                Self::KIND_DATA
            } else {
                return Err(Error::Other(format!(
                    "relocation at {:#x} targets {:#x} outside code and data",
                    rel.vaddr, rel.target
                )));
            };
            entries.push((rel.vaddr - base, kind));
        }
        for imp in elf.import_relocs()? {
            if imp.symbol == E32ImportSection::PURE_VIRTUAL
                && (base..base + len).contains(&imp.vaddr)
            {
                entries.push((imp.vaddr - base, Self::KIND_INFERRED));
            }
        }
        entries.sort_unstable();
        Ok(Self { entries })
    }

    pub fn count(&self) -> u32 {
        self.entries.len() as u32
    }

    /// Bytes this section takes in the image: none when there are no entries.
    pub fn stored_len(&self) -> u32 {
        if self.entries.is_empty() {
            0
        } else {
            self.bytes().len() as u32
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut blocks = Vec::new();
        let mut i = 0;
        while i < self.entries.len() {
            let page = self.entries[i].0 & !(Self::PAGE - 1);
            let mut words = Vec::new();
            while i < self.entries.len() && self.entries[i].0 & !(Self::PAGE - 1) == page {
                let (offset, kind) = self.entries[i];
                words.push((kind << 12) | (offset - page) as u16);
                i += 1;
            }
            if words.len() % 2 != 0 {
                words.push(0);
            }
            blocks.extend_from_slice(&page.to_le_bytes());
            blocks.extend_from_slice(&((8 + 2 * words.len()) as u32).to_le_bytes());
            for w in words {
                blocks.extend_from_slice(&w.to_le_bytes());
            }
        }
        let mut out = (blocks.len() as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&self.count().to_le_bytes());
        out.extend_from_slice(&blocks);
        out
    }
}
