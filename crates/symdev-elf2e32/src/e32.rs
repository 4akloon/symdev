use super::ElfImage;
use symdev_core::{Error, Result};
use symdev_uidcrc::UidCrc;

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
    pub fn import_offset(&self) -> u32 {
        E32ImageHeader::CODE_OFFSET + self.code_size + self.data_size
    }
}

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

/// Import ordinals by `(dll, symbol)`, as the `--libpath` DSOs define them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct E32Ordinals {
    map: std::collections::BTreeMap<(String, String), u32>,
}

impl E32Ordinals {
    pub fn insert(&mut self, dll: impl Into<String>, symbol: impl Into<String>, ordinal: u32) {
        self.map.insert((dll.into(), symbol.into()), ordinal);
    }

    pub fn get(&self, dll: &str, symbol: &str) -> Option<u32> {
        self.map
            .get(&(dll.to_string(), symbol.to_string()))
            .copied()
    }
}

/// Word fixups inside one ELF segment's bytes (`base` = its link address).
struct E32Fixups;

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

    fn word(bytes: &[u8], base: u32, vaddr: u32) -> Result<u32> {
        let r = Self::slot(bytes, base, vaddr)?;
        Ok(u32::from_le_bytes([
            bytes[r.start],
            bytes[r.start + 1],
            bytes[r.start + 2],
            bytes[r.start + 3],
        ]))
    }

    fn set_word(bytes: &mut [u8], base: u32, vaddr: u32, value: u32) -> Result<()> {
        let r = Self::slot(bytes, base, vaddr)?;
        bytes[r].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    /// `R_ARM_ABS32` words in `[base, base + len)` become `S + A`, `R_ARM_GLOB_DAT`
    /// words `S`; `R_ARM_RELATIVE` words stay as linked (experiments 44, 49, 51).
    fn absolute(elf: &ElfImage, bytes: &mut [u8], base: u32) -> Result<()> {
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

/// UID prefix of an E32 image (`iUid1`/`iUid2`/`iUid3` + uidcrc checksum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E32Uid {
    pub uid1: u32,
    pub uid2: u32,
    pub uid3: u32,
}

impl E32Uid {
    /// Observed on experiment-6 `hello.exe` (`--targettype=EXE`, `--uid2` omitted).
    pub const EXE_UID2: u32 = 0;

    pub fn for_exe(uid1: u32, uid3: u32) -> Self {
        Self {
            uid1,
            uid2: Self::EXE_UID2,
            uid3,
        }
    }

    pub fn crc(&self) -> UidCrc {
        UidCrc::new(self.uid1, self.uid2, self.uid3)
    }

    pub fn bytes(&self) -> [u8; 16] {
        self.crc().bytes()
    }
}

/// E32 image header (`E32ImageHeader` as laid out by elf2e32_next).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct E32ImageHeader {
    pub uid: E32Uid,
    pub header_crc: u32,
    pub module_version: u32,
    pub compression_type: u32,
    pub tool_major: u8,
    pub tool_minor: u8,
    pub tool_build: i16,
    pub time_lo: u32,
    pub time_hi: u32,
    pub flags: u32,
    pub code_size: u32,
    pub data_size: u32,
    pub heap_size_min: i32,
    pub heap_size_max: i32,
    pub stack_size: i32,
    pub bss_size: i32,
    pub entry_point: u32,
    pub code_base: u32,
    pub data_base: u32,
    pub dll_ref_table_count: i32,
    pub export_dir_offset: u32,
    pub export_dir_count: u32,
    pub text_size: u32,
    pub code_offset: u32,
    pub data_offset: u32,
    pub import_offset: u32,
    pub code_reloc_offset: u32,
    pub data_reloc_offset: u32,
    pub process_priority: u16,
    pub cpu_identifier: u16,
}

impl E32ImageHeader {
    pub const SIGNATURE: [u8; 4] = *b"EPOC";
    pub const CRC_INITIALISER: u32 = 0xc90f_daa2;
    pub const COMPRESSION_DEFLATE: u32 = 0x101f_7afc;
    pub const TOOL_MAJOR: u8 = 3;
    pub const TOOL_MINOR: u8 = 0;
    pub const TOOL_BUILD: i16 = 2;
    pub const HEAP_MIN: i32 = 0x1000;
    pub const HEAP_MAX: i32 = 0x10_0000;
    pub const STACK: i32 = 0x2000;
    pub const PRIORITY_FOREGROUND: u16 = 350;
    pub const CPU_ARMV5: u16 = 0x2001;
    /// Recorded `--fpu=softvfp` (no HW-float bits) plus elf2e32_next EXE defaults:
    /// ELF imports, header format V, EKA2 entry, EABI, no-call-entry.
    pub const FLAGS_EXE_SOFTVFP: u32 = 0x1000_0000 | 0x0200_0000 | 0x20 | 0x8 | 0x2;
    pub const SIZE: usize = 124;
    pub const CODE_OFFSET: u32 = 156;

    pub fn bytes(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        let mut off = 0usize;
        put(&mut out, &mut off, &self.uid.bytes());
        put(&mut out, &mut off, &Self::SIGNATURE);
        put(&mut out, &mut off, &self.header_crc.to_le_bytes());
        put(&mut out, &mut off, &self.module_version.to_le_bytes());
        put(&mut out, &mut off, &self.compression_type.to_le_bytes());
        put(&mut out, &mut off, &[self.tool_major, self.tool_minor]);
        put(&mut out, &mut off, &self.tool_build.to_le_bytes());
        put(&mut out, &mut off, &self.time_lo.to_le_bytes());
        put(&mut out, &mut off, &self.time_hi.to_le_bytes());
        put(&mut out, &mut off, &self.flags.to_le_bytes());
        put(&mut out, &mut off, &self.code_size.to_le_bytes());
        put(&mut out, &mut off, &self.data_size.to_le_bytes());
        put(&mut out, &mut off, &self.heap_size_min.to_le_bytes());
        put(&mut out, &mut off, &self.heap_size_max.to_le_bytes());
        put(&mut out, &mut off, &self.stack_size.to_le_bytes());
        put(&mut out, &mut off, &self.bss_size.to_le_bytes());
        put(&mut out, &mut off, &self.entry_point.to_le_bytes());
        put(&mut out, &mut off, &self.code_base.to_le_bytes());
        put(&mut out, &mut off, &self.data_base.to_le_bytes());
        put(&mut out, &mut off, &self.dll_ref_table_count.to_le_bytes());
        put(&mut out, &mut off, &self.export_dir_offset.to_le_bytes());
        put(&mut out, &mut off, &self.export_dir_count.to_le_bytes());
        put(&mut out, &mut off, &self.text_size.to_le_bytes());
        put(&mut out, &mut off, &self.code_offset.to_le_bytes());
        put(&mut out, &mut off, &self.data_offset.to_le_bytes());
        put(&mut out, &mut off, &self.import_offset.to_le_bytes());
        put(&mut out, &mut off, &self.code_reloc_offset.to_le_bytes());
        put(&mut out, &mut off, &self.data_reloc_offset.to_le_bytes());
        put(&mut out, &mut off, &self.process_priority.to_le_bytes());
        put(&mut out, &mut off, &self.cpu_identifier.to_le_bytes());
        debug_assert_eq!(off, Self::SIZE);
        out
    }

    /// Uncompressed header (this + J + V) with `iHeaderCrc` stamped.
    pub fn uncompressed(
        &self,
        j: &E32ImageHeaderJ,
        v: &E32ImageHeaderV,
    ) -> [u8; Self::CODE_OFFSET as usize] {
        let mut out = [0u8; Self::CODE_OFFSET as usize];
        out[..Self::SIZE].copy_from_slice(&self.bytes());
        out[Self::SIZE..Self::SIZE + E32ImageHeaderJ::SIZE].copy_from_slice(&j.bytes());
        out[Self::SIZE + E32ImageHeaderJ::SIZE..].copy_from_slice(&v.bytes());
        out[20..24].copy_from_slice(&Self::CRC_INITIALISER.to_le_bytes());
        let n = usize::min(self.code_offset as usize, out.len());
        let crc = crc32(&out[..n]);
        out[20..24].copy_from_slice(&crc.to_le_bytes());
        out
    }
}

/// Compression tail (`E32ImageHeaderJ`).
pub struct E32ImageHeaderJ {
    pub uncompressed_size: u32,
}

impl E32ImageHeaderJ {
    pub const SIZE: usize = 4;

    pub fn bytes(&self) -> [u8; Self::SIZE] {
        self.uncompressed_size.to_le_bytes()
    }
}

/// Versioning tail (`E32ImageHeaderV`) with `iExportDesc[1]` pad when size is 0.
pub struct E32ImageHeaderV {
    pub secure_id: u32,
    pub vendor_id: u32,
    pub caps: u64,
    pub exception_descriptor: u32,
    pub spare2: u32,
    pub export_desc_size: u16,
    pub export_desc_type: u8,
}

impl E32ImageHeaderV {
    pub const SIZE: usize = 28;
    pub const EXPORT_DESC_FULL_BITMAP: u8 = 0x01;

    pub fn bytes(&self) -> [u8; Self::SIZE] {
        let mut out = [0u8; Self::SIZE];
        let mut off = 0usize;
        put(&mut out, &mut off, &self.secure_id.to_le_bytes());
        put(&mut out, &mut off, &self.vendor_id.to_le_bytes());
        put(&mut out, &mut off, &self.caps.to_le_bytes());
        put(&mut out, &mut off, &self.exception_descriptor.to_le_bytes());
        put(&mut out, &mut off, &self.spare2.to_le_bytes());
        put(&mut out, &mut off, &self.export_desc_size.to_le_bytes());
        out[off] = self.export_desc_type;
        out
    }
}

/// Symbian time: microseconds since 0001-01-01 UTC (`iTimeLo`/`iTimeHi`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct E32Time(pub u64);

impl E32Time {
    /// Microseconds from 0001-01-01 to 1970-01-01 (experiment 44: header time equals the
    /// file's mtime).
    const UNIX_EPOCH: u64 = 0x00dc_ddb3_0f2f_8000;

    pub fn from_system(t: std::time::SystemTime) -> Result<Self> {
        let since = t
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| Error::Other(format!("time before 1970: {e}")))?;
        Ok(Self(Self::UNIX_EPOCH + since.as_micros() as u64))
    }

    pub fn lo(&self) -> u32 {
        self.0 as u32
    }

    pub fn hi(&self) -> u32 {
        (self.0 >> 32) as u32
    }
}

/// A whole E32 image built from an ELF: header, code, imports, code relocations.
pub struct E32Image {
    pub header: E32ImageHeader,
    pub j: E32ImageHeaderJ,
    pub v: E32ImageHeaderV,
    pub code: E32CodeSection,
    pub data: E32DataSection,
    pub imports: E32ImportSection,
    pub relocs: E32RelocSection,
    pub data_relocs: E32RelocSection,
}

impl E32Image {
    /// elf2e32_next default `--version 10.0` (the `{000a0000}` in `--linkas`).
    pub const MODULE_VERSION: u32 = 0x000a_0000;

    /// EXE with the recorded elf2e32_next defaults (heap, stack, priority, ARMv5, softvfp).
    pub fn exe(
        elf: &ElfImage,
        uid: E32Uid,
        caps: symdev_core::Capabilities,
        ordinals: &E32Ordinals,
        time: E32Time,
    ) -> Result<Self> {
        let layout = E32Layout::from_elf(elf)?;
        let code = E32CodeSection::from_elf(elf, &layout, ordinals)?;
        let data = E32DataSection::from_elf(elf, &layout)?;
        let imports = E32ImportSection::from_elf(elf, layout.code_base)?;
        let relocs = E32RelocSection::code_from_elf(elf, &layout)?;
        let data_relocs = E32RelocSection::data_from_elf(elf, &layout)?;
        let import_len = imports.bytes().len() as u32;
        let reloc_len = relocs.stored_len();
        let data_reloc_len = data_relocs.stored_len();
        let code_reloc_offset = layout.import_offset() + import_len;
        let secure_id = uid.uid3;
        let header = E32ImageHeader {
            uid,
            header_crc: E32ImageHeader::CRC_INITIALISER,
            module_version: Self::MODULE_VERSION,
            compression_type: E32ImageHeader::COMPRESSION_DEFLATE,
            tool_major: E32ImageHeader::TOOL_MAJOR,
            tool_minor: E32ImageHeader::TOOL_MINOR,
            tool_build: E32ImageHeader::TOOL_BUILD,
            time_lo: time.lo(),
            time_hi: time.hi(),
            flags: E32ImageHeader::FLAGS_EXE_SOFTVFP,
            code_size: layout.code_size,
            data_size: layout.data_size,
            heap_size_min: E32ImageHeader::HEAP_MIN,
            heap_size_max: E32ImageHeader::HEAP_MAX,
            stack_size: E32ImageHeader::STACK,
            bss_size: layout.bss_size as i32,
            entry_point: layout.entry_point,
            code_base: layout.code_base,
            data_base: layout.data_base,
            dll_ref_table_count: imports.dll_count() as i32,
            export_dir_offset: 0,
            export_dir_count: 0,
            text_size: layout.code_size,
            code_offset: E32ImageHeader::CODE_OFFSET,
            data_offset: if layout.data_size == 0 {
                0
            } else {
                E32ImageHeader::CODE_OFFSET + layout.code_size
            },
            import_offset: layout.import_offset(),
            code_reloc_offset: if relocs.count() == 0 {
                0
            } else {
                code_reloc_offset
            },
            data_reloc_offset: if data_relocs.count() == 0 {
                0
            } else {
                code_reloc_offset + reloc_len
            },
            process_priority: E32ImageHeader::PRIORITY_FOREGROUND,
            cpu_identifier: E32ImageHeader::CPU_ARMV5,
        };
        let j = E32ImageHeaderJ {
            uncompressed_size: layout.code_size
                + layout.data_size
                + import_len
                + reloc_len
                + data_reloc_len,
        };
        let v = E32ImageHeaderV {
            secure_id,
            vendor_id: 0,
            caps: caps.bits(),
            exception_descriptor: layout.exception_descriptor,
            spare2: 0,
            export_desc_size: 0,
            export_desc_type: E32ImageHeaderV::EXPORT_DESC_FULL_BITMAP,
        };
        Ok(Self {
            header,
            j,
            v,
            code,
            data,
            imports,
            relocs,
            data_relocs,
        })
    }

    /// Default `elf2e32` output: header up to `iCodeOffset`, then the E32 deflate stream
    /// of the body (`docs/research/e32-deflate-spec.md` §1).
    pub fn compressed(&self) -> Result<Vec<u8>> {
        let header = E32ImageHeader {
            compression_type: E32ImageHeader::COMPRESSION_DEFLATE,
            ..self.header.clone()
        };
        let mut out = header.uncompressed(&self.j, &self.v).to_vec();
        out.extend_from_slice(&crate::E32Deflate::compress(&self.body())?);
        Ok(out)
    }

    fn body(&self) -> Vec<u8> {
        let mut out = self.code.bytes.clone();
        out.extend_from_slice(&self.data.bytes);
        out.extend_from_slice(&self.imports.bytes());
        for relocs in [&self.relocs, &self.data_relocs] {
            if relocs.count() > 0 {
                out.extend_from_slice(&relocs.bytes());
            }
        }
        out
    }

    /// `elf2e32 --uncompressed` output: `iCompressionType` 0, body stored as is.
    pub fn uncompressed(&self) -> Vec<u8> {
        let header = E32ImageHeader {
            compression_type: 0,
            ..self.header.clone()
        };
        let mut out = header.uncompressed(&self.j, &self.v).to_vec();
        out.extend_from_slice(&self.body());
        out
    }
}

fn put(out: &mut [u8], off: &mut usize, bytes: &[u8]) {
    let n = bytes.len();
    out[*off..*off + n].copy_from_slice(bytes);
    *off += n;
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_hex(s: &str) -> Vec<u8> {
        let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hello_exe() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello.exe.hex"))
    }

    fn hello_layout() -> E32Layout {
        let elf = ElfImage::parse(parse_hex(include_str!("testdata/hello.elf.hex"))).unwrap();
        E32Layout::from_elf(&elf).unwrap()
    }

    fn hello_elf() -> ElfImage {
        ElfImage::parse(parse_hex(include_str!("testdata/hello.elf.hex"))).unwrap()
    }

    fn hello_uncompressed() -> Vec<u8> {
        parse_hex(include_str!("testdata/hello_uncompressed.exe.hex"))
    }

    fn hello_imports() -> E32ImportSection {
        E32ImportSection::from_elf(&hello_elf(), 0x8000).unwrap()
    }

    fn hello_headers() -> (E32ImageHeader, E32ImageHeaderJ, E32ImageHeaderV) {
        let layout = hello_layout();
        let imports = hello_imports();
        let uid = E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9);
        let secure_id = uid.uid3;
        let hdr = E32ImageHeader {
            uid,
            header_crc: E32ImageHeader::CRC_INITIALISER,
            module_version: 0x000a_0000,
            compression_type: E32ImageHeader::COMPRESSION_DEFLATE,
            tool_major: E32ImageHeader::TOOL_MAJOR,
            tool_minor: E32ImageHeader::TOOL_MINOR,
            tool_build: E32ImageHeader::TOOL_BUILD,
            time_lo: 0x208d_5e00,
            time_hi: 0x00e3_3963,
            flags: E32ImageHeader::FLAGS_EXE_SOFTVFP,
            code_size: layout.code_size,
            data_size: layout.data_size,
            heap_size_min: E32ImageHeader::HEAP_MIN,
            heap_size_max: E32ImageHeader::HEAP_MAX,
            stack_size: E32ImageHeader::STACK,
            bss_size: layout.bss_size as i32,
            entry_point: layout.entry_point,
            code_base: layout.code_base,
            data_base: layout.data_base,
            dll_ref_table_count: imports.dll_count() as i32,
            export_dir_offset: 0,
            export_dir_count: 0,
            text_size: layout.code_size,
            code_offset: E32ImageHeader::CODE_OFFSET,
            data_offset: 0,
            import_offset: layout.import_offset(),
            code_reloc_offset: layout.import_offset() + imports.bytes().len() as u32,
            data_reloc_offset: 0,
            process_priority: E32ImageHeader::PRIORITY_FOREGROUND,
            cpu_identifier: E32ImageHeader::CPU_ARMV5,
        };
        let j = E32ImageHeaderJ {
            uncompressed_size: 0x1578,
        };
        let v = E32ImageHeaderV {
            secure_id,
            vendor_id: 0,
            caps: 0xbe000,
            exception_descriptor: layout.exception_descriptor,
            spare2: 0,
            export_desc_size: 0,
            export_desc_type: E32ImageHeaderV::EXPORT_DESC_FULL_BITMAP,
        };
        (hdr, j, v)
    }

    #[test]
    fn hello_elf_layout_matches_experiment_6_segments() {
        // readelf -lW hello.elf: LOAD R E 0x8000 filesz 0x144c; LOAD RW 0x400000 0/4;
        // entry 0x9098; dynsym Symbian$$CPP$$Exception$$Descriptor = 0x90f4.
        assert_eq!(
            hello_layout(),
            E32Layout {
                code_base: 0x8000,
                code_size: 0x144c,
                data_base: 0x40_0000,
                data_size: 0,
                bss_size: 4,
                entry_point: 0x1098,
                exception_descriptor: 0x10f5,
            }
        );
        assert_eq!(hello_layout().import_offset(), 0x14e8);
    }

    #[test]
    fn hello_import_section_matches_experiment_44() {
        let golden = hello_uncompressed();
        assert_eq!(golden.len(), 0x1614);
        let imports = hello_imports();
        assert_eq!(imports.dll_count(), 2);
        assert_eq!(imports.blocks[0].dll, "drtaeabi{000a0000}.dll");
        assert_eq!(imports.blocks[1].dll, "euser{000a0000}[100039e5].dll");
        assert_eq!(imports.bytes().as_slice(), &golden[0x14e8..0x15b8]);
    }

    fn hello_ordinals() -> E32Ordinals {
        let mut ordinals = E32Ordinals::default();
        for line in include_str!("testdata/hello_ordinals.txt").lines() {
            if line.starts_with('#') {
                continue;
            }
            let f: Vec<&str> = line.split_whitespace().collect();
            let ordinal = u32::from_str_radix(f[2].trim_start_matches("0x"), 16).unwrap();
            ordinals.insert(f[0], f[1], ordinal);
        }
        ordinals
    }

    #[test]
    fn hello_code_section_matches_experiment_44() {
        let golden = hello_uncompressed();
        let code =
            E32CodeSection::from_elf(&hello_elf(), &hello_layout(), &hello_ordinals()).unwrap();
        assert_eq!(code.bytes.len(), 0x144c);
        assert_eq!(code.bytes.as_slice(), &golden[0x9c..0x14e8]);
    }

    #[test]
    fn code_section_needs_every_import_ordinal() {
        let err = E32CodeSection::from_elf(&hello_elf(), &hello_layout(), &E32Ordinals::default())
            .unwrap_err()
            .to_string();
        assert!(err.contains("no ordinal"), "{err}");
    }

    #[test]
    fn hello_uncompressed_image_matches_experiment_44() {
        let golden = hello_uncompressed();
        let caps = symdev_core::Capabilities::from_names(&[
            "LocalServices",
            "NetworkServices",
            "ReadUserData",
            "WriteUserData",
            "UserEnvironment",
            "Location",
        ])
        .unwrap();
        // Experiment-44 hello_u.exe header time (2026-09-19 10:43:54 UTC).
        let time = E32Time(0x00e3_3986_c0a8_ae80);
        let image = E32Image::exe(
            &hello_elf(),
            E32Uid::for_exe(0x1000_007a, 0xe79e_4cf9),
            caps,
            &hello_ordinals(),
            time,
        )
        .unwrap();
        assert_eq!(image.uncompressed(), golden);
    }

    #[test]
    fn hello_body_deflate_matches_experiment_6_stream() {
        let body = &hello_uncompressed()[0x9c..];
        let stream = &hello_exe()[0x9c..];
        assert_eq!(body.len(), 0x1578);
        assert_eq!(stream.len(), 3432);
        assert_eq!(
            crate::E32Deflate::compress(body).unwrap().as_slice(),
            stream
        );
        assert_eq!(
            crate::E32Deflate::decompress(stream, body.len()).unwrap(),
            body
        );
    }

    #[test]
    fn e32_time_from_unix_epoch_matches_symbian_offset() {
        let t = E32Time::from_system(std::time::UNIX_EPOCH).unwrap();
        assert_eq!(t.0, 0x00dc_ddb3_0f2f_8000);
    }

    #[test]
    fn hello_code_relocs_match_experiment_44() {
        let golden = hello_uncompressed();
        let relocs = E32RelocSection::code_from_elf(&hello_elf(), &hello_layout()).unwrap();
        assert_eq!(relocs.count(), 32);
        assert_eq!(
            relocs
                .entries
                .iter()
                .filter(|(_, k)| *k == E32RelocSection::KIND_DATA)
                .count(),
            1
        );
        // Code relocations are the last section: 0x15b8 to end of file.
        assert_eq!(relocs.bytes().as_slice(), &golden[0x15b8..]);
    }

    #[test]
    fn hello_elf_needs_two_of_six_dsos() {
        // DT_NEEDED lists six DSOs; only drtaeabi and euser are versioned imports.
        assert_eq!(
            hello_elf().needed_dlls().unwrap(),
            ["drtaeabi{000a0000}.dll", "euser{000a0000}[100039e5].dll"]
        );
    }

    #[test]
    fn elf_parse_rejects_non_elf() {
        let err = ElfImage::parse(hello_exe()).unwrap_err().to_string();
        assert!(err.contains("ELF magic"), "{err}");
    }

    #[test]
    fn hello_exe_header_after_uid_is_epoc() {
        let golden = hello_exe();
        assert_eq!(&golden[16..20], b"EPOC");
        assert_eq!(E32ImageHeader::SIGNATURE, *b"EPOC");
        assert_eq!(&golden[16..20], &E32ImageHeader::SIGNATURE);
    }

    #[test]
    fn hello_exe_uncompressed_header_matches_experiment_6() {
        let golden = hello_exe();
        assert_eq!(golden.len(), 3588);
        let (hdr, j, v) = hello_headers();
        let bytes = hdr.uncompressed(&j, &v);
        assert_eq!(bytes.len(), 156);
        assert_eq!(&bytes[..16], &hdr.uid.bytes());
        assert_eq!(bytes.as_slice(), &golden[..156]);
    }
}
