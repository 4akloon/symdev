//! Test-only builders: a minimal ELF32 little-endian ARM file with chosen sections, and an
//! `ar` archive of chosen members, for the in-place patches' tests.

/// One section of a [`TestElf`]: its type, entry size and contents.
pub(crate) struct TestSection {
    pub kind: u32,
    pub entsize: u32,
    pub data: Vec<u8>,
}

/// An ELF32 little-endian ARM file: the 52-byte header, the sections' contents in order,
/// then the section header table (index 0 the null section).
pub(crate) struct TestElf {
    pub e_type: u16,
    pub machine: u16,
    pub sections: Vec<TestSection>,
}

impl TestElf {
    pub const SHT_STRTAB: u32 = 3;
    pub const SHT_RELA: u32 = 4;
    pub const SHT_REL: u32 = 9;

    /// A relocatable ARM object (`ET_REL`).
    pub fn object(sections: Vec<TestSection>) -> Self {
        Self {
            e_type: 1,
            machine: 40,
            sections,
        }
    }

    /// `SHT_REL` with one 8-byte entry per `(offset, type)`, symbol 1.
    pub fn rel(entries: &[(u32, u8)]) -> TestSection {
        let mut data = Vec::new();
        for (offset, kind) in entries {
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&((1u32 << 8) | u32::from(*kind)).to_le_bytes());
        }
        TestSection {
            kind: Self::SHT_REL,
            entsize: 8,
            data,
        }
    }

    /// `SHT_RELA`: as [`Self::rel`] with a 4-byte addend after each entry.
    pub fn rela(entries: &[(u32, u8)]) -> TestSection {
        let mut data = Vec::new();
        for (offset, kind) in entries {
            data.extend_from_slice(&offset.to_le_bytes());
            data.extend_from_slice(&((1u32 << 8) | u32::from(*kind)).to_le_bytes());
            data.extend_from_slice(&0x29u32.to_le_bytes());
        }
        TestSection {
            kind: Self::SHT_RELA,
            entsize: 12,
            data,
        }
    }

    /// A string table holding exactly `data`.
    pub fn strtab(data: &[u8]) -> TestSection {
        TestSection {
            kind: Self::SHT_STRTAB,
            entsize: 0,
            data: data.to_vec(),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut out = vec![0u8; 52];
        let mut placed = Vec::new();
        for s in &self.sections {
            placed.push(out.len() as u32);
            out.extend_from_slice(&s.data);
        }
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        let shoff = out.len() as u32;
        out.extend_from_slice(&[0u8; 40]);
        for (s, offset) in self.sections.iter().zip(placed) {
            let mut sh = [0u8; 40];
            sh[4..8].copy_from_slice(&s.kind.to_le_bytes());
            sh[16..20].copy_from_slice(&offset.to_le_bytes());
            sh[20..24].copy_from_slice(&(s.data.len() as u32).to_le_bytes());
            sh[36..40].copy_from_slice(&s.entsize.to_le_bytes());
            out.extend_from_slice(&sh);
        }
        out[..4].copy_from_slice(b"\x7fELF");
        out[4] = 1;
        out[5] = 1;
        out[6] = 1;
        out[16..18].copy_from_slice(&self.e_type.to_le_bytes());
        out[18..20].copy_from_slice(&self.machine.to_le_bytes());
        out[0x20..0x24].copy_from_slice(&shoff.to_le_bytes());
        out[0x28..0x2a].copy_from_slice(&52u16.to_le_bytes());
        out[0x2e..0x30].copy_from_slice(&40u16.to_le_bytes());
        let shnum = self.sections.len() as u16 + 1;
        out[0x30..0x32].copy_from_slice(&shnum.to_le_bytes());
        out
    }

    /// A System V `ar` archive of `(name field, contents)`; odd members padded with `\n`.
    pub fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = b"!<arch>\n".to_vec();
        for (name, data) in members {
            let header = format!(
                "{name:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n",
                0,
                0,
                0,
                644,
                data.len()
            );
            assert_eq!(header.len(), 60);
            out.extend_from_slice(header.as_bytes());
            out.extend_from_slice(data);
            if data.len() % 2 == 1 {
                out.push(b'\n');
            }
        }
        out
    }
}
