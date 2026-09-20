use super::*;

#[test]
fn dname_is_makekeys_example_usage() {
    assert_eq!(
        SelfSignedDsa::DNAME,
        "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com"
    );
}

#[test]
fn validate_password_rejects_shorter_than_four_characters() {
    let mut pkg = fake_pkg();
    pkg.password = "abc".into();
    let err = pkg.validate_password().unwrap_err();
    assert_eq!(
        err.to_string(),
        "SYMDEV_SIGN_PASSWORD must be at least 4 characters"
    );
    pkg.password = "".into();
    assert!(pkg.validate_password().is_err());
    pkg.password = "abcd".into();
    pkg.validate_password().unwrap();
}

#[test]
fn write_pkg_file_writes_recorded_pkg_next_to_exe() {
    let dir = tempfile::tempdir().unwrap();
    let path = fake_pkg()
        .write_pkg_file(
            dir.path(),
            &[("hello_reg.rsc".into(), SisPkgFile::reg_rsc_dest("hello"))],
        )
        .unwrap();
    assert_eq!(path, dir.path().join("hello.pkg"));
    let s = std::fs::read_to_string(&path).unwrap();
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n"
    );
}

#[test]
fn pkg_text_includes_verified_reg_rsc_dest() {
    let s = fake_pkg().pkg_text(&[("hello_reg.rsc".into(), SisPkgFile::reg_rsc_dest("hello"))]);
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n\"hello_reg.rsc\"\t\t-\"!:\\private\\10003a3f\\import\\apps\\hello_reg.rsc\"\n"
    );
}
