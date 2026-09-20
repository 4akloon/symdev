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
    let project = dir.path().join("p");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(project.join("bld.inf"), "PRJ_TESTMMPFILES\ntest.mmp\n").unwrap();
    let err = fake_at(fake_sdk(dir.path()))
        .build(&Project { root: project })
        .unwrap_err();
    assert_eq!(err.to_string(), "no MMP to build");
}

/// Gaps 1 and 2: a Carbide block-comment header and a conditional section.
#[test]
fn a_block_comment_and_a_conditional_are_preprocessed_away() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("p");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("bld.inf"),
        "/*\n============\n Name: bld.inf\n============\n*/\n\n         PRJ_MMPFILES\n#ifdef GCCE\napp.mmp\n#else\nother.mmp\n#endif\n",
    )
    .unwrap();
    std::fs::write(
        project.join("app.mmp"),
        "/* generated */\nTARGET app.exe\nTARGETTYPE EXE\n         #ifdef MARM_ARMV5\nSOURCE arm.cpp\n#endif\n#if GCCE\nSOURCE never.cpp\n#endif\n",
    )
    .unwrap();
    // The build fails at the compiler (there is no source), but the front end is past.
    let err = fake_at(fake_sdk(dir.path()))
        .build(&Project { root: project })
        .unwrap_err()
        .to_string();
    assert!(err.contains("arm.cpp"), "{err}");
    assert!(!err.contains("never.cpp"), "{err}");
}
