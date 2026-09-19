use symdev_core::{Error, Result};

/// One `PT_LOAD` program header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfSegment {
    pub vaddr: u32,
    pub offset: u32,
    pub file_size: u32,
    pub mem_size: u32,
}

/// Little-endian ELF32 ARM image as linked for elf2e32 (the parts E32 needs).
#[derive(Debug)]
pub struct ElfImage {
    bytes: Vec<u8>,
    entry: u32,
    loads: Vec<(u32, ElfSegment)>,
    dynsym: Option<(usize, usize, usize, usize)>,
}

impl ElfImage {
    const PT_LOAD: u32 = 1;
    const PF_X: u32 = 1;
    const PF_W: u32 = 2;
    const SHT_DYNSYM: u32 = 11;
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
            dynsym: None,
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
            if elf.u32_at(sh + 4)? != Self::SHT_DYNSYM {
                continue;
            }
            let link = elf.u32_at(sh + 24)? as usize;
            let str_sh = shoff + link * shentsize;
            elf.dynsym = Some((
                elf.u32_at(sh + 16)? as usize,
                elf.u32_at(sh + 20)? as usize,
                elf.u32_at(str_sh + 16)? as usize,
                elf.u32_at(str_sh + 20)? as usize,
            ));
            break;
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

    /// Value of a `.dynsym` symbol by exact name.
    pub fn dynamic_symbol(&self, name: &str) -> Result<Option<u32>> {
        let Some((off, size, str_off, str_size)) = self.dynsym else {
            return Ok(None);
        };
        let strtab = self
            .bytes
            .get(str_off..str_off + str_size)
            .ok_or_else(|| Error::Other("ELF .dynstr out of range".into()))?;
        for sym in (off..off + size).step_by(16) {
            let name_off = self.u32_at(sym)? as usize;
            let rest = strtab
                .get(name_off..)
                .ok_or_else(|| Error::Other("ELF symbol name out of range".into()))?;
            let end = rest.iter().position(|&b| b == 0).unwrap_or(rest.len());
            if &rest[..end] == name.as_bytes() {
                return Ok(Some(self.u32_at(sym + 4)?));
            }
        }
        Ok(None)
    }

    fn load(&self, want: impl Fn(u32) -> bool) -> Option<ElfSegment> {
        self.loads
            .iter()
            .find(|(flags, _)| want(*flags))
            .map(|(_, seg)| *seg)
    }

    fn u16_at(&self, off: usize) -> Result<u16> {
        self.bytes
            .get(off..off + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .ok_or_else(|| Error::Other(format!("ELF truncated at {off:#x}")))
    }

    fn u32_at(&self, off: usize) -> Result<u32> {
        self.bytes
            .get(off..off + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| Error::Other(format!("ELF truncated at {off:#x}")))
    }
}
