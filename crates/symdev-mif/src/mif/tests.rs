use super::*;

/// svgb-mif-spec.md §6.3: one SVG icon at `/c32,8`, with the real tool's bytes.
#[test]
fn one_icon_matches_the_recorded_container() {
    let data = vec![0xce, 0x56, 0xfa, 0x03, 0xff];
    let mif = MifFile::new(vec![MifIcon::svg("c.svg", data.clone())]);
    let bytes = mif.bytes().unwrap();
    assert_eq!(&bytes[..16], b"B##4\x02\0\0\0\x10\0\0\0\x02\0\0\0");
    // Two entries, both pointing at the one block after the table.
    assert_eq!(&bytes[16..24], [0x20, 0, 0, 0, 0x25, 0, 0, 0]);
    assert_eq!(&bytes[24..32], &bytes[16..24]);
    assert_eq!(&bytes[32..36], b"C##4");
    assert_eq!(&bytes[44..48], &(data.len() as u32).to_le_bytes());
    assert_eq!(
        &bytes[48..64],
        [1, 0, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0]
    );
    assert_eq!(&bytes[64..], &data);
    assert_eq!(bytes.len(), 32 + 32 + data.len());
}

/// svgb-mif-spec.md §7: the header text and the stem rule.
#[test]
fn mbg_text_matches_the_recorded_header() {
    let mif = MifFile::new(vec![MifIcon::svg("c.svg", vec![0])]);
    assert_eq!(
        mif.mbg_text("c.mif"),
        " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMifC\r\n\t{\r\n\
         \tEMbmCC = 16384,\r\n\tEMbmCC_mask = 16385,\r\n\tEMbmCLastElement\r\n\t};\r\n"
    );
    let two = MifFile::new(vec![
        MifIcon::svg("UPPER.svg", vec![0]),
        MifIcon::svg("bB_cc.svg", vec![0]),
    ]);
    let text = two.mbg_text("out_aif.mif");
    assert!(text.contains("enum TMifOut_aif\r\n"), "{text}");
    assert!(text.contains("\tEMbmOut_aifUpper = 16384,\r\n"), "{text}");
    assert!(text.contains("\tEMbmOut_aifBb_cc = 16386,\r\n"), "{text}");
}

/// svgb-mif-spec.md §6.2 and experiment 64: `/OPT` text to the two display-mode fields.
#[test]
fn depth_text_gives_the_display_modes() {
    let d = MifDepth::parse("c32,8").unwrap();
    assert_eq!((d.display_mode(), d.mask_mode()), (11, 4));
    assert_eq!(d.depth, "c32");
    assert_eq!(d.mask, Some(8));
    let d = MifDepth::parse("c24,1").unwrap();
    assert_eq!((d.display_mode(), d.mask_mode()), (8, 1));
    let d = MifDepth::parse("C24").unwrap();
    assert_eq!((d.display_mode(), d.mask_mode()), (8, 0));
    assert_eq!(d.depth, "c24");
    assert_eq!(d.mask, None);
    for (text, mode) in [
        ("1", 1),
        ("2", 2),
        ("4", 3),
        ("8", 4),
        ("c4", 5),
        ("c8", 6),
        ("c12", 10),
        ("c16", 7),
    ] {
        assert_eq!(
            MifDepth::parse(text).unwrap().display_mode(),
            mode,
            "{text}"
        );
    }
    for bad in ["", "c32,4", "c3", "24", "c32,", "c32,8,1", "/c32"] {
        assert!(MifDepth::parse(bad).is_err(), "{bad} must be rejected");
    }
}

/// Experiment 64: three `.bmp` icons give a 64-byte stub whose entries point into the
/// sibling `.mbm` by negated index, whatever the depths.
#[test]
fn bitmap_icons_give_the_recorded_stub() {
    let mif = MifFile::new(vec![
        MifIcon::bitmap("blackbox.bmp", 0, None),
        MifIcon::bitmap("bridges.bmp", 1, None),
        MifIcon::bitmap("cube.bmp", 2, None),
    ]);
    let bytes = mif.bytes().unwrap();
    let mut want = b"B##4\x02\0\0\0\x10\0\0\0\x06\0\0\0".to_vec();
    for offset in [0u32, 0, 0xffff_ffff, 0xffff_ffff, 0xffff_fffe, 0xffff_fffe] {
        want.extend_from_slice(&offset.to_le_bytes());
        want.extend_from_slice(&[0, 0, 0, 0]);
    }
    assert_eq!(bytes, want);
}

/// Experiment 64: `/c24,8 blackbox.bmp /c24 cube.bmp` — the mask is bitmap 1 of the
/// `.mbm`, so the next icon is bitmap 2; and the header lists the mask.
#[test]
fn a_bitmap_mask_takes_its_own_slot() {
    let mif = MifFile::new(vec![
        MifIcon::bitmap("blackbox.bmp", 0, Some(1)),
        MifIcon::bitmap("cube.bmp", 2, None),
    ]);
    let bytes = mif.bytes().unwrap();
    assert_eq!(bytes.len(), 48);
    assert_eq!(&bytes[12..16], [4, 0, 0, 0]);
    assert_eq!(&bytes[16..24], [0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&bytes[24..32], [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
    assert_eq!(&bytes[32..40], [0xfe, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
    assert_eq!(&bytes[40..48], &bytes[32..40]);
    assert_eq!(
        mif.mbg_text("m.mif"),
        " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMifM\r\n\t{\r\n\
         \tEMbmMBlackbox = 16384,\r\n\tEMbmMBlackbox_mask = 16385,\r\n\tEMbmMCube = 16386,\r\n\
         \tEMbmMLastElement\r\n\t};\r\n"
    );
}

/// Experiment 64: `/c24 blackbox.bmp /c32,8 a.svg /c24 cube.bmp` — the SVG block sits
/// right after the six entries, the bitmaps count only bitmaps.
#[test]
fn a_mixed_container_interleaves_in_source_order() {
    let data = vec![0xce, 0x56, 0xfa, 0x03];
    let mif = MifFile::new(vec![
        MifIcon::bitmap("blackbox.bmp", 0, None),
        MifIcon::svg("a.svg", data.clone()),
        MifIcon::bitmap("cube.bmp", 1, None),
    ]);
    let bytes = mif.bytes().unwrap();
    assert_eq!(&bytes[16..32], [0u8; 16]);
    let len = (32 + data.len()) as u32;
    let entry = [0x40u32.to_le_bytes(), len.to_le_bytes()].concat();
    assert_eq!(&bytes[32..40], entry.as_slice());
    assert_eq!(&bytes[40..48], entry.as_slice());
    assert_eq!(&bytes[48..56], [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0]);
    assert_eq!(&bytes[56..64], &bytes[48..56]);
    assert_eq!(&bytes[64..68], b"C##4");
    assert_eq!(bytes.len(), 64 + 32 + data.len());
    let text = mif.mbg_text("mix.mif");
    assert!(text.contains("\tEMbmMixBlackbox = 16384,\r\n\tEMbmMixA = 16386,\r\n\tEMbmMixA_mask = 16387,\r\n\tEMbmMixCube = 16388,\r\n\tEMbmMixLastElement\r\n"), "{text}");
}

/// Experiment 64: two SVGs at `/c32` (no mask) still step by two, without `_mask` lines.
#[test]
fn a_mask_less_icon_still_advances_by_two() {
    let depth = MifDepth::parse("c32").unwrap();
    let mif = MifFile::new(vec![
        MifIcon::svg_at("a.svg", vec![0], &depth, false),
        MifIcon::svg_at("b.svg", vec![0], &depth, true),
    ]);
    assert_eq!(
        mif.mbg_text("two.mif"),
        " \r\n/* This file has been generated, DO NOT MODIFY. */\r\nenum TMifTwo\r\n\t{\r\n\
         \tEMbmTwoA = 16384,\r\n\tEMbmTwoB = 16386,\r\n\tEMbmTwoLastElement\r\n\t};\r\n"
    );
    let bytes = mif.bytes().unwrap();
    // Second block: depth 11, animated, mask 0 (spec §6.3's two-icon example, mask-less).
    let second = 48 + 33;
    assert_eq!(
        &bytes[second + 16..second + 32],
        [1, 0, 0, 0, 11, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0]
    );
}
