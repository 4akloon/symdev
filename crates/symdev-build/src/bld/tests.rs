use crate::model::{BldExport, BldInf};

/// A `bld.inf` with no conditionals: both preprocessor passes give the same text.
fn parse(text: &str) -> Result<BldInf, crate::bld::ParseError> {
    BldInf::parse(text, text)
}

#[test]
fn omitted_platforms_collects_mmp() {
    let b = parse("PRJ_MMPFILES\nhello.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("hello.mmp")]);
    assert!(b.test_mmp_files.is_empty());
}

#[test]
fn gcce_token_accepted() {
    let b = parse("PRJ_PLATFORMS\nWINSCW ARMV5 GCCE\nPRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.platforms, ["WINSCW", "ARMV5", "GCCE"]);
}

/// §4.2/§4.6: GCCE is an optional platform the SDK appends to every component, so a
/// list that omits it must not veto the build.
#[test]
fn a_platform_list_without_gcce_still_builds() {
    let b = parse("PRJ_PLATFORMS\nWINSCW\nPRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.platforms, ["WINSCW", "GCCE"]);
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
}

#[test]
fn default_expands_and_a_minus_removes() {
    let b = parse("PRJ_PLATFORMS\nDEFAULT -EDG\nPRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.platforms, ["WINSCW", "GCCXML", "GCCE"]);
    assert!(parse("PRJ_PLATFORMS\n-EDG\nPRJ_MMPFILES\na.mmp\n").is_err());
    assert!(parse("PRJ_PLATFORMS\nVC6\nPRJ_MMPFILES\na.mmp\n").is_err());
}

#[test]
fn unknown_section_errors() {
    assert!(parse("PRJ_NONSENSE\n").is_err());
}

#[test]
fn test_mmpfiles_collected_separately() {
    let b = parse("PRJ_MMPFILES\na.mmp\nPRJ_TESTMMPFILES\nt.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
    assert_eq!(b.test_mmp_files, [std::path::PathBuf::from("t.mmp")]);
}

#[test]
fn armv5_abiv2_token_accepted() {
    parse("PRJ_PLATFORMS\nARMV5_ABIV2\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn exports_are_parsed_with_their_destination() {
    let b = parse("PRJ_EXPORTS\nfoo.h\n..\\inc\\bar.h \\epoc32\\include\\bar.h\n:zip x.zip\nPRJ_MMPFILES\na.mmp\n")
        .unwrap();
    assert_eq!(
        b.exports,
        [
            BldExport {
                source: "foo.h".into(),
                dest: None,
                zip: false
            },
            BldExport {
                source: "..\\inc\\bar.h".into(),
                dest: Some("\\epoc32\\include\\bar.h".into()),
                zip: false
            },
            BldExport {
                source: "x.zip".into(),
                dest: None,
                zip: true
            },
        ]
    );
    assert!(parse("PRJ_EXPORTS\na.h b.h c.h\n").is_err());
    assert!(parse("PRJ_EXPORTS\n:tar x.tar\n").is_err());
}

/// §1.6: `PRJ_EXPORTS` comes from the pass with no macros, `PRJ_MMPFILES` from the one
/// with the platform's.
#[test]
fn each_section_comes_from_its_own_pass() {
    let b = BldInf::parse("PRJ_EXPORTS\nfoo.h\n", "PRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.exports.len(), 1);
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
}

/// Gap 5: a `gnumakefile` line must not be collected as an MMP path.
#[test]
fn a_makefile_hand_off_is_refused_by_name() {
    for line in [
        "gnumakefile Icons_scalable_dc.mk",
        "makefile build.mak",
        "nmakefile build.mak",
    ] {
        let err = parse(&format!("PRJ_MMPFILES\n{line}\n"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("makefile") || err.contains("MAKEFILE"),
            "{err}"
        );
        assert!(err.to_lowercase().contains("make"), "{err}");
    }
}

#[test]
fn qualifiers_are_checked_and_ignore_drops_the_line() {
    let b = parse("PRJ_MMPFILES\na.mmp TIDY\nb.mmp IGNORE\nc.mmp BUILD_AS_ARM\n").unwrap();
    assert_eq!(
        b.mmp_files,
        [
            std::path::PathBuf::from("a.mmp"),
            std::path::PathBuf::from("c.mmp")
        ]
    );
    // `BLD` is not a qualifier of this SDK's bld.inf parser.
    assert!(parse("PRJ_MMPFILES\na.mmp BLD\n").is_err());
    assert!(parse("PRJ_MMPFILES\na.mmp NONSENSE\n").is_err());
}

#[test]
fn an_extension_block_is_refused_by_name() {
    let err = parse("PRJ_EXTENSIONS\nSTART EXTENSION s60/mifconv\nEND\n")
        .unwrap_err()
        .to_string();
    assert!(err.contains("s60/mifconv"), "{err}");
    assert!(err.contains("makefile_templates"), "{err}");
}

#[test]
fn a_section_header_takes_the_whole_line() {
    assert!(parse("PRJ_MMPFILES a.mmp\n").is_err());
}
