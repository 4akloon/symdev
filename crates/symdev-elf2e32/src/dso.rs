//! Import library (`.dso`) for a DLL: an ELF32 ARM shared object whose `ER_RO` holds
//! one ordinal word per export (experiment 52). Layout observed on elf2e32_next output;
//! `.hash` from `docs/research/dso-hash-spec.md` (clean-room).

use symdev_core::{Error, Result};

use crate::{E32ExportKind, E32Exports};

/// The `.dso` written next to a DLL (`--dso=<path>`).
pub struct E32Dso<'a> {
    /// `DT_SONAME` and the base version name: the `.dso` file name.
    pub soname: &'a str,
    /// `--linkas` (the DLL's versioned name, e.g. `mathlib{000a0000}[e5d1b001].dll`).
    pub linkas: &'a str,
    pub exports: &'a E32Exports,
}

impl E32Dso<'_> {
    const EHDR: usize = 52;
    const SHDR: usize = 40;
    const PHDR: usize = 32;
    const SECTIONS: usize = 9;
    const SHSTRTAB: &'static [u8] =
        b"\0ER_RO\0.dynamic\0.hash\0.version_d\0.version\0.strtab\0.dynsym\0.shstrtab\0";

    pub fn bytes(&self) -> Result<Vec<u8>> {
        let n = self.exports.entries.len();
        if n == 0 {
            return Err(Error::Other("DSO without exports".into()));
        }
        // .strtab: exports in ordinal order, then soname, then linkas.
        let names: Vec<String> = self.exports.entries.iter().map(|e| e.dso_name()).collect();
        let mut strtab = vec![0u8];
        let mut name_at = Vec::new();
        for name in &names {
            name_at.push(strtab.len() as u32);
            strtab.extend_from_slice(name.as_bytes());
            strtab.push(0);
        }
        let soname_at = strtab.len() as u32;
        strtab.extend_from_slice(self.soname.as_bytes());
        strtab.push(0);
        let linkas_at = strtab.len() as u32;
        strtab.extend_from_slice(self.linkas.as_bytes());
        strtab.push(0);
        // Zero-padded to a multiple of 4, padding counted in its size (experiment 52).
        strtab.resize(strtab.len().next_multiple_of(4), 0);

        // ER_RO: ordinal words 1..=n, then a zero word.
        let mut er_ro = Vec::new();
        for ordinal in 1..=n as u32 {
            er_ro.extend_from_slice(&ordinal.to_le_bytes());
        }
        er_ro.extend_from_slice(&0u32.to_le_bytes());

        let hash = Self::hash_table(&names);
        let verdef = Self::verdef(self.soname, soname_at, self.linkas, linkas_at);
        // .version: local for the null symbol, version 2 for every export.
        let mut versym = 0u16.to_le_bytes().to_vec();
        for _ in 0..n {
            versym.extend_from_slice(&2u16.to_le_bytes());
        }
        let mut dynsym = vec![0u8; 16];
        for (i, e) in self.exports.entries.iter().enumerate() {
            // Functions: STB_GLOBAL | STT_FUNC, size 4. Data: STT_OBJECT with the
            // .def size (experiment 54).
            let (info, size) = match e.kind {
                E32ExportKind::Function => (0x12u8, 4u32),
                E32ExportKind::Data(size) => (0x11, size),
            };
            dynsym.extend_from_slice(&name_at[i].to_le_bytes());
            dynsym.extend_from_slice(&(4 * i as u32).to_le_bytes());
            dynsym.extend_from_slice(&size.to_le_bytes());
            dynsym.extend_from_slice(&[info, 0]); // default visibility
            dynsym.extend_from_slice(&1u16.to_le_bytes()); // ER_RO
        }

        // Section contents after the headers, each at a 4-aligned offset.
        let base = Self::EHDR + Self::SECTIONS * Self::SHDR;
        let mut offsets = Vec::new();
        let mut at = base;
        let dynamic_len = 10 * 8;
        let sizes = [
            er_ro.len(),
            dynamic_len,
            hash.len(),
            verdef.len(),
            versym.len(),
            strtab.len(),
            dynsym.len(),
            Self::SHSTRTAB.len(),
        ];
        for size in sizes {
            at = at.next_multiple_of(4);
            offsets.push(at);
            at += size;
        }
        let phoff = at.next_multiple_of(4);
        let [
            o_ro,
            o_dyn,
            o_hash,
            o_verdef,
            o_versym,
            o_str,
            o_sym,
            o_shstr,
        ] = offsets[..]
        else {
            return Err(Error::Other("DSO layout".into()));
        };
        let dynamic = [
            (14u32, soname_at),             // DT_SONAME
            (6, o_sym as u32),              // DT_SYMTAB
            (11, 16),                       // DT_SYMENT
            (5, o_str as u32),              // DT_STRTAB
            (10, strtab.len() as u32),      // DT_STRSZ
            (0x6fff_fff0, o_versym as u32), // DT_VERSYM
            (0x6fff_fffc, o_verdef as u32), // DT_VERDEF
            (0x6fff_fffd, 2),               // DT_VERDEFNUM
            (4, o_hash as u32),             // DT_HASH
            (0, 0),                         // DT_NULL
        ];

        let mut out = vec![0u8; phoff + 2 * Self::PHDR];
        // ELF header.
        out[..16].copy_from_slice(b"\x7fELF\x01\x01\x01\0\0\0\0\0\0\0\0\0");
        let mut w = Writer { out: &mut out };
        w.u16(16, 3); // ET_DYN
        w.u16(18, 40); // EM_ARM
        w.u32(20, 1);
        w.u32(24, 0);
        w.u32(28, phoff as u32);
        w.u32(32, Self::EHDR as u32);
        w.u32(36, 0x0400_0004);
        w.u16(40, Self::EHDR as u16);
        w.u16(42, Self::PHDR as u16);
        w.u16(44, 2);
        w.u16(46, Self::SHDR as u16);
        w.u16(48, Self::SECTIONS as u16);
        w.u16(50, 8);
        // Section headers: name, type, flags, addr, offset, size, link, info, align, entsize.
        let shdrs: [[u32; 10]; 9] = [
            [0; 10],
            [1, 1, 6, 0, o_ro as u32, er_ro.len() as u32, 0, 0, 4, 0],
            [7, 6, 0, 0, o_dyn as u32, dynamic_len as u32, 6, 0, 4, 8],
            [0x10, 5, 0, 0, o_hash as u32, hash.len() as u32, 7, 0, 4, 0],
            [
                0x16,
                0x6fff_fffd,
                0,
                0,
                o_verdef as u32,
                verdef.len() as u32,
                6,
                2,
                4,
                8,
            ],
            [
                0x21,
                0x6fff_ffff,
                0,
                0,
                o_versym as u32,
                versym.len() as u32,
                7,
                0,
                2,
                2,
            ],
            [0x2a, 3, 0, 0, o_str as u32, strtab.len() as u32, 0, 0, 0, 1],
            [
                0x32,
                11,
                0,
                0,
                o_sym as u32,
                dynsym.len() as u32,
                6,
                1,
                4,
                16,
            ],
            [
                0x3a,
                3,
                0,
                0,
                o_shstr as u32,
                Self::SHSTRTAB.len() as u32,
                0,
                0,
                0,
                1,
            ],
        ];
        for (i, sh) in shdrs.iter().enumerate() {
            for (k, v) in sh.iter().enumerate() {
                w.u32(Self::EHDR + i * Self::SHDR + 4 * k, *v);
            }
        }
        w.put(o_ro, &er_ro);
        for (i, (tag, val)) in dynamic.iter().enumerate() {
            w.u32(o_dyn + 8 * i, *tag);
            w.u32(o_dyn + 8 * i + 4, *val);
        }
        w.put(o_hash, &hash);
        w.put(o_verdef, &verdef);
        w.put(o_versym, &versym);
        w.put(o_str, &strtab);
        w.put(o_sym, &dynsym);
        w.put(o_shstr, Self::SHSTRTAB);
        // Program headers: LOAD (ER_RO, X) and DYNAMIC (R).
        let phdrs: [[u32; 8]; 2] = [
            [
                1,
                o_ro as u32,
                0,
                0,
                er_ro.len() as u32,
                er_ro.len() as u32,
                0x8000_0001, // PF_X plus the top (processor-specific) bit, as observed
                4,
            ],
            [2, o_dyn as u32, 0, 0, dynamic_len as u32, 0, 4, 4],
        ];
        for (i, ph) in phdrs.iter().enumerate() {
            for (k, v) in ph.iter().enumerate() {
                w.u32(phoff + i * Self::PHDR + 4 * k, *v);
            }
        }
        Ok(out)
    }

    /// SysV `.hash` (dso-hash-spec.md): `nbucket = N/3 + N%3` with `N = n + 1`; each
    /// bucket holds its lowest symbol index, chains ascend.
    fn hash_table(symbols: &[String]) -> Vec<u8> {
        let nchain = symbols.len() + 1;
        let nbucket = nchain / 3 + nchain % 3;
        let mut bucket = vec![0u32; nbucket];
        let mut chain = vec![0u32; nchain];
        let mut tail = vec![0u32; nbucket];
        for (i, name) in symbols.iter().enumerate() {
            let index = i as u32 + 1;
            let b = Self::elf_hash(name.as_bytes()) as usize % nbucket;
            if bucket[b] == 0 {
                bucket[b] = index;
            } else {
                chain[tail[b] as usize] = index;
            }
            tail[b] = index;
        }
        let mut out = Vec::new();
        for v in [nbucket as u32, nchain as u32]
            .into_iter()
            .chain(bucket)
            .chain(chain)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }

    /// Two version definitions: the base (`soname`, `VER_FLG_BASE`) and the DLL's
    /// `linkas` version that every export carries.
    fn verdef(soname: &str, soname_at: u32, linkas: &str, linkas_at: u32) -> Vec<u8> {
        let mut out = Vec::new();
        for (flags, index, name, at, next) in [
            (1u16, 1u16, soname, soname_at, 0x1cu32),
            (0, 2, linkas, linkas_at, 0),
        ] {
            out.extend_from_slice(&1u16.to_le_bytes()); // vd_version
            out.extend_from_slice(&flags.to_le_bytes());
            out.extend_from_slice(&index.to_le_bytes());
            out.extend_from_slice(&1u16.to_le_bytes()); // vd_cnt
            out.extend_from_slice(&Self::elf_hash(name.as_bytes()).to_le_bytes());
            out.extend_from_slice(&0x14u32.to_le_bytes()); // vd_aux
            out.extend_from_slice(&next.to_le_bytes()); // vd_next
            out.extend_from_slice(&at.to_le_bytes()); // vda_name
            out.extend_from_slice(&0u32.to_le_bytes()); // vda_next
        }
        out
    }

    fn elf_hash(name: &[u8]) -> u32 {
        let mut h = 0u32;
        for &c in name {
            h = (h << 4).wrapping_add(u32::from(c));
            let g = h & 0xf000_0000;
            if g != 0 {
                h ^= g >> 24;
            }
            h &= !g;
        }
        h
    }
}

struct Writer<'a> {
    out: &'a mut [u8],
}

impl Writer<'_> {
    fn put(&mut self, at: usize, bytes: &[u8]) {
        self.out[at..at + bytes.len()].copy_from_slice(bytes);
    }

    fn u16(&mut self, at: usize, v: u16) {
        self.put(at, &v.to_le_bytes());
    }

    fn u32(&mut self, at: usize, v: u32) {
        self.put(at, &v.to_le_bytes());
    }
}
