use super::*;

#[test]
fn package_empty_artifacts_is_no_e32_artifact() {
    let err = fake_pkg().package(&[]).unwrap_err();
    assert_eq!(err.to_string(), "no E32 artifact");
}

#[test]
fn package_missing_e32_file_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.exe");
    let err = fake_pkg().package(&[Artifact::exe(path)]).unwrap_err();
    assert_eq!(err.to_string(), "E32 not found");
}

/// D1 = A (experiment 114 §1.7): the key symdev generates is unencrypted, so a password
/// would protect nothing; the original `makekeys` allows such a key and `signsis` signs
/// with it given no pass phrase.
#[test]
fn a_generated_self_signed_pair_needs_no_password() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = SisPackage {
        password: String::new(),
        ..fake_pkg()
    };
    let exe = dir.path().join("hello.exe");
    std::fs::write(&exe, hello_exe_bytes()).unwrap();
    let got = pkg.package(&[Artifact::exe(exe)]).unwrap();
    assert!(got.primary.is_file());
}

#[test]
fn an_encrypted_key_of_the_users_still_needs_four_characters() {
    let dir = tempfile::tempdir().unwrap();
    let (cer, key) = (dir.path().join("a.cer"), dir.path().join("a.key"));
    std::fs::write(&cer, "-----BEGIN CERTIFICATE-----\n").unwrap();
    std::fs::write(
        &key,
        "-----BEGIN DSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n",
    )
    .unwrap();
    let pkg = SisPackage {
        password: "ab".into(),
        cert: Some(cer),
        key: Some(key),
        ..fake_pkg()
    };
    let exe = dir.path().join("hello.exe");
    std::fs::write(&exe, hello_exe_bytes()).unwrap();
    let e = pkg.package(&[Artifact::exe(exe)]).unwrap_err().to_string();
    assert!(e.contains("at least 4 characters"), "{e}");
}

#[test]
fn existing_signing_pair_when_both_files_exist() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    std::fs::write(&key, b"k").unwrap();
    let mut pkg = fake_pkg();
    pkg.cert = Some(cert.clone());
    pkg.key = Some(key.clone());
    assert_eq!(pkg.existing_signing_pair(), Some((cert, key)));
}

#[test]
fn existing_signing_pair_none_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let cert = dir.path().join("hello.cer");
    let key = dir.path().join("hello.key");
    std::fs::write(&cert, b"c").unwrap();
    let mut pkg = fake_pkg();
    pkg.cert = Some(cert.clone());
    pkg.key = Some(key);
    assert_eq!(pkg.existing_signing_pair(), None);
    pkg.cert = None;
    pkg.key = None;
    assert_eq!(pkg.existing_signing_pair(), None);
    pkg.cert = Some(cert);
    pkg.key = None;
    assert_eq!(pkg.existing_signing_pair(), None);
}
