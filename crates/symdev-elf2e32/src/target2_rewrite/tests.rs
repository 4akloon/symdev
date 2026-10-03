use super::Target2Rewrite;
use crate::test_elf::{TestElf, TestSection};

const TARGET2: u8 = 41;
const ABS32: u8 = 2;
const PREL31: u8 = 42;

fn object(sections: Vec<TestSection>) -> Vec<u8> {
    TestElf::object(sections).bytes()
}

#[test]
fn rewrites_every_target2_of_an_object_and_nothing_else() {
    let before = object(vec![
        TestElf::rel(&[(0, TARGET2), (4, PREL31), (8, TARGET2)]),
        TestElf::rela(&[(0, TARGET2), (4, ABS32)]),
    ]);
    let want = object(vec![
        TestElf::rel(&[(0, ABS32), (4, PREL31), (8, ABS32)]),
        TestElf::rela(&[(0, ABS32), (4, ABS32)]),
    ]);
    let got = Target2Rewrite::object(&before).unwrap();
    assert_eq!(got.rewritten(), 3);
    assert_eq!(got.bytes(), want.as_slice());
    assert_eq!(got.into_bytes().len(), before.len());
}

#[test]
fn an_object_without_target2_comes_back_unchanged() {
    let before = object(vec![TestElf::rel(&[(0, ABS32), (4, PREL31)])]);
    let got = Target2Rewrite::object(&before).unwrap();
    assert_eq!(got.rewritten(), 0);
    assert_eq!(got.bytes(), before.as_slice());
}

#[test]
fn rewrites_the_elf_members_of_an_archive_in_place() {
    let with = object(vec![TestElf::rel(&[(0, TARGET2), (4, PREL31)])]);
    let without = object(vec![TestElf::rel(&[(0, PREL31)])]);
    let fixed = object(vec![TestElf::rel(&[(0, ABS32), (4, PREL31)])]);
    // A symbol table and a long-name table (not ELF, left alone), an odd-sized member so
    // the padding byte is exercised, and a member named through the long-name table.
    let symbols: &[u8] = b"\0\0\0\x01\0\0\0\x08f\0x";
    let names: &[u8] = b"a_rather_long_member_name.o/\n";
    let before = TestElf::archive(&[
        ("/", symbols),
        ("//", names),
        ("plain.o/", &without),
        ("/0", &with),
    ]);
    let want = TestElf::archive(&[
        ("/", symbols),
        ("//", names),
        ("plain.o/", &without),
        ("/0", &fixed),
    ]);
    let got = Target2Rewrite::archive(&before).unwrap();
    assert_eq!(got.rewritten(), 1);
    assert_eq!(got.bytes(), want.as_slice());
}

#[test]
fn refuses_what_is_not_a_relocatable_arm_object() {
    let err = |bytes: &[u8]| Target2Rewrite::object(bytes).err().unwrap().to_string();
    assert!(
        err(b"!<arch>\n").contains("not an ELF file"),
        "{}",
        err(b"!<arch>\n")
    );
    let mut shared = TestElf::object(vec![TestElf::rel(&[(0, TARGET2)])]);
    shared.e_type = 3;
    let e = err(&shared.bytes());
    assert!(e.contains("not a relocatable object"), "{e}");
    let mut x86 = TestElf::object(vec![]);
    x86.machine = 3;
    let e = err(&x86.bytes());
    assert!(e.contains("not ARM"), "{e}");
    let mut big = object(vec![TestElf::rel(&[(0, TARGET2)])]);
    big[5] = 2;
    assert!(err(&big).contains("little-endian"), "{}", err(&big));
}

#[test]
fn refuses_a_relocation_section_it_cannot_walk_and_changes_nothing() {
    let mut odd = TestElf::rel(&[(0, TARGET2)]);
    odd.data.truncate(6);
    let e = Target2Rewrite::object(&object(vec![odd])).err().unwrap();
    assert!(e.to_string().contains("whole number of entries"), "{e}");
    let mut wide = TestElf::rel(&[(0, TARGET2)]);
    wide.entsize = 12;
    let e = Target2Rewrite::object(&object(vec![wide])).err().unwrap();
    assert!(e.to_string().contains("entry size 12"), "{e}");
    let mut cut = object(vec![TestElf::rel(&[(0, TARGET2)])]);
    cut.truncate(cut.len() - 20);
    let e = Target2Rewrite::object(&cut).err().unwrap();
    assert!(e.to_string().contains("past the end"), "{e}");
}

#[test]
fn an_archive_error_names_the_member() {
    let text: &[u8] = b"not an object";
    let e = Target2Rewrite::archive(&TestElf::archive(&[("notes.txt/", text)]))
        .err()
        .unwrap()
        .to_string();
    assert!(
        e.contains("notes.txt/") && e.contains("not an ELF file"),
        "{e}"
    );
    let e = Target2Rewrite::archive(b"\x7fELF")
        .err()
        .unwrap()
        .to_string();
    assert!(e.contains("not an ar archive"), "{e}");
    let mut cut = TestElf::archive(&[("plain.o/", &object(vec![]))]);
    cut.truncate(cut.len() - 1);
    let e = Target2Rewrite::archive(&cut).err().unwrap().to_string();
    assert!(e.contains("past the end"), "{e}");
}

#[test]
fn a_malformed_section_header_table_is_refused_without_panicking() {
    let err = |bytes: &[u8]| Target2Rewrite::object(bytes).err().unwrap().to_string();
    let mut wide = object(vec![TestElf::rel(&[(0, TARGET2)])]);
    wide[0x2e] = 44;
    assert!(
        err(&wide).contains("section header size 44"),
        "{}",
        err(&wide)
    );
    let mut extended = object(vec![]);
    extended[0x30..0x32].copy_from_slice(&0u16.to_le_bytes());
    assert!(
        err(&extended).contains("extended section numbering"),
        "{}",
        err(&extended)
    );
    let mut far = object(vec![TestElf::rel(&[(0, TARGET2)])]);
    far[0x20..0x24].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(err(&far).contains("past the end"), "{}", err(&far));
}

#[test]
fn a_malformed_member_header_is_refused_and_long_names_are_named() {
    let err = |bytes: &[u8]| Target2Rewrite::archive(bytes).err().unwrap().to_string();
    let good = object(vec![]);
    let mut size = TestElf::archive(&[("plain.o/", &good)]);
    size[8 + 48..8 + 58].copy_from_slice(b"12x4      ");
    assert!(err(&size).contains("bad size field"), "{}", err(&size));
    let mut end = TestElf::archive(&[("plain.o/", &good)]);
    end[8 + 58] = b'!';
    assert!(err(&end).contains("bad header terminator"), "{}", err(&end));
    // A name past 15 characters lives in the `//` table; the error gives it, not `/0`.
    let text: &[u8] = b"not an object";
    let names: &[u8] = b"a_rather_long_member_name.o/\n";
    let long = TestElf::archive(&[("//", names), ("/0", text)]);
    let e = err(&long);
    assert!(e.contains("`a_rather_long_member_name.o`"), "{e}");
}
