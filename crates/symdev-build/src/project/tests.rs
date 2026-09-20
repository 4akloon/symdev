use std::path::{Path, PathBuf};

use super::{HostPath, ProjectCpp, ProjectLine, ProjectPass};

fn tokens(line: &str) -> Vec<String> {
    let records = ProjectLine::records(line);
    records
        .first()
        .map(|r| r.tokens.clone())
        .unwrap_or_default()
}

#[test]
fn a_quoted_run_keeps_its_spaces_and_loses_its_quotes() {
    assert_eq!(
        tokens("SOURCEPATH \"..\\a dir with spaces\""),
        ["SOURCEPATH", "..\\a dir with spaces"]
    );
}

#[test]
fn a_quote_inside_a_word_splits_the_word() {
    assert_eq!(tokens("MACRO -D\"QUOTED=1\""), ["MACRO", "-D", "QUOTED=1"]);
}

#[test]
fn an_empty_quoted_string_is_dropped_and_tabs_separate() {
    assert_eq!(
        tokens("\tLIBRARY\teuser.lib \"\""),
        ["LIBRARY", "euser.lib"]
    );
}

#[test]
fn line_markers_set_the_file_and_the_line_number() {
    let records =
        ProjectLine::records("# 1 \"a.mmp\"\nTARGET x.exe\n\n# 7 \"b.mmp\" 2\nSOURCE a.cpp\n");
    assert_eq!(records.len(), 2);
    assert_eq!((records[0].file.as_str(), records[0].number), ("a.mmp", 1));
    assert_eq!((records[1].file.as_str(), records[1].number), ("b.mmp", 7));
    assert_eq!(records[1].directive(), "SOURCE");
    assert_eq!(records[1].args(), ["a.cpp"]);
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("symdev-project-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn epocroot() -> Option<PathBuf> {
    std::env::var_os("SYMDEV_EPOCROOT")
        .map(PathBuf::from)
        .filter(|p| p.join("epoc32/tools/variant/variant.cfg").is_file())
}

#[test]
fn host_path_finds_a_component_whose_case_differs() {
    let dir = scratch("hostpath");
    std::fs::create_dir_all(dir.join("Inc")).unwrap();
    std::fs::write(dir.join("Inc/Header.HRH"), "").unwrap();
    assert_eq!(
        HostPath::find(&dir, "inc\\header.hrh"),
        Some(dir.join("Inc/Header.HRH"))
    );
    assert_eq!(HostPath::find(&dir, "inc\\missing.hrh"), None);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn ifdef_gcce_is_true_while_if_gcce_is_false() {
    let Some(epocroot) = epocroot() else {
        return;
    };
    let dir = scratch("macros");
    let mmp = dir.join("probe.mmp");
    std::fs::write(
        &mmp,
        "/* Carbide header */\n#ifdef GCCE\nMACRO IFDEF_TRUE\n#endif\n\
         #if GCCE\nMACRO IF_TRUE\n#endif\n#ifdef UREL\nMACRO UREL_TRUE\n#endif\n\
         #ifdef __SECURE_SOFTWARE_INSTALL__\nMACRO VARIANT_SEEN\n#endif\n\
         OPTION GCCE -O3\n",
    )
    .unwrap();
    let text = ProjectCpp::new(&epocroot)
        .run(&mmp, ProjectPass::Gcce)
        .unwrap();
    assert!(text.contains("IFDEF_TRUE"), "{text}");
    assert!(!text.contains("IF_TRUE"), "{text}");
    assert!(!text.contains("UREL_TRUE"), "{text}");
    assert!(text.contains("VARIANT_SEEN"), "{text}");
    let option = ProjectLine::records(&text)
        .into_iter()
        .find(|r| r.directive() == "OPTION")
        .unwrap();
    assert_eq!(option.tokens, ["OPTION", "GCCE", "-O3"]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_platform_pass_defines_nothing() {
    let Some(epocroot) = epocroot() else {
        return;
    };
    let dir = scratch("platform-pass");
    let bld = dir.join("bld.inf");
    std::fs::write(&bld, "#ifdef GCCE\nPRJ_EXPORTS\nx.h\n#endif\n").unwrap();
    let text = ProjectCpp::new(&epocroot)
        .run(&bld, ProjectPass::Platform)
        .unwrap();
    assert!(!text.contains("PRJ_EXPORTS"), "{text}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_platform_source_include_names_itself_and_the_fix() {
    let Some(epocroot) = epocroot() else {
        return;
    };
    let dir = scratch("platform-paths");
    let mmp = dir.join("p.mmp");
    std::fs::write(&mmp, "#include <platform_paths.hrh>\nTARGET x.exe\n").unwrap();
    let err = ProjectCpp::new(&epocroot)
        .run(&mmp, ProjectPass::Gcce)
        .unwrap_err()
        .to_string();
    assert!(err.contains("platform_paths.hrh"), "{err}");
    assert!(err.contains("SYSTEMINCLUDE \\epoc32\\include"), "{err}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_variant_header_is_found_despite_its_case() {
    let Some(epocroot) = epocroot() else {
        return;
    };
    let header = ProjectCpp::new(&epocroot).variant_header().unwrap();
    assert!(header.is_file(), "{}", header.display());
    assert_eq!(
        header.parent(),
        Some(epocroot.join("epoc32/include/variant").as_path())
    );
    assert_ne!(Path::new(&header).file_name(), None);
}
