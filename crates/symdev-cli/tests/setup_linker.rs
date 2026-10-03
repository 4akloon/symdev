#[test]
fn setup_linker_links_both_roles_to_this_binary() {
    let dir = tempfile::tempdir().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("symdev");
    let ok = std::process::Command::new(&bin)
        .args(["setup-linker", "--dir"])
        .arg(dir.path())
        .status()
        .unwrap()
        .success();
    assert!(ok);
    for role in ["symdev-ld", "symdev-rustc"] {
        assert_eq!(std::fs::read_link(dir.path().join(role)).unwrap(), bin);
    }
    // A second run accepts its own links and changes nothing.
    assert!(
        std::process::Command::new(&bin)
            .args(["setup-linker", "--dir"])
            .arg(dir.path())
            .status()
            .unwrap()
            .success()
    );
}
