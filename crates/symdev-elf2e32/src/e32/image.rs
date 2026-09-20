//! `E32Image`: a whole E32 image built from an ELF.
use super::code_section::E32CodeSection;
use super::data_section::E32DataSection;
use super::dll::E32Dll;
use super::header::{E32ImageHeader, E32ImageHeaderJ, E32ImageHeaderV};
use super::imports::E32ImportSection;
use super::layout::E32Layout;
use super::ordinals::E32Ordinals;
use super::reloc_section::E32RelocSection;
use super::time::E32Time;
use super::uid::E32Uid;
use crate::ElfImage;
use symdev_core::{Error, Result};

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
        Self::new(elf, uid, uid.uid3, caps, ordinals, time, None)
    }

    /// `secure_id`: the image's secure identity (`--sid`), UID3 unless the MMP or the
    /// manifest names another (experiment 66: only this field and the header CRC move).
    /// `dll`: `Some` for a DLL (export directory and description), `None` for an EXE.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        elf: &ElfImage,
        uid: E32Uid,
        secure_id: u32,
        caps: symdev_core::Capabilities,
        ordinals: &E32Ordinals,
        time: E32Time,
        dll: Option<E32Dll<'_>>,
    ) -> Result<Self> {
        let elf_layout = E32Layout::from_elf(elf)?;
        let mut code = E32CodeSection::from_elf(elf, &elf_layout, ordinals)?;
        let mut relocs = E32RelocSection::code_from_elf(elf, &elf_layout)?;
        let data_relocs = E32RelocSection::data_from_elf(elf, &elf_layout)?;
        let data = E32DataSection::from_elf(elf, &elf_layout)?;
        let imports = E32ImportSection::from_elf(elf, elf_layout.code_base)?;
        let mut layout = elf_layout;
        let mut v = E32ImageHeaderV {
            secure_id,
            vendor_id: 0,
            caps: caps.bits(),
            exception_descriptor: layout.exception_descriptor,
            spare2: 0,
            export_desc_type: E32ImageHeaderV::EXPORT_DESC_FULL_BITMAP,
            export_desc: Vec::new(),
        };
        let (flags, export_table_at, export_dir_count) = match dll {
            None => (E32ImageHeader::FLAGS_EXE_SOFTVFP, None, 0),
            Some(E32Dll {
                exports,
                allow_data,
            }) => {
                // elf2e32_next refuses a DLL with writable data unless --dlldata
                // (experiment 54: "contains initialized/uninitialized writable data").
                if !allow_data && (layout.data_size > 0 || layout.bss_size > 0) {
                    let which = if layout.data_size > 0 {
                        "initialized"
                    } else {
                        "uninitialized"
                    };
                    return Err(Error::Other(format!(
                        "DLL contains {which} writable data; add EPOCALLOWDLLDATA to the MMP \
                         (elf2e32 --dlldata)"
                    )));
                }
                if exports.entries.is_empty() {
                    return Err(Error::Other(
                        "TODO: DLL without exports (not observed)".into(),
                    ));
                }
                (v.export_desc_type, v.export_desc) = exports.description()?;
                let table_at = elf_layout.code_size;
                code.bytes.extend_from_slice(&exports.table());
                // Each export slot is relocated like any code pointer (experiments 52, 54).
                let data_end = layout.data_base + layout.data_size + layout.bss_size;
                for (i, e) in exports.entries.iter().enumerate() {
                    let kind = if (layout.data_base..data_end).contains(&e.address) {
                        E32RelocSection::KIND_DATA
                    } else {
                        E32RelocSection::KIND_TEXT
                    };
                    relocs.entries.push((table_at + 4 + 4 * i as u32, kind));
                }
                relocs.entries.sort_unstable();
                layout.code_size = code.bytes.len() as u32;
                (
                    E32ImageHeader::FLAGS_DLL_SOFTVFP,
                    Some(table_at),
                    exports.entries.len() as u32,
                )
            }
        };
        let code_offset = v.code_offset();
        let export_dir_offset = export_table_at.map_or(0, |at| code_offset + at + 4);
        let import_len = imports.bytes().len() as u32;
        let reloc_len = relocs.stored_len();
        let data_reloc_len = data_relocs.stored_len();
        let code_reloc_offset = layout.import_offset(code_offset) + import_len;
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
            flags,
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
            export_dir_offset,
            export_dir_count,
            text_size: layout.code_size,
            code_offset,
            data_offset: if layout.data_size == 0 {
                0
            } else {
                code_offset + layout.code_size
            },
            import_offset: layout.import_offset(code_offset),
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
