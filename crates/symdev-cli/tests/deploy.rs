use predicates::prelude::*;

mod common;
use common::{HELLO, bin, write_toml};

#[test]
fn deploy_missing_sisx() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    bin()
        .current_dir(&dir)
        .arg("deploy")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "SISX not found: build/hello.sisx (run symdev package)",
        ))
        .stderr(predicate::str::contains("not implemented").not());
}

#[test]
fn deploy_prints_existing_sisx_path() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, HELLO);
    std::fs::create_dir_all(dir.path().join("build")).unwrap();
    let sisx = dir.path().join("build/hello.sisx");
    std::fs::write(&sisx, b"").unwrap();
    bin()
        .current_dir(&dir)
        .arg("deploy")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::eq(format!("{}\n", sisx.display())));
}
