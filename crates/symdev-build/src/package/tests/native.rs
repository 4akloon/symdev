use super::*;

#[test]
fn package_writes_native_sis_and_keys_without_wine() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let stale_cer = dir.path().join("hello.cer");
    let stale_key = dir.path().join("hello.key");
    std::fs::write(&stale_cer, b"stale-cer").unwrap();
    std::fs::write(&stale_key, b"stale-key").unwrap();
    let mut pkg = fake_pkg();
    pkg.uid3 = 0xe79e_4cf9;
    pkg.version = (1, 0, 24);
    pkg.vendor = "Vendor".into();
    pkg.capabilities = hello_caps();
    let out = pkg
        .package(&[Artifact::exe(exe_path)])
        .expect("native makekeys must not spawn Wine when cert/key are absent");
    assert_eq!(out.primary.file_name().unwrap(), "hello.sisx");
    assert!(dir.path().join("hello.sis").is_file());
    assert!(dir.path().join("hello.sisx").is_file());
    assert!(dir.path().join("hello.pkg").is_file());
    let rsc = dir.path().join("hello_reg.rsc");
    assert!(rsc.is_file());
    let rsc_bytes = std::fs::read(&rsc).unwrap();
    assert_eq!(
        &rsc_bytes[..16],
        &symdev_rcomp::RscUid::registration(0xe79e_4cf9).bytes()
    );
    let sis = std::fs::read(dir.path().join("hello.sis")).unwrap();
    let stored = symdev_sis::SisCompressed::smallest(&rsc_bytes)
        .unwrap()
        .data;
    assert!(sis.windows(stored.len()).any(|w| w == stored));
    let cer = std::fs::read(&stale_cer).unwrap();
    let key = std::fs::read(&stale_key).unwrap();
    assert_ne!(cer, b"stale-cer");
    assert_ne!(key, b"stale-key");
    let cer_text = String::from_utf8_lossy(&cer);
    let key_text = String::from_utf8_lossy(&key);
    assert!(cer_text.contains("BEGIN CERTIFICATE"), "{cer_text}");
    assert!(key_text.contains("BEGIN PRIVATE KEY"), "{key_text}");
}

#[test]
fn package_writes_native_sisx_without_wine_signsis() {
    let dir = tempfile::tempdir().unwrap();
    let exe_path = dir.path().join("hello.exe");
    std::fs::write(&exe_path, hello_exe_bytes()).unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(
        &cert,
        parse_hex(include_str!(
            "../../../../symdev-sis/src/testdata/test_dsa_cert.hex"
        )),
    )
    .unwrap();
    std::fs::write(
        &key,
        parse_hex(include_str!(
            "../../../../symdev-sis/src/testdata/test_dsa_key.hex"
        )),
    )
    .unwrap();
    let mut pkg = fake_pkg();
    pkg.uid3 = 0xe79e_4cf9;
    pkg.version = (1, 0, 24);
    pkg.vendor = "Vendor".into();
    pkg.capabilities = hello_caps();
    pkg.cert = Some(cert);
    pkg.key = Some(key);
    let out = pkg
        .package(&[Artifact::exe(exe_path)])
        .expect("native SISX must not spawn Wine signsis");
    assert!(out.primary.is_file());
    assert_eq!(out.primary.file_name().unwrap(), "hello.sisx");
    assert!(dir.path().join("hello.sis").is_file());
    assert!(dir.path().join("hello.sisx").is_file());
    let sisx = std::fs::read(&out.primary).unwrap();
    assert_eq!(&sisx[..16], &symdev_sis::SisUid::new(0xe79e_4cf9).bytes());
    assert_ne!(sisx, hello_sis_golden());
    assert!(sisx.len() > hello_sis_golden().len());
}
