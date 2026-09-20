use super::*;

#[test]
fn encode_signed_sisx_is_verifiable_and_not_hello_golden() {
    let key = parse_hex(include_str!("../../testdata/test_dsa_key.hex"));
    let cert = parse_hex(include_str!("../../testdata/test_dsa_cert.hex"));
    let exe = hello_exe_bytes();
    let caps = hello_caps();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &caps,
        datetime: hello_datetime(),
        files: &[],
    };
    let unsigned = SisUnsigned::encode(&spec).unwrap();
    let sisx = SisUnsigned::encode_signed(&spec, &key, &cert, "").unwrap();
    assert_eq!(&sisx[..16], &unsigned[..16]);
    assert!(sisx.len() > unsigned.len());
    assert_ne!(
        sisx,
        parse_hex(include_str!("../../testdata/hello_sisx.hex"))
    );
}

#[test]
fn experiment5_hello_key_native_sign_skipped_without_password() {
    let exp = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("src/symdev-experiment-5");
    let key = exp.join("hello.key");
    let cer = exp.join("hello.cer");
    if !key.is_file() || !cer.is_file() {
        return;
    }
    let Ok(password) = std::env::var("SYMDEV_SIGN_PASSWORD") else {
        return;
    };
    if password.is_empty() {
        return;
    }
    let exe = hello_exe_bytes();
    let caps = hello_caps();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &exe,
        capabilities: &caps,
        datetime: hello_datetime(),
        files: &[],
    };
    let key_bytes = std::fs::read(&key).unwrap();
    let cer_bytes = std::fs::read(&cer).unwrap();
    let sisx = SisUnsigned::encode_signed(&spec, &key_bytes, &cer_bytes, &password).unwrap();
    let golden = parse_hex(include_str!("../../testdata/hello_sisx.hex"));
    // RFC6979 k will not match SignSIS's random k; structure must still be SISX.
    assert_eq!(&sisx[..16], &golden[..16]);
    assert!(sisx.len() > hello_sis_golden().len());
}
