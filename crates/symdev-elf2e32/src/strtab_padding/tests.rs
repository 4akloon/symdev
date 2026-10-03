use super::StrtabPadding;
use crate::test_elf::TestElf;

fn dso(e_type: u16, strtabs: &[&[u8]]) -> Vec<u8> {
    let mut elf = TestElf::object(strtabs.iter().map(|s| TestElf::strtab(s)).collect());
    elf.e_type = e_type;
    elf.bytes()
}

#[test]
fn zeroes_what_follows_the_last_nul_of_every_string_table() {
    // 428 of the SDK's 570 `.dso` pad `.strtab` with 1–3 spaces after the last NUL, which
    // lld refuses as "non-null terminated" (experiment 109 §2, change 2).
    let before = dso(3, &[b"\0euser\0  ", b"\0.dynsym\0", b"\0a\0b\0 "]);
    let want = dso(3, &[b"\0euser\0\0\0", b"\0.dynsym\0", b"\0a\0b\0\0"]);
    let got = StrtabPadding::zero(&before).unwrap();
    assert_eq!(got.zeroed(), 3);
    assert_eq!(got.bytes(), want.as_slice());
    assert_eq!(got.into_bytes().len(), before.len());
}

#[test]
fn a_file_without_padding_comes_back_unchanged() {
    let before = dso(1, &[b"\0x\0", b""]);
    let got = StrtabPadding::zero(&before).unwrap();
    assert_eq!(got.zeroed(), 0);
    assert_eq!(got.bytes(), before.as_slice());
}

#[test]
fn refuses_a_string_table_with_no_nul_and_a_file_that_is_not_elf() {
    let e = StrtabPadding::zero(&dso(3, &[b"euser"])).err().unwrap();
    assert!(
        e.to_string().contains("section 1") && e.to_string().contains("no NUL"),
        "{e}"
    );
    let e = StrtabPadding::zero(b"!<arch>\n").err().unwrap();
    assert!(e.to_string().contains("not an ELF file"), "{e}");
}
