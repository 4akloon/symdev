//! Test SDK files: a `.dso` whose string table is padded with spaces, and a `.lib` archive
//! whose one object has an `R_ARM_TARGET2`, both as small as an ELF can be.
use std::fs;
use std::path::{Path, PathBuf};

/// ELF32 little-endian ARM, `e_type`, one section of `kind`/`entsize` holding `data`.
pub fn elf(e_type: u16, kind: u32, entsize: u32, data: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; 52];
    out.extend_from_slice(data);
    while out.len() % 4 != 0 {
        out.push(0);
    }
    let shoff = out.len() as u32;
    out.extend_from_slice(&[0u8; 40]);
    let mut sh = [0u8; 40];
    sh[4..8].copy_from_slice(&kind.to_le_bytes());
    sh[16..20].copy_from_slice(&52u32.to_le_bytes());
    sh[20..24].copy_from_slice(&(data.len() as u32).to_le_bytes());
    sh[36..40].copy_from_slice(&entsize.to_le_bytes());
    out.extend_from_slice(&sh);
    out[..6].copy_from_slice(b"\x7fELF\x01\x01");
    out[16..18].copy_from_slice(&e_type.to_le_bytes());
    out[18..20].copy_from_slice(&40u16.to_le_bytes());
    out[0x20..0x24].copy_from_slice(&shoff.to_le_bytes());
    out[0x2e..0x30].copy_from_slice(&40u16.to_le_bytes());
    out[0x30..0x32].copy_from_slice(&2u16.to_le_bytes());
    out
}

/// A `.dso`: a shared object whose string table ends `pad`.
pub fn dso(pad: &[u8]) -> Vec<u8> {
    elf(3, 3, 0, &[b"\0euser\0".as_slice(), pad].concat())
}

/// An object with one `SHT_REL` entry of relocation type `kind`.
pub fn object(kind: u8) -> Vec<u8> {
    let mut rel = 0u32.to_le_bytes().to_vec();
    rel.extend_from_slice(&((1u32 << 8) | u32::from(kind)).to_le_bytes());
    elf(1, 9, 8, &rel)
}

/// A one-member `ar` archive.
pub fn archive(member: &[u8]) -> Vec<u8> {
    let header = format!(
        "{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
        "a.o/",
        0,
        0,
        0,
        644,
        member.len()
    );
    [b"!<arch>\n".as_slice(), header.as_bytes(), member].concat()
}

/// An SDK under `root` with `lib/euser.dso` (padded), `lib/avkon.dso` (not padded),
/// `urel/usrt2_2.lib` (one TARGET2) and `urel/eexe.lib` (none). Returns the EPOCROOT.
pub fn sdk(root: &Path) -> PathBuf {
    let armv5 = root.join("epoc32/release/armv5");
    fs::create_dir_all(armv5.join("lib")).unwrap();
    fs::create_dir_all(armv5.join("urel")).unwrap();
    fs::write(armv5.join("lib/euser.dso"), dso(b"  ")).unwrap();
    fs::write(armv5.join("lib/avkon.dso"), dso(b"")).unwrap();
    fs::write(armv5.join("urel/usrt2_2.lib"), archive(&object(41))).unwrap();
    fs::write(armv5.join("urel/eexe.lib"), archive(&object(2))).unwrap();
    root.to_path_buf()
}
