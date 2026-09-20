//! `E32Dso`: the SysV `.hash` table and the two `.gnu.version_d` definitions.
use super::E32Dso;

impl E32Dso<'_> {
    /// SysV `.hash` (dso-hash-spec.md): `nbucket = N/3 + N%3` with `N = n + 1`; each
    /// bucket holds its lowest symbol index, chains ascend.
    pub(super) fn hash_table(symbols: &[String]) -> Vec<u8> {
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
    pub(super) fn verdef(soname: &str, soname_at: u32, linkas: &str, linkas_at: u32) -> Vec<u8> {
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
