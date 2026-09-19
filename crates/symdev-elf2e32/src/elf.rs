use symdev_core::{Error, Result};

/// One `PT_LOAD` program header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfSegment {
    pub vaddr: u32,
    pub offset: u32,
    pub file_size: u32,
    pub mem_size: u32,
}

/// A dynamic relocation against an undefined symbol, tagged with the DLL (and the
/// DSO file) that `.gnu.version_r` says provides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfImportReloc {
    pub dll: String,
    pub dso: String,
    pub symbol: String,
    pub vaddr: u32,
}

/// A dynamic relocation against a defined symbol: the word at `vaddr` refers to `target`.
/// `absolute` words are rewritten to the symbol address: `R_ARM_ABS32` adds the
/// word already in place (`S + A`), `R_ARM_GLOB_DAT` does not (`S`, `addend_in_place`
/// false). `R_ARM_RELATIVE` words already hold the link-time address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElfLocalReloc {
    pub vaddr: u32,
    pub target: u32,
    pub absolute: bool,
    pub addend_in_place: bool,
}

/// One `.gnu.version_r` auxiliary entry.
#[derive(Debug, Clone)]
struct ElfVersion {
    index: u16,
    dll: String,
    dso: String,
}

#[derive(Debug, Clone, Copy)]
struct ElfRel {
    vaddr: u32,
    kind: u32,
    symbol: usize,
    symbol_name: usize,
    symbol_value: u32,
    symbol_section: u16,
}

#[derive(Debug, Clone, Copy)]
struct ElfSection {
    kind: u32,
    addr: u32,
    offset: usize,
    size: usize,
    link: usize,
}

/// A defined global `.dynsym` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElfSymbol {
    pub name: String,
    pub value: u32,
    pub size: u32,
    /// `STT_*`.
    pub kind: u8,
    /// `st_shndx`.
    pub section: u16,
}

impl ElfSymbol {
    const STT_OBJECT: u8 = 1;
    const STT_FUNC: u8 = 2;
    const SHN_ABS: u16 = 0xfff1;

    pub fn is_function(&self) -> bool {
        self.kind == Self::STT_FUNC
    }

    pub fn is_object(&self) -> bool {
        self.kind == Self::STT_OBJECT
    }

    /// `SHN_ABS` (linker-made markers such as the version-name symbol).
    pub fn is_absolute(&self) -> bool {
        self.section == Self::SHN_ABS
    }
}

/// Little-endian ELF32 ARM image as linked for elf2e32 (the parts E32 needs).
#[derive(Debug)]
pub struct ElfImage {
    bytes: Vec<u8>,
    entry: u32,
    loads: Vec<(u32, ElfSegment)>,
    sections: Vec<ElfSection>,
}

impl ElfImage {
    const PT_LOAD: u32 = 1;
    const PF_X: u32 = 1;
    const PF_W: u32 = 2;
    const SHT_DYNAMIC: u32 = 6;
    const SHT_DYNSYM: u32 = 11;
    const SHT_GNU_VERNEED: u32 = 0x6fff_fffe;
    const SHT_GNU_VERSYM: u32 = 0x6fff_ffff;
    const DT_NULL: u32 = 0;
    const DT_PLTRELSZ: u32 = 2;
    const DT_REL: u32 = 17;
    const DT_RELSZ: u32 = 18;
    const DT_JMPREL: u32 = 23;
    const SHN_UNDEF: u16 = 0;
    const STB_GLOBAL: u8 = 1;
    const R_ARM_ABS32: u32 = 2;
    const R_ARM_GLOB_DAT: u32 = 21;
    const R_ARM_RELATIVE: u32 = 23;
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

    /// Value of a `.dynsym` symbol by exact name.
    pub fn dynamic_symbol(&self, name: &str) -> Result<Option<u32>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(None);
        };
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            if self.string(dynsym.link, self.u32_at(sym)? as usize)? == name {
                return Ok(Some(self.u32_at(sym + 4)?));
            }
        }
        Ok(None)
    }

    /// Defined `STB_GLOBAL` `.dynsym` entries, in `.dynsym` order.
    pub fn exported_symbols(&self) -> Result<Vec<ElfSymbol>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            let info = *self
                .bytes
                .get(sym + 12)
                .ok_or_else(|| Error::Other("ELF .dynsym truncated".into()))?;
            let shndx = self.u16_at(sym + 14)?;
            if shndx == Self::SHN_UNDEF || info >> 4 != Self::STB_GLOBAL {
                continue;
            }
            out.push(ElfSymbol {
                name: self.string(dynsym.link, self.u32_at(sym)? as usize)?,
                value: self.u32_at(sym + 4)?,
                size: self.u32_at(sym + 8)?,
                kind: info & 0xf,
                section: shndx,
            });
        }
        Ok(out)
    }

    /// DLL names from `.gnu.version_r`, in section order.
    pub fn needed_dlls(&self) -> Result<Vec<String>> {
        Ok(self.versions()?.into_iter().map(|v| v.dll).collect())
    }

    /// File bytes of a segment (its `p_filesz` part).
    pub fn segment_bytes(&self, seg: ElfSegment) -> Result<&[u8]> {
        let start = seg.offset as usize;
        self.bytes
            .get(start..start + seg.file_size as usize)
            .ok_or_else(|| Error::Other(format!("ELF segment at {:#x} out of range", seg.vaddr)))
    }

    /// For a Symbian DSO: the ordinal an exported symbol names, i.e. the word its
    /// `.dynsym` value points at in its section (experiment 44: `ER_RO` ordinal table).
    pub fn dso_ordinal(&self, name: &str) -> Result<Option<u32>> {
        let Some(dynsym) = self.section(Self::SHT_DYNSYM) else {
            return Ok(None);
        };
        for sym in (dynsym.offset..dynsym.offset + dynsym.size).step_by(16) {
            let shndx = self.u16_at(sym + 14)?;
            if shndx == Self::SHN_UNDEF
                || self.string(dynsym.link, self.u32_at(sym)? as usize)? != name
            {
                continue;
            }
            let section = self
                .sections
                .get(shndx as usize)
                .ok_or_else(|| Error::Other(format!("DSO symbol {name} in missing section")))?;
            let value = self.u32_at(sym + 4)?;
            let at = value
                .checked_sub(section.addr)
                .ok_or_else(|| Error::Other(format!("DSO symbol {name} below its section")))?;
            return Ok(Some(self.u32_at(section.offset + at as usize)?));
        }
        Ok(None)
    }

    /// `DT_REL` then `DT_JMPREL` relocations whose symbol is undefined, in file order.
    pub fn import_relocs(&self) -> Result<Vec<ElfImportReloc>> {
        let versym = self
            .section(Self::SHT_GNU_VERSYM)
            .ok_or_else(|| Error::Other("ELF has no .gnu.version".into()))?;
        let versions = self.versions()?;
        let mut out = Vec::new();
        for rel in self.dynamic_relocs()? {
            if rel.symbol == 0 || rel.symbol_section != Self::SHN_UNDEF {
                continue;
            }
            let index = self.u16_at(versym.offset + rel.symbol * 2)? & 0x7fff;
            let version = versions.iter().find(|v| v.index == index).ok_or_else(|| {
                Error::Other(format!(
                    "ELF import at {:#x} has no version {index}",
                    rel.vaddr
                ))
            })?;
            let dynsym = self
                .section(Self::SHT_DYNSYM)
                .ok_or_else(|| Error::Other("ELF has no .dynsym".into()))?;
            out.push(ElfImportReloc {
                dll: version.dll.clone(),
                dso: version.dso.clone(),
                symbol: self.string(dynsym.link, rel.symbol_name)?,
                vaddr: rel.vaddr,
            });
        }
        Ok(out)
    }

    /// Dynamic relocations against defined symbols (the image's own fixups), in file order.
    pub fn local_relocs(&self) -> Result<Vec<ElfLocalReloc>> {
        let mut out = Vec::new();
        for rel in self.dynamic_relocs()? {
            if rel.symbol != 0 && rel.symbol_section == Self::SHN_UNDEF {
                continue;
            }
            if !matches!(
                rel.kind,
                Self::R_ARM_ABS32 | Self::R_ARM_GLOB_DAT | Self::R_ARM_RELATIVE
            ) {
                return Err(Error::Other(format!(
                    "TODO: ARM relocation type {} at {:#x} (not observed)",
                    rel.kind, rel.vaddr
                )));
            }
            out.push(ElfLocalReloc {
                vaddr: rel.vaddr,
                target: rel.symbol_value,
                absolute: rel.kind != Self::R_ARM_RELATIVE,
                addend_in_place: rel.kind == Self::R_ARM_ABS32,
            });
        }
        Ok(out)
    }

    /// `DT_REL` then `DT_JMPREL` entries. `DT_RELSZ` may already span the PLT
    /// relocations (experiment-6 hello.elf); each entry counts once.
    fn dynamic_relocs(&self) -> Result<Vec<ElfRel>> {
        let dynsym = self
            .section(Self::SHT_DYNSYM)
            .ok_or_else(|| Error::Other("ELF has no .dynsym".into()))?;
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (start, size) in [
            (Self::DT_REL, Self::DT_RELSZ),
            (Self::DT_JMPREL, Self::DT_PLTRELSZ),
        ] {
            let (Some(at), Some(len)) = (self.dynamic(start)?, self.dynamic(size)?) else {
                continue;
            };
            for rel in self.file_range(at, len)?.step_by(8) {
                if !seen.insert(rel) {
                    continue;
                }
                let info = self.u32_at(rel + 4)?;
                let symbol = (info >> 8) as usize;
                let entry = dynsym.offset + symbol * 16;
                out.push(ElfRel {
                    vaddr: self.u32_at(rel)?,
                    kind: info & 0xff,
                    symbol,
                    symbol_name: self.u32_at(entry)? as usize,
                    symbol_value: self.u32_at(entry + 4)?,
                    symbol_section: self.u16_at(entry + 14)?,
                });
            }
        }
        Ok(out)
    }

    /// Auxiliary entries of `.gnu.version_r`, in section order.
    fn versions(&self) -> Result<Vec<ElfVersion>> {
        let Some(verneed) = self.section(Self::SHT_GNU_VERNEED) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        let mut need = verneed.offset;
        loop {
            let count = self.u16_at(need + 2)? as usize;
            let dso = self.string(verneed.link, self.u32_at(need + 4)? as usize)?;
            let mut aux = need + self.u32_at(need + 8)? as usize;
            for _ in 0..count {
                out.push(ElfVersion {
                    index: self.u16_at(aux + 6)?,
                    dll: self.string(verneed.link, self.u32_at(aux + 8)? as usize)?,
                    dso: dso.clone(),
                });
                let next = self.u32_at(aux + 12)? as usize;
                if next == 0 {
                    break;
                }
                aux += next;
            }
            let next = self.u32_at(need + 12)? as usize;
            if next == 0 {
                break;
            }
            need += next;
        }
        Ok(out)
    }

    fn dynamic(&self, tag: u32) -> Result<Option<u32>> {
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
    fn file_range(&self, at: u32, len: u32) -> Result<std::ops::Range<usize>> {
        let section = self
            .sections
            .iter()
            .find(|s| (s.addr != 0 && s.addr == at) || (s.addr == 0 && s.offset == at as usize))
            .ok_or_else(|| Error::Other(format!("ELF dynamic pointer {at:#x} has no section")))?;
        Ok(section.offset..section.offset + len as usize)
    }

    fn section(&self, kind: u32) -> Option<ElfSection> {
        self.sections.iter().copied().find(|s| s.kind == kind)
    }

    fn string(&self, strtab: usize, at: usize) -> Result<String> {
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
