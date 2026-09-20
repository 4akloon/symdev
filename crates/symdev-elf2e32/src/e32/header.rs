//! `E32ImageHeader` / `E32ImageHeaderJ` / `E32ImageHeaderV`: the three header parts.
use super::uid::E32Uid;

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
    /// The EXE flags plus the DLL bit (experiment 52: `0x1200002b`).
    pub const FLAGS_DLL_SOFTVFP: u32 = Self::FLAGS_EXE_SOFTVFP | 0x1;
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

    /// Uncompressed header (this + J + V, zero-padded to `iCodeOffset`) with
    /// `iHeaderCrc` stamped.
    pub fn uncompressed(&self, j: &E32ImageHeaderJ, v: &E32ImageHeaderV) -> Vec<u8> {
        let mut out = self.bytes().to_vec();
        out.extend_from_slice(&j.bytes());
        out.extend_from_slice(&v.bytes());
        out.resize(v.code_offset() as usize, 0);
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

/// Versioning tail (`E32ImageHeaderV`): fixed fields, then `iExportDesc` (at least the
/// one-byte pad when empty).
pub struct E32ImageHeaderV {
    pub secure_id: u32,
    pub vendor_id: u32,
    pub caps: u64,
    pub exception_descriptor: u32,
    pub spare2: u32,
    pub export_desc_type: u8,
    pub export_desc: Vec<u8>,
}

impl E32ImageHeaderV {
    pub const SIZE: usize = 28;
    pub const EXPORT_DESC_FULL_BITMAP: u8 = 0x01;
    /// DLL with every ordinal present (experiment 52).
    pub const EXPORT_DESC_NO_HOLES: u8 = 0x00;
    /// Absent ordinals, sparse presence bitmap (experiment 54).
    pub const EXPORT_DESC_SPARSE_BITMAP: u8 = 0x02;

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::SIZE);
        out.extend_from_slice(&self.secure_id.to_le_bytes());
        out.extend_from_slice(&self.vendor_id.to_le_bytes());
        out.extend_from_slice(&self.caps.to_le_bytes());
        out.extend_from_slice(&self.exception_descriptor.to_le_bytes());
        out.extend_from_slice(&self.spare2.to_le_bytes());
        out.extend_from_slice(&(self.export_desc.len() as u16).to_le_bytes());
        out.push(self.export_desc_type);
        out.extend_from_slice(&self.export_desc);
        if self.export_desc.is_empty() {
            out.push(0);
        }
        out
    }

    /// `iCodeOffset`: the whole header, rounded up to 4 (experiment 54: a 3-byte
    /// description moves the code from 0x9c to 0xa0).
    pub fn code_offset(&self) -> u32 {
        let len = E32ImageHeader::SIZE + E32ImageHeaderJ::SIZE + self.bytes().len();
        len.next_multiple_of(4) as u32
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
