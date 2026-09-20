use super::*;

#[test]
fn hello_exe_sha1_matches_experiment_29() {
    assert_eq!(
        <[u8; 20]>::from(Sha1::digest(hello_exe_bytes())),
        [
            0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5,
            0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
        ]
    );
}

#[test]
fn encode_unsigned_sis_matches_hello_pkg_fixture() {
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: hello_datetime(),
        files: &[],
    })
    .unwrap();
    let golden = hello_sis_golden();
    assert_eq!(golden.len(), 4000);
    assert_eq!(bytes, golden);
}

#[test]
fn encode_unsigned_sis_with_reg_rsc_matches_experiment_43() {
    // Wine makesis -v on experiment-7 hello.pkg plus the SDK-example `_reg.rsc` line.
    let golden = parse_hex(include_str!("../../testdata/hello_reg_sis.hex"));
    let rsc = symdev_rcomp::Rsc::registration(0xe79e_4cf9, "hello")
        .unwrap()
        .bytes()
        .unwrap();
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: crate::SisDateTime::new(
            crate::SisDate::new(2026, 8, 19),
            crate::SisTime::new(9, 2, 53),
        ),
        files: &[SisPkgFile {
            dest: &SisPkgFile::reg_rsc_dest("hello"),
            data: &rsc,
        }],
    })
    .unwrap();
    assert_eq!(bytes.len(), golden.len());
    assert_eq!(bytes, golden);
}

#[test]
fn e32_files_carry_their_own_capabilities() {
    let mut dll = vec![0u8; 0x9c];
    dll[16..20].copy_from_slice(b"EPOC");
    dll[0x88..0x8c].copy_from_slice(&0x8000u32.to_le_bytes());
    let file = SisPkgFile {
        dest: "!:\\sys\\bin\\mathlib.dll",
        data: &dll,
    };
    assert_eq!(file.e32_capabilities().map(|w| w.value), Some(0x8000));
    let rsc = SisPkgFile {
        dest: "!:\\resource\\apps\\gui.rsc",
        data: b"\x6b\x4a\x1f\x10not an E32 image at all",
    };
    assert!(rsc.e32_capabilities().is_none());
}

#[test]
fn encode_unsigned_three_file_gui_sis_matches_experiment_51() {
    // Wine makesis on gui.exe (no capabilities) + gui.rsc + gui_reg.rsc, in .pkg order.
    let exe = parse_hex(include_str!("../../testdata/exp51_gui_exe.hex"));
    let rsc = parse_hex(include_str!("../../testdata/exp51_gui_rsc.hex"));
    let reg = parse_hex(include_str!("../../testdata/exp51_gui_reg_rsc.hex"));
    let golden = parse_hex(include_str!("../../testdata/exp51_gui_sis.hex"));
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "gui",
        uid3: 0xe5d1_a001,
        version: (1, 0, 0),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &[],
        datetime: crate::SisDateTime::new(
            crate::SisDate::new(2026, 8, 19),
            crate::SisTime::new(16, 1, 18),
        ),
        files: &[
            SisPkgFile {
                dest: "!:\\resource\\apps\\gui.rsc",
                data: &rsc,
            },
            SisPkgFile {
                dest: &SisPkgFile::reg_rsc_dest("gui"),
                data: &reg,
            },
        ],
    })
    .unwrap();
    assert_eq!(bytes.len(), golden.len());
    assert_eq!(bytes, golden);
}

#[test]
fn encode_unsigned_sis_uses_project_fields_not_hello_goldens() {
    let bytes = SisUnsigned::encode(&SisUnsignedSpec {
        name: "other",
        uid3: 0xe000_0001,
        version: (0, 1, 0),
        vendor: "symdev",
        vendor_localized: "symdev",
        exe: &hello_exe_bytes(),
        capabilities: &[],
        datetime: hello_datetime(),
        files: &[],
    })
    .unwrap();
    assert_ne!(bytes, hello_sis_golden());
    assert_eq!(&bytes[..16], &SisUid::new(0xe000_0001).bytes());
    assert_ne!(&bytes[..16], &SisUid::new(0xe79e_4cf9).bytes());
}

#[test]
fn encode_unsigned_sis_rejects_capability_bits_not_derived() {
    let err = SisUnsigned::encode(&SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &["NotACapability".into()],
        datetime: hello_datetime(),
        files: &[],
    })
    .unwrap_err();
    assert_eq!(
        err.to_string(),
        "capability bit not yet derived: NotACapability"
    );
}
