use std::path::{Path, PathBuf};

use super::*;
use crate::{Mmp, MmpResource};

fn res(file: &str, targetpath: Option<&str>) -> MmpResource {
    MmpResource {
        file: file.into(),
        sourcepath: Some("..\\data".into()),
        targetpath: targetpath.map(Into::into),
        header: true,
        lang: Vec::new(),
    }
}

#[test]
fn app_resource_installs_to_its_targetpath() {
    assert_eq!(
        res("gui.rss", Some("\\resource\\apps"))
            .install_dest()
            .unwrap(),
        "!:\\resource\\apps\\gui.rsc"
    );
}

#[test]
fn reg_resource_installs_to_import_apps_like_the_sdk_pkg() {
    assert_eq!(
        res("gui_reg.rss", Some("\\private\\10003a3f\\apps"))
            .install_dest()
            .unwrap(),
        "!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc"
    );
}

#[test]
fn resource_source_follows_sourcepath() {
    assert_eq!(
        res("gui.rss", None).source(Path::new("/p/group")),
        PathBuf::from("/p/group/../data/gui.rss")
    );
}

#[test]
fn library_names_become_dso() {
    let mut m = Mmp::parse("TARGET gui.exe\nTARGETTYPE EXE\nSOURCE a.cpp\n").unwrap();
    m.library = vec!["euser.lib".into(), "avkon.lib".into(), "x.dso".into()];
    assert_eq!(m.dso_libraries(), ["euser.dso", "avkon.dso", "x.dso"]);
}

#[test]
fn casefold_links_wrong_case_includes() {
    let dir = std::env::temp_dir().join(format!("symdev-casefold-{}", std::process::id()));
    let inc = dir.join("include");
    std::fs::create_dir_all(inc.join("sub")).unwrap();
    std::fs::write(
        inc.join("fbs.h"),
        "#include <FbsMessage.h>\n#include \"Sub/Deep.h\"\n",
    )
    .unwrap();
    std::fs::write(inc.join("fbsmessage.h"), "x").unwrap();
    std::fs::write(inc.join("sub").join("deep.h"), "y").unwrap();
    let out = SdkIncludeCaseFold::ensure(&inc, &dir.join("overlay")).unwrap();
    assert_eq!(
        std::fs::read_to_string(out.join("FbsMessage.h")).unwrap(),
        "x"
    );
    assert_eq!(
        std::fs::read_to_string(out.join("Sub/Deep.h")).unwrap(),
        "y"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
