use crate::{
    SisArray, SisBlob37, SisController, SisDate, SisDateTime, SisEncode, SisFile, SisFiles,
    SisHash, SisInfo, SisLanguage, SisLanguages, SisPkgUid, SisProduct, SisProductVersion,
    SisProducts, SisSignatures39, SisString, SisTime, SisU32, SisVersion, SisWord41, SisWords,
    SisWords16, SisWords19,
};

fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn hello_type39_golden() -> Vec<u8> {
    parse_hex(include_str!("../testdata/hello_type39.hex"))
}

fn hello_sig_blob() -> [u8; 48] {
    [
        0x30, 0x2c, 0x02, 0x14, 0x4d, 0xe0, 0xcf, 0xec, 0x52, 0x8a, 0x05, 0x95, 0x13, 0x7d, 0xfc,
        0x0c, 0x66, 0x34, 0xe4, 0x00, 0x75, 0x28, 0xae, 0xa0, 0x02, 0x14, 0x50, 0x20, 0x97, 0x21,
        0xc3, 0x8a, 0xb4, 0xdd, 0xb9, 0xc0, 0x1d, 0x71, 0x53, 0xd3, 0x3d, 0xe7, 0x10, 0x62, 0xa8,
        0xc0, 0x00, 0x00,
    ]
}

fn hello_der() -> Vec<u8> {
    hello_type39_golden()[148..148 + 1171].to_vec()
}

fn hello_controller() -> SisController {
    SisController::new(
        SisInfo::new(
            SisPkgUid::new(0xe79e_4cf9),
            SisString::new("Vendor"),
            SisArray::new(vec![SisString::new("hello").field()]),
            SisArray::new(vec![SisString::new("Vendor-EN").field()]),
            SisVersion::new(1, 0, 24),
            SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
        ),
        SisWords16::new(SisWords::new(vec![0x21])),
        SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
        SisProducts::new(SisArray::new(vec![
            SisProduct::new(
                SisPkgUid::new(0x1027_52ae),
                SisProductVersion::new(SisVersion::new(0, 0, 0)),
                SisArray::new(vec![SisString::new("S60ProductID").field()]),
            )
            .field(),
        ])),
        SisWords19::new(SisWords::new(vec![0x14])),
        SisFiles::new(
            SisArray::new(vec![
                SisFile::new(
                    SisString::new("!:\\sys\\bin\\hello.exe"),
                    SisString::new(""),
                    Some(SisWord41::new(0x000b_e000)),
                    SisHash::new(
                        [1, 0x25, 0x14],
                        [
                            0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a,
                            0x77, 0xe5, 0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
                        ],
                    ),
                    SisString::new(""),
                    [3588, 0, 3588, 0, 0],
                )
                .field(),
            ]),
            SisWords::new(vec![0x0d]),
            SisWords::new(vec![0x1a]),
        ),
        SisU32::new(0),
    )
}

#[test]
fn hello_frozen_dsa_signature_verifies_signed_bytes() {
    let signed = hello_controller().signed_bytes();
    assert_eq!(signed.len(), 528);
    SisBlob37::new(hello_sig_blob().to_vec())
        .verify_dsa_sha1(&signed, &hello_der())
        .unwrap();
}

#[test]
fn hello_frozen_dsa_signature_rejects_wrong_bytes() {
    let mut signed = hello_controller().signed_bytes();
    signed[0] ^= 1;
    assert!(
        SisBlob37::new(hello_sig_blob().to_vec())
            .verify_dsa_sha1(&signed, &hello_der())
            .is_err()
    );
}

fn type37_blob(sigs: &SisSignatures39) -> Vec<u8> {
    let field = sigs.field().bytes();
    let n37 = u32::from_le_bytes(field[76 + 4..76 + 8].try_into().unwrap()) as usize;
    field[76 + 8..76 + 8 + n37].to_vec()
}

fn test_cert_der() -> Vec<u8> {
    parse_hex(include_str!("../testdata/test_dsa_cert.hex"))
}

#[test]
fn live_sign_traditional_pem_verifies() {
    let controller = hello_controller();
    let key = parse_hex(include_str!("../testdata/test_dsa_key.hex"));
    let cert = test_cert_der();
    let signed = controller
        .signatures_from_key_and_cert(&key, &cert, "")
        .unwrap();
    assert_eq!(signed.chain.cert.data, cert);
    SisBlob37::new(type37_blob(&signed))
        .verify_dsa_sha1(&controller.signed_bytes(), &cert)
        .unwrap();
}

#[test]
fn live_sign_encrypted_traditional_pem_verifies() {
    let controller = hello_controller();
    let key = parse_hex(include_str!("../testdata/test_dsa_key_3des.hex"));
    let cert = test_cert_der();
    let signed = controller
        .signatures_from_key_and_cert(&key, &cert, "test")
        .unwrap();
    SisBlob37::new(type37_blob(&signed))
        .verify_dsa_sha1(&controller.signed_bytes(), &cert)
        .unwrap();
}

#[test]
fn live_sign_pkcs8_pem_verifies() {
    let controller = hello_controller();
    let key = parse_hex(include_str!("../testdata/test_dsa_key_pkcs8.hex"));
    let cert = test_cert_der();
    let signed = controller
        .signatures_from_key_and_cert(&key, &cert, "")
        .unwrap();
    SisBlob37::new(type37_blob(&signed))
        .verify_dsa_sha1(&controller.signed_bytes(), &cert)
        .unwrap();
}
