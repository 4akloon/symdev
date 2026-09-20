//! Tests for the `makekeys` argv adapter and job (experiment 40).

use super::*;
use std::path::PathBuf;

#[test]
fn generate_for_uses_the_given_subject() {
    let pair = SelfSignedDsa::generate_for("CN=Alice,O=Example", SystemTime::now()).unwrap();
    let pem = String::from_utf8(pair.cert_pem().to_vec()).unwrap();
    use x509_cert::der::DecodePem;
    let cert = x509_cert::Certificate::from_pem(pem.as_bytes()).unwrap();
    let subject = cert.tbs_certificate.subject.to_string();
    assert!(subject.contains("CN=Alice"), "{subject}");
    assert_eq!(cert.tbs_certificate.issuer, cert.tbs_certificate.subject);
}

#[test]
fn generate_for_rejects_malformed_subject() {
    let err = SelfSignedDsa::generate_for("not a dn", SystemTime::now())
        .err()
        .map(|e| e.to_string())
        .unwrap_or_default();
    assert!(err.contains("signing subject"), "{err}");
}

#[test]
fn dname_is_makekeys_example_usage() {
    assert_eq!(
        SelfSignedDsa::DNAME,
        "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
    );
}

#[test]
fn args_match_experiment_8() {
    let tool = MakekeysTool {
        wine: PathBuf::from("/usr/bin/wine"),
        makekeys: PathBuf::from("/sdk/epoc32/tools/makekeys.exe"),
    };
    assert_eq!(
        tool.args("secret", "hello.key", "hello.cer"),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/makekeys.exe",
            "-cert",
            "-expdays",
            "3650",
            "-password",
            "secret",
            "-len",
            "2048",
            "-dname",
            "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com",
            "hello.key",
            "hello.cer",
        ]
    );
}

#[test]
fn from_args_match_experiment_8() {
    let m = Makekeys::from_args(&[
        "makekeys".into(),
        "-cert".into(),
        "-expdays".into(),
        "3650".into(),
        "-password".into(),
        "secret".into(),
        "-len".into(),
        "2048".into(),
        "-dname".into(),
        SelfSignedDsa::DNAME.into(),
        "hello.key".into(),
        "hello.cer".into(),
    ])
    .unwrap();
    assert_eq!(m.password, "secret");
    assert_eq!(m.key, PathBuf::from("hello.key"));
    assert_eq!(m.cer, PathBuf::from("hello.cer"));
}
