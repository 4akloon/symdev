use symdev_uidcrc::UidCrc;

/// UID prefix of an E32 image (`iUid1`/`iUid2`/`iUid3` + uidcrc checksum).
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

    fn hello_headers() -> (E32ImageHeader, E32ImageHeaderJ, E32ImageHeaderV) {
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
            code_size: 0x144c,
            data_size: 0,
            heap_size_min: E32ImageHeader::HEAP_MIN,
            heap_size_max: E32ImageHeader::HEAP_MAX,
            stack_size: E32ImageHeader::STACK,
            bss_size: 4,
            entry_point: 0x1098,
            code_base: 0x8000,
            data_base: 0x40_0000,
            dll_ref_table_count: 2,
            export_dir_offset: 0,
            export_dir_count: 0,
            text_size: 0x144c,
            code_offset: E32ImageHeader::CODE_OFFSET,
            data_offset: 0,
            import_offset: 0x14e8,
            code_reloc_offset: 0x15b8,
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
            exception_descriptor: 0x10f5,
            spare2: 0,
            export_desc_size: 0,
            export_desc_type: E32ImageHeaderV::EXPORT_DESC_FULL_BITMAP,
        };
        (hdr, j, v)
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
