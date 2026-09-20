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
