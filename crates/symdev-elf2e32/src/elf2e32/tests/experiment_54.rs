//! Tests for experiment 54 (frozen `.def`, ABSENT ordinals, data exports).
use super::*;
use crate::E32ImageHeaderV;

fn time_of(img: &[u8]) -> E32Time {
    let lo = u32::from_le_bytes(img[0x24..0x28].try_into().unwrap());
    let hi = u32::from_le_bytes(img[0x28..0x2c].try_into().unwrap());
    E32Time((u64::from(hi) << 32) | u64::from(lo))
}

/// Experiment 54 DLL job (compressed, as the SDK recipe runs it).
fn experiment_54(uid3: &str, base: &str, dlldata: bool) -> Elf2E32 {
    let mut a = args(&[
        "elf2e32",
        &format!("--sid=0x{uid3}"),
        "--uid1=0x10000079",
        "--uid2=0x1000008d",
        &format!("--uid3=0x{uid3}"),
        "--fpu=softvfp",
        "--targettype=DLL",
        &format!("--output={base}.dll"),
        &format!("--elfinput={base}.elf"),
        &format!("--linkas={base}{{000a0000}}[{uid3}].dll"),
        "--libpath=/sdk/epoc32/release/armv5/lib",
    ]);
    if dlldata {
        a.push("--dlldata".into());
    }
    Elf2E32::from_args(&a).unwrap()
}

/// Encode with a frozen `.def` and check image, `.def` and (optionally) `.dso`.
fn check_experiment_54(
    job: &Elf2E32,
    elf: &ElfImage,
    def_in: Option<&str>,
    golden: (&str, &str, Option<&str>),
    soname: &str,
) {
    check_experiment_54_with(job, elf, &E32Ordinals::default(), def_in, golden, soname);
}

fn check_experiment_54_with(
    job: &Elf2E32,
    elf: &ElfImage,
    ordinals: &E32Ordinals,
    def_in: Option<&str>,
    golden: (&str, &str, Option<&str>),
    soname: &str,
) {
    let frozen = def_in.map(|t| E32DefFile::parse(t).unwrap());
    let exports = E32Exports::from_elf(elf, frozen.as_ref()).unwrap();
    let dll = unhex(golden.0);
    assert_eq!(
        job.encode_elf_with(elf, ordinals, time_of(&dll), Some(&exports))
            .unwrap(),
        dll
    );
    assert_eq!(exports.def_text(), golden.1);
    if let Some(dso) = golden.2 {
        let bytes = E32Dso {
            soname,
            linkas: &job.linkas,
            exports: &exports,
        }
        .bytes()
        .unwrap();
        assert_eq!(bytes, unhex(dso));
    }
}

#[test]
fn experiment_54_absent_ordinal_matches_elf2e32_next() {
    // `_Z8MathGonei @ 2 ABSENT`: slot → entry point, full presence bitmap 0xfd,
    // `_._.absent_export_2` in the .dso.
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp52_mathlib_elf.hex"))).unwrap();
    check_experiment_54(
        &experiment_54("e5d1b001", "mathlib", false),
        &elf,
        Some(include_str!("../../testdata/exp54_absent_in.def")),
        (
            include_str!("../../testdata/exp54_absent_dll.hex"),
            include_str!("../../testdata/exp54_absent_out.def"),
            Some(include_str!("../../testdata/exp54_absent_dso.hex")),
        ),
        "e.dso",
    );
}

#[test]
fn experiment_54_new_symbol_after_frozen_ones_matches_elf2e32_next() {
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp52_mathlib_elf.hex"))).unwrap();
    check_experiment_54(
        &experiment_54("e5d1b001", "mathlib", false),
        &elf,
        Some(include_str!("../../testdata/exp54_new_in.def")),
        (
            include_str!("../../testdata/exp54_new_dll.hex"),
            include_str!("../../testdata/exp54_new_out.def"),
            None,
        ),
        "c.dso",
    );
    let frozen = E32DefFile::parse(include_str!("../../testdata/exp54_new_in.def")).unwrap();
    let exports = E32Exports::from_elf(&elf, Some(&frozen)).unwrap();
    assert_eq!(exports.new_names(), ["_Z7MathAddii"]);
}

#[test]
fn experiment_54_class_exports_data_and_comments_match_elf2e32_next() {
    // vtable/typeinfo are data exports, `_ZTS` is skipped, comments come back after
    // ` ; `, kinds follow the .def (`_ZTI` frozen without DATA stays a function).
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp54_shape_elf.hex"))).unwrap();
    check_experiment_54_with(
        &experiment_54("e5d1b003", "shape", false),
        &elf,
        &ordinals_table(include_str!("../../testdata/exp54_shape_ordinals.txt")),
        Some(include_str!("../../testdata/exp54_shape_in.def")),
        (
            include_str!("../../testdata/exp54_shape_dll.hex"),
            include_str!("../../testdata/exp54_shape_out.def"),
            Some(include_str!("../../testdata/exp54_shape_dso.hex")),
        ),
        "sc.dso",
    );
}

#[test]
fn experiment_54_sparse_export_bitmap_matches_elf2e32_next() {
    // 42 ordinals, 2 and 20 absent: type 2, meta 0x05 + 0xfd 0xf7, code at 0xa0.
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp54_f40_elf.hex"))).unwrap();
    let job = experiment_54("e5d1b002", "f", false);
    let frozen = E32DefFile::parse(include_str!("../../testdata/exp54_f40_in.def")).unwrap();
    let exports = E32Exports::from_elf(&elf, Some(&frozen)).unwrap();
    assert_eq!(
        exports.description().unwrap(),
        (
            E32ImageHeaderV::EXPORT_DESC_SPARSE_BITMAP,
            vec![0x05, 0xfd, 0xf7]
        )
    );
    let dll = unhex(include_str!("../../testdata/exp54_f40_dll.hex"));
    let none = E32Ordinals::default();
    assert_eq!(
        job.encode_elf_with(&elf, &none, time_of(&dll), Some(&exports))
            .unwrap(),
        dll
    );
}

#[test]
fn experiment_54_dll_data_needs_dlldata_and_then_matches_elf2e32_next() {
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp54_data_elf.hex"))).unwrap();
    let exports = E32Exports::from_elf(&elf, None).unwrap();
    let none = E32Ordinals::default();
    let err = experiment_54("e5d1b004", "data", false)
        .encode_elf_with(&elf, &none, E32Time(0), Some(&exports))
        .unwrap_err()
        .to_string();
    assert!(err.contains("initialized writable data"), "{err}");
    check_experiment_54(
        &experiment_54("e5d1b004", "data", true),
        &elf,
        None,
        (
            include_str!("../../testdata/exp54_data_dll.hex"),
            include_str!("../../testdata/exp54_data_out.def"),
            Some(include_str!("../../testdata/exp54_data_dso.hex")),
        ),
        "dataD.dso",
    );
}

#[test]
fn frozen_symbol_missing_from_elf_is_an_error() {
    let elf = ElfImage::parse(unhex(include_str!("../../testdata/exp52_mathlib_elf.hex"))).unwrap();
    let frozen =
        E32DefFile::parse("EXPORTS\n\t_Z9MathTwicei @ 1 NONAME\n\t_Z8MathGonei @ 2 NONAME\n")
            .unwrap();
    let err = E32Exports::from_elf(&elf, Some(&frozen))
        .unwrap_err()
        .to_string();
    assert!(err.contains("_Z8MathGonei"), "{err}");
}
