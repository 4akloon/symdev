//! `E32Layout`: header fields elf2e32 derives from the linked ELF.
use crate::ElfImage;
use symdev_core::{Error, Result};

/// E32 header fields elf2e32 derives from the linked ELF (experiment 6 `hello.elf`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E32Layout {
    pub code_base: u32,
    pub code_size: u32,
    pub data_base: u32,
    pub data_size: u32,
    pub bss_size: u32,
    pub entry_point: u32,
    pub exception_descriptor: u32,
}

impl E32Layout {
    pub const EXCEPTION_DESCRIPTOR_SYMBOL: &str = "Symbian$$CPP$$Exception$$Descriptor";

    pub fn from_elf(elf: &ElfImage) -> Result<Self> {
        let code = elf.code_segment()?;
        let data = elf.data_segment().ok_or_else(|| {
            Error::Other("TODO: ELF without a writable PT_LOAD (not observed)".into())
        })?;
        let entry_point = elf
            .entry()
            .checked_sub(code.vaddr)
            .ok_or_else(|| Error::Other(format!("ELF entry {:#x} below code base", elf.entry())))?;
        let descriptor = elf
            .dynamic_symbol(Self::EXCEPTION_DESCRIPTOR_SYMBOL)?
            .ok_or_else(|| {
                Error::Other(format!(
                    "TODO: ELF without {} (not observed)",
                    Self::EXCEPTION_DESCRIPTOR_SYMBOL
                ))
            })?;
        let descriptor = descriptor.checked_sub(code.vaddr).ok_or_else(|| {
            Error::Other(format!(
                "exception descriptor {descriptor:#x} below code base"
            ))
        })?;
        Ok(Self {
            code_base: code.vaddr,
            code_size: code.file_size,
            data_base: data.vaddr,
            data_size: data.file_size,
            bss_size: data.mem_size.saturating_sub(data.file_size),
            entry_point,
            // Low bit marks the descriptor present (experiment 6: 0x10f4 → 0x10f5).
            exception_descriptor: descriptor | 1,
        })
    }

    /// Import section follows code and data (`iImportOffset`).
    pub fn import_offset(&self, code_offset: u32) -> u32 {
        code_offset + self.code_size + self.data_size
    }
}
