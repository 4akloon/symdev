//! Tests for experiment 52 (DLL exports, `.def`/`.dso` output).
use super::*;
use crate::E32ExportKind;

fn experiment_52(uncompressed: bool) -> Elf2E32 {
    let mut a = args(&[
        "elf2e32",
        "--sid=0xe5d1b001",
        "--uid1=0x10000079",
        "--uid2=0x1000008d",
        "--uid3=0xe5d1b001",
        "--fpu=softvfp",
        "--targettype=DLL",
        "--ignorenoncallable",
        "--output=mathlib.dll",
        "--dso=mathlib.dso",
        "--defoutput=mathlib.def",
        "--elfinput=mathlib.elf",
        "--linkas=mathlib{000a0000}[e5d1b001].dll",
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]);
    if uncompressed {
        a.push("--uncompressed".into());
    }
    Elf2E32::from_args(&a).unwrap()
}

#[test]
fn experiment_52_dll_image_def_and_dso_match_elf2e32_next() {
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp52_mathlib_elf.hex"))).unwrap();
    let time_of = |img: &[u8]| {
        let lo = u32::from_le_bytes(img[0x24..0x28].try_into().unwrap());
        let hi = u32::from_le_bytes(img[0x28..0x2c].try_into().unwrap());
        E32Time((u64::from(hi) << 32) | u64::from(lo))
    };
    let none = E32Ordinals::default();
    let u = unhex(include_str!("../../testdata/exp52_mathlib_u_dll.hex"));
    assert_eq!(
        experiment_52(true)
            .encode_elf(&elf, &none, time_of(&u))
            .unwrap(),
        u
    );
    let c = unhex(include_str!("../../testdata/exp52_mathlib_dll.hex"));
    assert_eq!(
        experiment_52(false)
            .encode_elf(&elf, &none, time_of(&c))
            .unwrap(),
        c
    );

    let exports = E32Exports::from_elf(&elf, None).unwrap();
    let names: Vec<&str> = exports.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["_Z7MathAbsi", "_Z7MathAddii", "_Z9MathTwicei"]);
    assert_eq!(
        exports.def_text(),
        include_str!("../../testdata/exp52_mathlib.def")
    );
    let dso = E32Dso {
        soname: "mathlib.dso",
        linkas: "mathlib{000a0000}[e5d1b001].dll",
        exports: &exports,
    }
    .bytes()
    .unwrap();
    assert_eq!(
        dso,
        unhex(include_str!("../../testdata/exp52_mathlib_dso.hex"))
    );
    // The DSO we write reads back as the ordinal table the EXE side consumes.
    let back = ElfImage::parse(dso).unwrap();
    assert_eq!(back.dso_ordinal("_Z9MathTwicei").unwrap(), Some(3));
}

#[test]
fn linker_markers_are_never_exports_with_or_without_ignorenoncallable() {
    // Experiment 54: elf2e32_next skips `_edata`, `__bss_start`, … (NOTYPE) and the
    // SHN_ABS version symbol either way.
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp52_mathlib_elf.hex"))).unwrap();
    let exports = E32Exports::from_elf(&elf, None).unwrap();
    assert_eq!(exports.entries.len(), 3);
    assert!(
        exports
            .entries
            .iter()
            .all(|e| e.kind == E32ExportKind::Function)
    );
}
