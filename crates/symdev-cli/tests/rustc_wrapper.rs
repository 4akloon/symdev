use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// cargo runs `<project>/build/symdev-rustc` (an absolute path, experiment 114 §1.4 and
/// the cargo-run notes); the wrapper hands rustc the `sysroot/` beside it.
#[test]
fn symdev_rustc_runs_rustc_with_the_sysroot_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let build = dir.path().join("build");
    std::fs::create_dir_all(build.join("sysroot")).unwrap();
    let wrapper = build.join("symdev-rustc");
    std::os::unix::fs::symlink(assert_cmd::cargo::cargo_bin("symdev"), &wrapper).unwrap();
    let rustc = dir.path().join("rustc");
    std::fs::write(&rustc, "#!/bin/sh\necho \"$@\"\n").unwrap();
    std::fs::set_permissions(&rustc, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new(&wrapper)
        .arg("-vV")
        .env("SYMDEV_RUSTC", &rustc)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let want = format!("--sysroot {}/sysroot -vV\n", build.display());
    assert_eq!(String::from_utf8_lossy(&out.stdout), want);
}

#[test]
fn symdev_rustc_without_a_sysroot_says_how_to_make_one() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("symdev-rustc");
    std::os::unix::fs::symlink(assert_cmd::cargo::cargo_bin("symdev"), &wrapper).unwrap();
    let out = Command::new(&wrapper).arg("-vV").output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("sysroot") && err.contains("symdev build"),
        "{err}"
    );
}
