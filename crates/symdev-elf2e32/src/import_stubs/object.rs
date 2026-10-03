//! `ImportStubs::object`: the stubs as an ELF32 ARM relocatable object.
use super::ImportStubs;

impl ImportStubs {
    /// `ldr pc, [pc, #-4]`: loads the word after it into `pc` (ARMv5T: interworking).
    const LDR_PC: u32 = 0xe51f_f004;
    const STUB: usize = 8;
    const EHDR: usize = 52;
    const SHDR: usize = 40;
    /// `EF_ARM_EABI_VER5`, as the GCCE objects it is linked with.
    const EF_ARM_EABI_VER5: u32 = 0x0500_0000;
    const R_ARM_ABS32: u32 = 2;
    const STB_LOCAL_NOTYPE: u8 = 0x00;
    const STB_GLOBAL_NOTYPE: u8 = 0x10;
    const STB_GLOBAL_FUNC: u8 = 0x12;
    const STV_HIDDEN: u8 = 2;
    /// Section indices: null, the stubs, their relocations, `.symtab`, `.strtab`,
    /// `.shstrtab`.
    const TEXT: u16 = 1;
    const SYMTAB: u32 = 3;
    const SECTIONS: usize = 6;

    /// The stubs as an ELF32 little-endian ARM relocatable object (`ET_REL`), written
    /// directly: no assembler, so no GCCE. Per function `f`, at `8 × i` in
    /// [`ImportStubs::SECTION`]: `ldr pc, [pc, #-4]` and a zero word with an `R_ARM_ABS32`
    /// against `__real_f`; `__wrap_f` is a hidden global function there, size 8. The ARM
    /// mapping symbols `$a` and `$d` mark the instruction and the word.
    pub fn object(&self) -> Vec<u8> {
        let n = self.functions.len();
        let mut text = Vec::with_capacity(Self::STUB * n);
        for _ in 0..n {
            text.extend_from_slice(&Self::LDR_PC.to_le_bytes());
            text.extend_from_slice(&0u32.to_le_bytes());
        }

        let mut strtab = b"\0$a\0$d\0".to_vec();
        let (map_a, map_d) = (1, 4);
        let mut symtab = vec![0u8; 16];
        let mut symbol = |name: u32, value: u32, size: u32, info: u8, other: u8, shndx: u16| {
            symtab.extend_from_slice(&name.to_le_bytes());
            symtab.extend_from_slice(&value.to_le_bytes());
            symtab.extend_from_slice(&size.to_le_bytes());
            symtab.extend_from_slice(&[info, other]);
            symtab.extend_from_slice(&shndx.to_le_bytes());
        };
        for i in 0..n as u32 {
            let at = Self::STUB as u32 * i;
            symbol(map_a, at, 0, Self::STB_LOCAL_NOTYPE, 0, Self::TEXT);
            symbol(map_d, at + 4, 0, Self::STB_LOCAL_NOTYPE, 0, Self::TEXT);
        }
        let first_global = 1 + 2 * n as u32;
        let mut rel = Vec::with_capacity(8 * n);
        for (i, f) in self.functions.iter().enumerate() {
            let at = (Self::STUB * i) as u32;
            let wrap = Self::append_name(&mut strtab, "__wrap_", f);
            let real = Self::append_name(&mut strtab, "__real_", f);
            let (info, other) = (Self::STB_GLOBAL_FUNC, Self::STV_HIDDEN);
            symbol(wrap, at, Self::STUB as u32, info, other, Self::TEXT);
            symbol(real, 0, 0, Self::STB_GLOBAL_NOTYPE, 0, 0);
            let real_index = first_global + 2 * i as u32 + 1;
            rel.extend_from_slice(&(at + 4).to_le_bytes());
            rel.extend_from_slice(&((real_index << 8) | Self::R_ARM_ABS32).to_le_bytes());
        }

        let mut shstrtab = b"\0".to_vec();
        let names = [
            Self::append_name(&mut shstrtab, "", Self::SECTION),
            Self::append_name(&mut shstrtab, ".rel", Self::SECTION),
            Self::append_name(&mut shstrtab, "", ".symtab"),
            Self::append_name(&mut shstrtab, "", ".strtab"),
            Self::append_name(&mut shstrtab, "", ".shstrtab"),
        ];
        // name, type, flags, link, info, align, entsize (sh_addr 0; offset, size placed).
        let headers: [[u32; 7]; 5] = [
            [names[0], 1, 6, 0, 0, 4, 0],
            [names[1], 9, 0x40, Self::SYMTAB, Self::TEXT.into(), 4, 8],
            [names[2], 2, 0, Self::SYMTAB + 1, first_global, 4, 16],
            [names[3], 3, 0, 0, 0, 1, 0],
            [names[4], 3, 0, 0, 0, 1, 0],
        ];
        let mut out = vec![0u8; Self::EHDR];
        let mut placed = Vec::new();
        for body in [&text, &rel, &symtab, &strtab, &shstrtab] {
            out.resize(out.len().next_multiple_of(4), 0);
            placed.push((out.len() as u32, body.len() as u32));
            out.extend_from_slice(body);
        }
        out.resize(out.len().next_multiple_of(4), 0);
        let shoff = out.len() as u32;
        out.extend_from_slice(&[0u8; Self::SHDR]);
        for (h, (offset, size)) in headers.iter().zip(placed) {
            let [name, kind, flags, link, info, align, entsize] = *h;
            for v in [
                name, kind, flags, 0, offset, size, link, info, align, entsize,
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        Self::write_header(&mut out, shoff);
        out
    }

    /// `prefix` and `name`, NUL-terminated, appended to a string table; returns the offset.
    fn append_name(table: &mut Vec<u8>, prefix: &str, name: &str) -> u32 {
        let at = table.len() as u32;
        table.extend_from_slice(prefix.as_bytes());
        table.extend_from_slice(name.as_bytes());
        table.push(0);
        at
    }

    fn write_header(out: &mut [u8], shoff: u32) {
        out[..16].copy_from_slice(b"\x7fELF\x01\x01\x01\0\0\0\0\0\0\0\0\0");
        let fields: [(usize, u32, usize); 8] = [
            (0x10, 1, 2),  // ET_REL
            (0x12, 40, 2), // EM_ARM
            (0x14, 1, 4),  // EV_CURRENT
            (0x20, shoff, 4),
            (0x24, Self::EF_ARM_EABI_VER5, 4),
            (0x28, Self::EHDR as u32, 2),
            (0x2e, Self::SHDR as u32, 2),
            (0x30, Self::SECTIONS as u32, 2),
        ];
        for (at, v, len) in fields {
            out[at..at + len].copy_from_slice(&v.to_le_bytes()[..len]);
        }
        let shstrndx = (Self::SECTIONS - 1) as u16;
        out[0x32..0x34].copy_from_slice(&shstrndx.to_le_bytes());
    }
}
