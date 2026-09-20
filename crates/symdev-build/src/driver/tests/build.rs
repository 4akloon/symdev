use symdev_core::{BuildBackend, Project};

use super::*;

#[test]
fn build_errors_without_bld_inf() {
    let dir = tempfile::tempdir().unwrap();
    let err = fake()
        .build(&Project {
            root: dir.path().to_path_buf(),
        })
        .unwrap_err();
    assert_eq!(err.to_string(), "no bld.inf");
}

#[test]
fn build_errors_when_mmp_list_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("bld.inf"), "PRJ_TESTMMPFILES\ntest.mmp\n").unwrap();
    let err = fake()
        .build(&Project {
            root: dir.path().to_path_buf(),
        })
        .unwrap_err();
    assert_eq!(err.to_string(), "no MMP to build");
}
