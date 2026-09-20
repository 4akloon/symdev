//! Tests for experiment 49 (EXE with `.data`/BSS sections).
use super::*;

fn experiment_49(uncompressed: bool) -> Elf2E32 {
    let mut a = args(&[
        "elf2e32",
        "--uid1=0x1000007a",
        "--uid3=0xecacde23",
        "--fpu=softvfp",
        "--targettype=EXE",
        "--output=counter.exe",
        "--elfinput=counter.elf",
        "--linkas=counter{000a0000}[ecacde23].exe",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]);
    if uncompressed {
        a.push("--uncompressed".into());
    }
    Elf2E32::from_args(&a).unwrap()
}

#[test]
fn experiment_49_data_section_images_match_elf2e32_next() {
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/counter.elf.hex"))).unwrap();
    let ordinals = ordinals_table(include_str!("../../testdata/counter_ordinals.txt"));
    // counter_u.exe / counter.exe header time (both runs within the same second).
    let u = unhex(include_str!("../../testdata/counter_uncompressed.exe.hex"));
    let time_of = |img: &[u8]| {
        let lo = u32::from_le_bytes(img[0x24..0x28].try_into().unwrap());
        let hi = u32::from_le_bytes(img[0x28..0x2c].try_into().unwrap());
        E32Time((u64::from(hi) << 32) | u64::from(lo))
    };
    let bytes = experiment_49(true)
        .encode_elf(&elf, &ordinals, time_of(&u))
        .unwrap();
    assert_eq!(bytes, u);
    let c = unhex(include_str!("../../testdata/counter.exe.hex"));
    let bytes = experiment_49(false)
        .encode_elf(&elf, &ordinals, time_of(&c))
        .unwrap();
    assert_eq!(bytes, c);
}
