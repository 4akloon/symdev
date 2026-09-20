//! `ElfImage`: parsing and the code/data segments.
use super::types::{ElfSection, ElfSegment};
use symdev_core::{Error, Result};

/// Little-endian ELF32 ARM image as linked for elf2e32 (the parts E32 needs).
#[derive(Debug)]
pub struct ElfImage {
    pub(super) bytes: Vec<u8>,
    pub(super) entry: u32,
    pub(super) loads: Vec<(u32, ElfSegment)>,
    pub(super) sections: Vec<ElfSection>,
}

impl ElfImage {
    pub(super) const PT_LOAD: u32 = 1;
    pub(super) const PF_X: u32 = 1;
    pub(super) const PF_W: u32 = 2;
    pub(super) const SHT_DYNAMIC: u32 = 6;
    pub(super) const SHT_DYNSYM: u32 = 11;
    pub(super) const SHT_GNU_VERNEED: u32 = 0x6fff_fffe;
    pub(super) const SHT_GNU_VERSYM: u32 = 0x6fff_ffff;
    pub(super) const DT_NULL: u32 = 0;
    pub(super) const DT_PLTRELSZ: u32 = 2;
    pub(super) const DT_REL: u32 = 17;
    pub(super) const DT_RELSZ: u32 = 18;
    pub(super) const DT_JMPREL: u32 = 23;
    pub(super) const SHN_UNDEF: u16 = 0;
    pub(super) const STB_GLOBAL: u8 = 1;
    pub(super) const R_ARM_ABS32: u32 = 2;
    pub(super) const R_ARM_GLOB_DAT: u32 = 21;
    pub(super) const R_ARM_RELATIVE: u32 = 23;
    const EM_ARM: u16 = 40;

    pub fn parse(bytes: impl Into<Vec<u8>>) -> Result<Self> {
        let bytes = bytes.into();
        if bytes.get(..4) != Some(b"\x7fELF".as_slice()) {
            return Err(Error::Other("ELF magic missing".into()));
        }
        if bytes.get(4) != Some(&1) || bytes.get(5) != Some(&1) {
            return Err(Error::Other(
                "TODO: ELF other than 32-bit little-endian".into(),
            ));
        }
        let mut elf = Self {
            bytes,
            entry: 0,
            loads: Vec::new(),
            sections: Vec::new(),
        };
        if elf.u16_at(0x12)? != Self::EM_ARM {
            return Err(Error::Other("ELF machine is not ARM".into()));
        }
        elf.entry = elf.u32_at(0x18)?;
        let phoff = elf.u32_at(0x1c)? as usize;
        let shoff = elf.u32_at(0x20)? as usize;
        let phentsize = elf.u16_at(0x2a)? as usize;
        let phnum = elf.u16_at(0x2c)? as usize;
        let shentsize = elf.u16_at(0x2e)? as usize;
        let shnum = elf.u16_at(0x30)? as usize;

        for i in 0..phnum {
            let ph = phoff + i * phentsize;
            if elf.u32_at(ph)? != Self::PT_LOAD {
                continue;
            }
            let seg = ElfSegment {
                offset: elf.u32_at(ph + 4)?,
                vaddr: elf.u32_at(ph + 8)?,
                file_size: elf.u32_at(ph + 16)?,
                mem_size: elf.u32_at(ph + 20)?,
            };
            let flags = elf.u32_at(ph + 24)?;
            elf.loads.push((flags, seg));
        }

        for i in 0..shnum {
            let sh = shoff + i * shentsize;
            let section = ElfSection {
                kind: elf.u32_at(sh + 4)?,
                addr: elf.u32_at(sh + 12)?,
                offset: elf.u32_at(sh + 16)? as usize,
                size: elf.u32_at(sh + 20)? as usize,
                link: elf.u32_at(sh + 24)? as usize,
            };
            elf.sections.push(section);
        }
        Ok(elf)
    }

    pub fn entry(&self) -> u32 {
        self.entry
    }

    /// The executable `PT_LOAD` (E32 code section).
    pub fn code_segment(&self) -> Result<ElfSegment> {
        self.load(|flags| flags & Self::PF_X != 0)
            .ok_or_else(|| Error::Other("ELF has no executable PT_LOAD".into()))
    }

    /// The writable, non-executable `PT_LOAD` (E32 data + BSS).
    pub fn data_segment(&self) -> Option<ElfSegment> {
        self.load(|flags| flags & Self::PF_W != 0 && flags & Self::PF_X == 0)
    }

    /// File bytes of a segment (its `p_filesz` part).
    pub fn segment_bytes(&self, seg: ElfSegment) -> Result<&[u8]> {
        let start = seg.offset as usize;
        self.bytes
            .get(start..start + seg.file_size as usize)
            .ok_or_else(|| Error::Other(format!("ELF segment at {:#x} out of range", seg.vaddr)))
    }

    pub(super) fn load(&self, want: impl Fn(u32) -> bool) -> Option<ElfSegment> {
        self.loads
            .iter()
            .find(|(flags, _)| want(*flags))
            .map(|(_, seg)| *seg)
    }
}
