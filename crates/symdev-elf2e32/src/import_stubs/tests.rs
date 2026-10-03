use super::*;
use crate::ElfImage;

fn unhex(hex: &str) -> Vec<u8> {
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(b[at..at + 2].try_into().unwrap())
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// A relocatable object read back: `(name, type, flags, offset, size, link, info)` per
/// section, and NUL-terminated strings.
struct Object(Vec<u8>);

impl Object {
    fn sections(&self) -> Vec<(String, u32, u32, usize, usize, u32, u32)> {
        let b = &self.0;
        let (shoff, shnum, shstrndx) = (
            u32_at(b, 0x20) as usize,
            u16_at(b, 0x30) as usize,
            u16_at(b, 0x32) as usize,
        );
        let raw: Vec<[u32; 10]> = (0..shnum)
            .map(|i| std::array::from_fn(|k| u32_at(b, shoff + 40 * i + 4 * k)))
            .collect();
        let names = raw[shstrndx][4] as usize;
        raw.iter()
            .map(|s| {
                let name = self.string(names + s[0] as usize);
                (name, s[1], s[2], s[4] as usize, s[5] as usize, s[6], s[7])
            })
            .collect()
    }

    fn section(&self, name: &str) -> (String, u32, u32, usize, usize, u32, u32) {
        self.sections().into_iter().find(|s| s.0 == name).unwrap()
    }

    fn string(&self, at: usize) -> String {
        let end = self.0[at..].iter().position(|&c| c == 0).unwrap();
        String::from_utf8(self.0[at..at + end].to_vec()).unwrap()
    }

    /// `(name, value, size, info, other, shndx)` per `.symtab` entry.
    fn symbols(&self) -> Vec<(String, u32, u32, u8, u8, u16)> {
        let (_, _, _, at, size, link, _) = self.section(".symtab");
        let strtab = self.sections()[link as usize].3;
        (at..at + size)
            .step_by(16)
            .map(|s| {
                let name = self.string(strtab + u32_at(&self.0, s) as usize);
                let (value, size) = (u32_at(&self.0, s + 4), u32_at(&self.0, s + 8));
                (
                    name,
                    value,
                    size,
                    self.0[s + 12],
                    self.0[s + 13],
                    u16_at(&self.0, s + 14),
                )
            })
            .collect()
    }
}

fn two() -> ImportStubs {
    ImportStubs::new(["_ZN4User4ExitEi", "__aeabi_memclr4"].map(String::from)).unwrap()
}

#[test]
fn the_object_is_an_arm_eabi5_relocatable() {
    let o = two().object();
    assert_eq!(&o[..16], b"\x7fELF\x01\x01\x01\0\0\0\0\0\0\0\0\0");
    assert_eq!(
        (u16_at(&o, 0x10), u16_at(&o, 0x12)),
        (1, 40),
        "ET_REL, EM_ARM"
    );
    assert_eq!(u32_at(&o, 0x24), 0x0500_0000, "EABI version 5");
}

#[test]
fn each_stub_is_ldr_pc_and_a_zero_word() {
    let o = Object(two().object());
    let (_, kind, flags, at, size, _, _) = o.section(ImportStubs::SECTION);
    assert_eq!((kind, flags, size), (1, 6, 16), "PROGBITS, AX, 2 × 8");
    let want = [0x04, 0xf0, 0x1f, 0xe5, 0, 0, 0, 0];
    assert_eq!(&o.0[at..at + 8], want);
    assert_eq!(&o.0[at + 8..at + 16], want);
}

#[test]
fn each_word_is_an_abs32_against_the_real_symbol() {
    let o = Object(two().object());
    let rel_name = format!(".rel{}", ImportStubs::SECTION);
    let (_, kind, _, at, size, link, info) = o.section(&rel_name);
    let sections = o.sections();
    assert_eq!(kind, 9, "SHT_REL");
    assert_eq!(sections[link as usize].0, ".symtab");
    assert_eq!(sections[info as usize].0, ImportStubs::SECTION);
    let symbols = o.symbols();
    let relocs: Vec<(u32, u32, String)> = (at..at + size)
        .step_by(8)
        .map(|r| {
            let info = u32_at(&o.0, r + 4);
            (
                u32_at(&o.0, r),
                info & 0xff,
                symbols[(info >> 8) as usize].0.clone(),
            )
        })
        .collect();
    assert_eq!(
        relocs,
        [
            (4, 2, "__real__ZN4User4ExitEi".to_string()),
            (12, 2, "__real___aeabi_memclr4".to_string()),
        ]
    );
}

#[test]
fn the_wrap_symbols_are_hidden_functions_and_the_real_ones_undefined() {
    let o = Object(two().object());
    let text = o
        .sections()
        .iter()
        .position(|s| s.0 == ImportStubs::SECTION)
        .unwrap() as u16;
    let symbols = o.symbols();
    let find = |name: &str| {
        let s = symbols.iter().find(|s| s.0 == name).unwrap();
        (s.1, s.2, s.3, s.4, s.5)
    };
    // STB_GLOBAL | STT_FUNC, STV_HIDDEN.
    assert_eq!(find("__wrap__ZN4User4ExitEi"), (0, 8, 0x12, 2, text));
    assert_eq!(find("__wrap___aeabi_memclr4"), (8, 8, 0x12, 2, text));
    // STB_GLOBAL | STT_NOTYPE, undefined.
    assert_eq!(find("__real___aeabi_memclr4"), (0, 0, 0x10, 0, 0));
}

#[test]
fn mapping_symbols_mark_code_and_data_and_locals_come_first() {
    let o = Object(two().object());
    let symbols = o.symbols();
    let maps: Vec<(&str, u32)> = symbols
        .iter()
        .filter(|s| s.0.starts_with('$'))
        .map(|s| (s.0.as_str(), s.1))
        .collect();
    assert_eq!(maps, [("$a", 0), ("$d", 4), ("$a", 8), ("$d", 12)]);
    let first_global = symbols.iter().position(|s| s.3 >> 4 != 0).unwrap();
    assert_eq!(o.section(".symtab").6 as usize, first_global, "sh_info");
    assert!(symbols[first_global..].iter().all(|s| s.3 >> 4 == 1));
}

#[test]
fn functions_are_sorted_and_unique() {
    let stubs = ImportStubs::new(["b", "a", "b"].map(String::from)).unwrap();
    assert_eq!(stubs.functions(), ["a", "b"]);
}

#[test]
fn an_empty_or_nul_name_is_refused() {
    assert!(ImportStubs::new([String::new()]).is_err());
    assert!(ImportStubs::new(["a\0b".to_string()]).is_err());
}

#[test]
fn the_first_link_names_the_functions_its_plt_calls() {
    let elf = ElfImage::parse(unhex(include_str!("../testdata/hello_lld.elf.hex"))).unwrap();
    let stubs = ImportStubs::from_first_link(&elf).unwrap();
    assert_eq!(stubs.functions().len(), 16);
    assert!(stubs.functions().iter().any(|f| f == "_ZN4User4ExitEi"));
    // An ABS32 import (a typeinfo the exception tables name) is data, not a call.
    assert!(
        !stubs
            .functions()
            .iter()
            .any(|f| f == "_ZTI15XLeaveException")
    );
}

#[test]
fn a_gnu_link_is_refused_as_a_first_link() {
    let elf = ElfImage::parse(unhex(include_str!("../testdata/hello.elf.hex"))).unwrap();
    let err = ImportStubs::from_first_link(&elf).unwrap_err().to_string();
    assert!(err.contains("lld"), "{err}");
}

#[test]
fn jump_slots_lists_every_plt_relocation_and_a_gnu_link_has_none() {
    // The check after the second link: any R_ARM_JUMP_SLOT left means a call the stubs
    // did not cover, and elf2e32 would refuse it.
    let lld = ElfImage::parse(unhex(include_str!("../testdata/hello_lld.elf.hex"))).unwrap();
    let slots = lld.jump_slots().unwrap();
    assert_eq!(slots.len(), 16);
    assert_eq!(slots, lld.plt_imports().unwrap());
    let gnu = ElfImage::parse(unhex(include_str!("../testdata/hello.elf.hex"))).unwrap();
    assert_eq!(gnu.jump_slots().unwrap(), Vec::<String>::new());
}
