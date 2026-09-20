use super::*;
use crate::ElfImage;

fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn hello_exe() -> Vec<u8> {
    parse_hex(include_str!("../testdata/hello.exe.hex"))
}

fn hello_layout() -> E32Layout {
    let elf = ElfImage::parse(parse_hex(include_str!("../testdata/hello.elf.hex"))).unwrap();
    E32Layout::from_elf(&elf).unwrap()
}

fn hello_elf() -> ElfImage {
    ElfImage::parse(parse_hex(include_str!("../testdata/hello.elf.hex"))).unwrap()
}

fn hello_uncompressed() -> Vec<u8> {
    parse_hex(include_str!("../testdata/hello_uncompressed.exe.hex"))
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
        import_offset: layout.import_offset(E32ImageHeader::CODE_OFFSET),
        code_reloc_offset: layout.import_offset(E32ImageHeader::CODE_OFFSET)
            + imports.bytes().len() as u32,
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
        export_desc: Vec::new(),
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
    assert_eq!(
        hello_layout().import_offset(E32ImageHeader::CODE_OFFSET),
        0x14e8
    );
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
    for line in include_str!("../testdata/hello_ordinals.txt").lines() {
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
    let code = E32CodeSection::from_elf(&hello_elf(), &hello_layout(), &hello_ordinals()).unwrap();
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
