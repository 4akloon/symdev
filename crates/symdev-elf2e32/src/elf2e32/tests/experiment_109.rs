//! Tests for experiment 109: `hello` linked by LLVM lld (`rust-lld`), read through the five
//! lld-only rules. The expected E32 bytes are the image that ran in EKA2L1 (written by the
//! spike's elf2e32), not the SDK's elf2e32's, which never saw an lld ELF.
//! `hello_lld.elf.hex` is the spike's no-GCCE link relinked with `-z max-page-size=0x1000`
//! (22 KB instead of 79 KB of page padding); its E32 bytes are unchanged by that.
use super::*;
use crate::{E32Layout, E32RelocSection, ElfLinker};

fn lld_hello_elf() -> ElfImage {
    ElfImage::parse(lld_hello_bytes()).unwrap()
}

fn lld_hello_bytes() -> Vec<u8> {
    unhex(include_str!("../../testdata/hello_lld.elf.hex"))
}

fn lld_hello_ordinals() -> E32Ordinals {
    ordinals_table(include_str!("../../testdata/hello_lld_ordinals.txt"))
}

/// The ELF with `Linker: LLD` overwritten: the same bytes as any other linker's ELF.
fn lld_hello_without_comment() -> ElfImage {
    let mut bytes = lld_hello_bytes();
    let at = bytes.windows(11).position(|w| w == b"Linker: LLD").unwrap();
    bytes[at + 8..at + 11].copy_from_slice(b"XXX");
    ElfImage::parse(bytes).unwrap()
}

/// `GcceBuild`'s EXE argv for the spike's `hello` (UID3 0xef9f2cab, no capabilities).
fn lld_hello(uncompressed: bool) -> Elf2E32 {
    let mut a = args(&[
        "elf2e32",
        "--uid1=0x1000007a",
        "--uid3=0xef9f2cab",
        "--fpu=softvfp",
        "--targettype=EXE",
        "--output=hello.exe",
        "--elfinput=hello.elf",
        "--linkas=hello{000a0000}[ef9f2cab].exe",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]);
    if uncompressed {
        a.push("--uncompressed".into());
    }
    Elf2E32::from_args(&a).unwrap()
}

fn time_of(img: &[u8]) -> E32Time {
    let lo = u32::from_le_bytes(img[0x24..0x28].try_into().unwrap());
    let hi = u32::from_le_bytes(img[0x28..0x2c].try_into().unwrap());
    E32Time((u64::from(hi) << 32) | u64::from(lo))
}

fn uncompressed_image() -> Vec<u8> {
    let golden = unhex(include_str!(
        "../../testdata/hello_lld_uncompressed.exe.hex"
    ));
    lld_hello(true)
        .encode_elf(&lld_hello_elf(), &lld_hello_ordinals(), time_of(&golden))
        .unwrap()
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

#[test]
fn experiment_109_lld_hello_matches_the_image_that_ran_in_eka2l1() {
    let u = unhex(include_str!(
        "../../testdata/hello_lld_uncompressed.exe.hex"
    ));
    assert_eq!(uncompressed_image(), u);
    let c = unhex(include_str!("../../testdata/hello_lld.exe.hex"));
    let bytes = lld_hello(false)
        .encode_elf(&lld_hello_elf(), &lld_hello_ordinals(), time_of(&c))
        .unwrap();
    assert_eq!(bytes, c);
}

#[test]
fn lld_is_recognised_by_its_comment_and_gnu_elfs_are_not() {
    assert_eq!(lld_hello_elf().linker(), ElfLinker::Lld);
    assert_eq!(hello_elf().linker(), ElfLinker::Gnu);
    let counter = ElfImage::parse(unhex(include_str!("../../testdata/counter.elf.hex"))).unwrap();
    assert_eq!(counter.linker(), ElfLinker::Gnu);
    assert_eq!(lld_hello_without_comment().linker(), ElfLinker::Gnu);
}

#[test]
fn an_lld_elf_without_its_linker_comment_is_refused_as_before() {
    let err = lld_hello(true)
        .encode_elf(
            &lld_hello_without_comment(),
            &lld_hello_ordinals(),
            E32Time(0),
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("writable PT_LOAD (not observed)"), "{err}");
}

#[test]
fn without_its_comment_the_lld_elf_is_read_by_the_observed_rules() {
    let elf = lld_hello_without_comment();
    let slot = elf.import_relocs().unwrap();
    let slot = slot.iter().find(|r| r.vaddr == 0x83ec).unwrap();
    assert!(slot.addend_in_place);
    let local = elf.local_relocs().unwrap();
    let rel = local.iter().find(|r| r.vaddr == 0x8154).unwrap();
    assert_eq!(
        rel.target, 0,
        "symbol 0's value, as GNU's section symbols are read"
    );
    assert!(E32Layout::from_elf(&elf).is_err());
}

#[test]
fn rule_a_an_lld_plt_slot_holding_plt0_is_not_an_import_addend() {
    let elf = lld_hello_elf();
    let code = elf.segment_bytes(elf.code_segment().unwrap()).unwrap();
    assert_eq!(word(code, 0x3ec), 0x82c0, "lazy-binding value: PLT0");
    let imports = elf.import_relocs().unwrap();
    let jump = imports.iter().find(|r| r.vaddr == 0x83ec).unwrap();
    assert!(!jump.addend_in_place, "R_ARM_JUMP_SLOT resolves to S");
    let abs = imports.iter().find(|r| r.vaddr == 0x8490).unwrap();
    assert!(abs.addend_in_place, "R_ARM_ABS32 keeps S + A");
    // `User::HandleException` is euser ordinal 0x265, addend 0.
    assert_eq!(word(&uncompressed_image(), 0x9c + 0x3ec), 0x265);
}

#[test]
fn rule_b_no_writable_pt_load_is_empty_data_at_the_data_section() {
    let layout = E32Layout::from_elf(&lld_hello_elf()).unwrap();
    assert_eq!(
        (layout.data_base, layout.data_size, layout.bss_size),
        (0x40_0000, 0, 0)
    );
}

#[test]
fn rule_c_the_exception_descriptor_is_read_from_symtab() {
    let layout = E32Layout::from_elf(&lld_hello_elf()).unwrap();
    assert_eq!(layout.exception_descriptor, 0x254 | 1);
}

#[test]
fn rule_d_a_symbol_less_relative_targets_the_word_as_linked() {
    let local = lld_hello_elf().local_relocs().unwrap();
    let rel = local.iter().find(|r| r.vaddr == 0x8154).unwrap();
    assert_eq!((rel.target, rel.absolute), (0x843e, false));
}

#[test]
fn rule_e_a_target_one_past_the_code_is_a_code_relocation() {
    let elf = lld_hello_elf();
    let layout = E32Layout::from_elf(&elf).unwrap();
    assert_eq!(layout.code_base + layout.code_size, 0x84e0);
    let local = elf.local_relocs().unwrap();
    let limit = local.iter().find(|r| r.vaddr == 0x8258).unwrap();
    assert_eq!(limit.target, 0x84e0, ".ARM.exidx$$Limit");
    let relocs = E32RelocSection::code_from_elf(&elf, &layout).unwrap();
    assert!(
        relocs
            .entries
            .contains(&(0x258, E32RelocSection::KIND_TEXT))
    );
}
