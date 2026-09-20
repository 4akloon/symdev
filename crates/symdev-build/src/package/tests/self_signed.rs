use super::*;

fn hello_makekeys_not_before() -> SystemTime {
    // Frozen experiment-8 hello.cer Not Before: 2026-09-17 15:21:21 GMT
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_789_654_881)
}

#[test]
fn generated_self_signed_dsa_verifies_with_injected_dates() {
    use der::{Decode, DecodePem, Encode};
    let not_before = hello_makekeys_not_before();
    let generated = SelfSignedDsa::generate(not_before).unwrap();
    let cert_pem = generated.cert_pem();
    let key_pem = generated.key_pem();
    let cert = x509_cert::Certificate::from_pem(cert_pem).unwrap();
    assert_eq!(
        cert.tbs_certificate.serial_number,
        x509_cert::serial_number::SerialNumber::from(1u32)
    );
    assert_eq!(
        cert.tbs_certificate.signature.oid.to_string(),
        "1.2.840.10040.4.3"
    );
    let subject = cert.tbs_certificate.subject.to_string();
    assert!(subject.contains("Joe Bloggs"), "{subject}");
    assert!(subject.contains("Development"), "{subject}");
    assert!(subject.contains("Acme Ltd"), "{subject}");
    assert!(subject.contains("GB"), "{subject}");
    assert!(subject.contains("noone@nowhere.com"), "{subject}");
    assert_eq!(
        cert.tbs_certificate.validity.not_before.to_unix_duration(),
        std::time::Duration::from_secs(1_789_654_881)
    );
    assert_eq!(
        cert.tbs_certificate.validity.not_after.to_unix_duration(),
        std::time::Duration::from_secs(1_789_654_881 + 3650 * 86400)
    );
    let der = x509_cert::Certificate::from_pem(cert_pem)
        .unwrap()
        .to_der()
        .unwrap();
    let parsed = x509_cert::Certificate::from_der(&der).unwrap();
    symdev_sis::SisBlob37::new(parsed.signature.raw_bytes().to_vec())
        .verify_dsa_sha1(&parsed.tbs_certificate.to_der().unwrap(), &der)
        .unwrap();
    let spec = SisUnsignedSpec {
        name: "hello",
        uid3: 0xe79e_4cf9,
        version: (1, 0, 24),
        vendor: "Vendor",
        vendor_localized: "Vendor-EN",
        exe: &hello_exe_bytes(),
        capabilities: &hello_caps(),
        datetime: hello_datetime(),
        files: &[],
    };
    let sisx = SisUnsigned::encode_signed(&spec, key_pem, cert_pem, "").unwrap();
    assert!(sisx.len() > hello_sis_golden().len());
    let exp = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("src/symdev-experiment-5/hello.cer");
    if exp.is_file() {
        let frozen = std::fs::read(&exp).unwrap();
        assert_ne!(
            cert_pem, frozen,
            "dates/serial/k/key material block a hello.cer byte-match"
        );
    }
}
