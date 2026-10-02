mod common;
use common::{bin, fake_epocroot, hello_with_uid3, write_toml};

/// `symdev freeze` reads only the EPOCROOT (for the `bld.inf`); a stale compiler
/// variable is not its business.
#[test]
fn freeze_ignores_a_stale_compiler_variable() {
    let dir = tempfile::tempdir().unwrap();
    write_toml(&dir, &hello_with_uid3());
    std::fs::create_dir_all(dir.path().join("group")).unwrap();
    std::fs::write(
        dir.path().join("group/bld.inf"),
        "PRJ_MMPFILES\nhello.mmp\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("group/hello.mmp"),
        "TARGET hello.exe\nTARGETTYPE EXE\nSOURCE main.cpp\n",
    )
    .unwrap();
    let epocroot = fake_epocroot(&dir);
    bin()
        .current_dir(&dir)
        .env("SYMDEV_EPOCROOT", &epocroot)
        .env("SYMDEV_GXX", "/nonexistent/symdev/g++")
        .arg("freeze")
        .assert()
        .success()
        .stdout("exports already frozen\n");
}
