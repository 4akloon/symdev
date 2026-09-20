use crate::driver::source::resolve_source;

#[test]
fn resolve_source_prefers_mmp_dir_then_project_root() {
    let dir = tempfile::tempdir().unwrap();
    let mmp_dir = dir.path().join("group");
    std::fs::create_dir(&mmp_dir).unwrap();
    std::fs::write(mmp_dir.join("hello.cpp"), b"//").unwrap();
    let found = resolve_source(None, &mmp_dir, dir.path(), "hello.cpp").unwrap();
    assert_eq!(found, mmp_dir.join("hello.cpp"));
}

#[test]
fn resolve_source_uses_sourcepath() {
    let dir = tempfile::tempdir().unwrap();
    let mmp_dir = dir.path().join("group");
    let src_dir = mmp_dir.join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("hello.cpp"), b"//").unwrap();
    let found = resolve_source(Some("src"), &mmp_dir, dir.path(), "hello.cpp").unwrap();
    assert_eq!(found, src_dir.join("hello.cpp"));
}

#[test]
fn resolve_source_accepts_symbian_backslash_sourcepath() {
    let dir = tempfile::tempdir().unwrap();
    let mmp_dir = dir.path().join("group");
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::create_dir_all(&mmp_dir).unwrap();
    std::fs::write(dir.path().join("src/app.cpp"), b"//").unwrap();
    let found = resolve_source(Some("..\\src"), &mmp_dir, dir.path(), "app.cpp").unwrap();
    assert_eq!(found, mmp_dir.join("../src/app.cpp"));
}

#[test]
fn resolve_source_missing_errors() {
    let dir = tempfile::tempdir().unwrap();
    let err = resolve_source(None, dir.path(), dir.path(), "nope.cpp").unwrap_err();
    assert!(err.to_string().contains("source not found: nope.cpp"));
}
