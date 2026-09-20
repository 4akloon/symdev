//! `E32ImportBlock` / `E32ImportSection`: the E32 import section.
use crate::ElfImage;
use symdev_core::{Error, Result};

/// One DLL's block in the ELF-format import section: code offsets of the import slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32ImportBlock {
    pub dll: String,
    pub code_offsets: Vec<u32>,
}

/// E32 import section, `KImageImpFmt_ELF` layout (experiment 44 uncompressed `hello.exe`):
/// `u32 size`, per DLL `u32 name_offset, u32 count, count × u32 code offset`, then the
/// NUL-terminated names, padded to 4.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32ImportSection {
    pub blocks: Vec<E32ImportBlock>,
}

impl E32ImportSection {
    /// Not imported: slots stay as linked and get an inferred-kind code relocation
    /// (experiment 51: the only undefined symbol elf2e32_next treats this way).
    pub const PURE_VIRTUAL: &str = "__cxa_pure_virtual";

    /// DLL blocks sorted by DLL name (experiment 51); slots in `DT_REL` then
    /// `DT_JMPREL` order.
    pub fn from_elf(elf: &ElfImage, code_base: u32) -> Result<Self> {
        let relocs: Vec<_> = elf
            .import_relocs()?
            .into_iter()
            .filter(|r| r.symbol != Self::PURE_VIRTUAL)
            .collect();
        let mut dlls = elf.needed_dlls()?;
        dlls.sort();
        let mut blocks = Vec::new();
        for dll in dlls {
            let code_offsets = relocs
                .iter()
                .filter(|r| r.dll == dll)
                .map(|r| {
                    r.vaddr.checked_sub(code_base).ok_or_else(|| {
                        Error::Other(format!("import slot {:#x} below code base", r.vaddr))
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            if !code_offsets.is_empty() {
                blocks.push(E32ImportBlock { dll, code_offsets });
            }
        }
        Ok(Self { blocks })
    }

    /// `iDllRefTableCount`.
    pub fn dll_count(&self) -> u32 {
        self.blocks.len() as u32
    }

    pub fn bytes(&self) -> Vec<u8> {
        let table: usize = self
            .blocks
            .iter()
            .map(|b| 8 + 4 * b.code_offsets.len())
            .sum();
        let mut names = Vec::new();
        let mut body = Vec::new();
        for block in &self.blocks {
            let name_offset = (4 + table + names.len()) as u32;
            body.extend_from_slice(&name_offset.to_le_bytes());
            body.extend_from_slice(&(block.code_offsets.len() as u32).to_le_bytes());
            for off in &block.code_offsets {
                body.extend_from_slice(&off.to_le_bytes());
            }
            names.extend_from_slice(block.dll.as_bytes());
            names.push(0);
        }
        body.extend_from_slice(&names);
        while (4 + body.len()) % 4 != 0 {
            body.push(0);
        }
        let mut out = ((4 + body.len()) as u32).to_le_bytes().to_vec();
        out.extend_from_slice(&body);
        out
    }
}
