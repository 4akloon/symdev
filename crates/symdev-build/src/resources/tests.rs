use std::path::{Path, PathBuf};

use super::*;
use crate::{Mmp, MmpResource};

fn res(file: &str, targetpath: Option<&str>) -> MmpResource {
    MmpResource {
        file: file.into(),
        sourcepath: Some("..\\data".into()),
        targetpath: targetpath.map(Into::into),
        header: true,
        ..MmpResource::default()
    }
}

#[test]
fn app_resource_installs_to_its_targetpath() {
    assert_eq!(
        res("gui.rss", Some("\\resource\\apps"))
            .install_dest("SC")
            .unwrap(),
        "!:\\resource\\apps\\gui.rsc"
    );
}

#[test]
fn reg_resource_installs_to_import_apps_like_the_sdk_pkg() {
    assert_eq!(
        res("gui_reg.rss", Some("\\private\\10003a3f\\apps"))
            .install_dest("SC")
            .unwrap(),
        "!:\\private\\10003a3f\\import\\apps\\gui_reg.rsc"
    );
}

/// §6.2/§6.3: `TARGET` inside the block renames the `.rsc` and the `.rsg`, and the
/// extension follows the language code.
#[test]
fn target_inside_the_block_renames_the_outputs() {
    let mut r = res("Puzzles.rss", Some("\\resource\\apps"));
    r.target = Some("Puzzles_0xa000ef77".into());
    assert_eq!(r.stem().unwrap(), "Puzzles_0xa000ef77");
    assert_eq!(r.output("SC").unwrap(), "Puzzles_0xa000ef77.rsc");
    assert_eq!(r.header_name().unwrap(), "Puzzles_0xa000ef77.rsg");
    assert_eq!(
        r.install_dest("SC").unwrap(),
        "!:\\resource\\apps\\Puzzles_0xa000ef77.rsc"
    );
    // A directory or an extension written on TARGET is discarded.
    r.target = Some("..\\out\\other.rsc".into());
    assert_eq!(r.stem().unwrap(), "other");
}

#[test]
fn a_language_code_picks_the_extension() {
    let mut r = res("gui.rss", Some("\\resource\\apps"));
    assert_eq!(r.languages(&[]), ["SC"]);
    assert_eq!(r.languages(&["01".to_string()]), ["01"]);
    r.lang = vec!["02".into(), "03".into()];
    assert_eq!(r.languages(&["01".to_string()]), ["02", "03"]);
    assert_eq!(r.output("02").unwrap(), "gui.r02");
    assert_eq!(r.install_dest("03").unwrap(), "!:\\resource\\apps\\gui.r03");
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

/// §4.3/§4.6: an export bound for `epoc32/include` is staged in the build directory,
/// which is already the first `-I`; anything else is named in a warning.
#[test]
fn exports_are_staged_into_the_build_directory() {
    use crate::model::BldExport;
    use crate::resources::ProjectExports;

    let dir = std::env::temp_dir().join(format!("symdev-exports-{}", std::process::id()));
    let group = dir.join("group");
    let inc = dir.join("inc");
    let build = dir.join("build");
    std::fs::create_dir_all(&group).unwrap();
    std::fs::create_dir_all(&inc).unwrap();
    std::fs::create_dir_all(&build).unwrap();
    std::fs::write(inc.join("api.h"), "x").unwrap();
    std::fs::write(inc.join("deep.h"), "y").unwrap();
    std::fs::write(group.join("icon.mif"), "z").unwrap();

    let exports = [
        BldExport {
            source: "..\\inc\\api.h".into(),
            dest: None,
            zip: false,
        },
        BldExport {
            source: "..\\inc\\deep.h".into(),
            dest: Some("\\epoc32\\include\\sub\\deep.h".into()),
            zip: false,
        },
        BldExport {
            source: "icon.mif".into(),
            dest: Some("z:\\resource\\apps\\icon.mif".into()),
            zip: false,
        },
    ];
    let warnings = ProjectExports::stage(&exports, &group, &build).unwrap();
    assert_eq!(std::fs::read_to_string(build.join("api.h")).unwrap(), "x");
    assert_eq!(
        std::fs::read_to_string(build.join("sub/deep.h")).unwrap(),
        "y"
    );
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("icon.mif"), "{warnings:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}
