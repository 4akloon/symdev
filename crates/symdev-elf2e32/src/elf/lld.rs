//! The five rules elf2e32 applies only to an ELF linked by LLVM lld (experiment 109 §3).
//!
//! Each rule is derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
//! 109); never observed from the SDK's elf2e32, which never saw an lld ELF. The owner
//! accepted them on 2026-10-03 as a narrow exception to "not observed → error", for ELFs
//! that `ElfLinker` recognises as lld's only: for every other ELF each rule answers as the
//! observed elf2e32 does, so GNU ld ELFs give the same E32 bytes as before.
use super::image::ElfImage;
use super::linker::ElfLinker;
use super::types::{ElfRel, ElfSegment};
use symdev_core::{Error, Result};

impl ElfLinker {
    /// Rule (a), derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
    /// 109); never observed from the SDK's elf2e32. Whether an import slot's word is its
    /// addend. GNU ld: always (observed). lld: only for `R_ARM_ABS32` (`S + A`);
    /// `R_ARM_GLOB_DAT` and `R_ARM_JUMP_SLOT` resolve to `S`, and lld initialises a
    /// `.got.plt` slot to PLT0's address for lazy binding, which is no addend.
    pub(super) fn import_addend_in_place(self, kind: u32) -> bool {
        match self {
            Self::Gnu => true,
            Self::Lld => kind == ElfImage::R_ARM_ABS32,
        }
    }

    /// Rule (e), derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
    /// 109); never observed from the SDK's elf2e32. Whether a relocation target lies in
    /// the code. lld: also the address one past its end — `.ARM.exidx$$Limit` in the
    /// exception descriptor, a symbol-less `R_ARM_RELATIVE` word; GNU ld names a section
    /// symbol inside the code for it instead.
    pub(crate) fn code_contains(self, code: &std::ops::Range<u32>, target: u32) -> bool {
        code.contains(&target) || (self == Self::Lld && target == code.end)
    }
}

impl ElfImage {
    const SHT_SYMTAB: u32 = 2;

    /// Rule (b), derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
    /// 109); never observed from the SDK's elf2e32. With no writable `PT_LOAD` (lld drops a
    /// zero-sized one; GNU ld always emits one at `-Tdata`), an lld ELF's data is empty
    /// and based at its `.data` output section. `None` for any other linker.
    pub(crate) fn lld_empty_data_segment(&self) -> Result<Option<ElfSegment>> {
        if self.linker != ElfLinker::Lld {
            return Ok(None);
        }
        let data = self.section_named(".data").ok_or_else(|| {
            Error::Other(
                "lld ELF has neither a writable PT_LOAD nor a .data section: link with a \
                 script that places .data at -Tdata"
                    .into(),
            )
        })?;
        Ok(Some(ElfSegment {
            vaddr: data.addr,
            offset: data.offset as u32,
            file_size: 0,
            mem_size: 0,
        }))
    }

    /// Rule (c), derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
    /// 109); never observed from the SDK's elf2e32. A hidden symbol of an lld ELF, read
    /// from `.symtab`: lld keeps hidden symbols out of `.dynsym`, where GNU ld 2.29.1 copies
    /// them as `LOCAL`. `None` for any other linker.
    pub(crate) fn lld_hidden_symbol(&self, name: &str) -> Result<Option<u32>> {
        if self.linker != ElfLinker::Lld {
            return Ok(None);
        }
        let Some(symtab) = self.section(Self::SHT_SYMTAB) else {
            return Ok(None);
        };
        for sym in (symtab.offset..symtab.offset + symtab.size).step_by(16) {
            if self.string(symtab.link, self.u32_at(sym)? as usize)? == name {
                return Ok(Some(self.u32_at(sym + 4)?));
            }
        }
        Ok(None)
    }

    /// Rule (d), derived from the ARM ABI / lld behaviour, verified in EKA2L1 (experiment
    /// 109); never observed from the SDK's elf2e32. lld writes `R_ARM_RELATIVE` with no
    /// symbol (`B + A`, the addend in place): its target is the word as linked. GNU ld names
    /// a section symbol in every one. `None` for any other relocation or linker.
    pub(super) fn lld_relative_target(&self, rel: &ElfRel) -> Result<Option<u32>> {
        if self.linker != ElfLinker::Lld || rel.symbol != 0 || rel.kind != Self::R_ARM_RELATIVE {
            return Ok(None);
        }
        let (_, seg) = self
            .loads
            .iter()
            .find(|(_, seg)| (seg.vaddr..seg.vaddr + seg.file_size).contains(&rel.vaddr))
            .ok_or_else(|| {
                Error::Other(format!(
                    "R_ARM_RELATIVE at {:#x} is outside every loaded segment",
                    rel.vaddr
                ))
            })?;
        self.u32_at((seg.offset + rel.vaddr - seg.vaddr) as usize)
            .map(Some)
    }
}
