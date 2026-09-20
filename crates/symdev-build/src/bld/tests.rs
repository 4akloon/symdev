use crate::model::BldInf;

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
    parse("PRJ_PLATFORMS\nWINSCW ARMV5 GCCE\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn winscw_only_rejected() {
    assert!(parse("PRJ_PLATFORMS\nWINSCW\nPRJ_MMPFILES\na.mmp\n").is_err());
}

#[test]
fn unknown_section_errors() {
    assert!(parse("PRJ_EXTENSIONS\n").is_err());
}

#[test]
fn test_mmpfiles_collected_separately() {
    let b = parse("PRJ_MMPFILES\na.mmp\nPRJ_TESTMMPFILES\nt.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
    assert_eq!(b.test_mmp_files, [std::path::PathBuf::from("t.mmp")]);
}

#[test]
fn empty_platforms_errors() {
    let err = parse("PRJ_PLATFORMS\n\nPRJ_MMPFILES\na.mmp\n").unwrap_err();
    assert!(
        err.to_string()
            .contains("PRJ_PLATFORMS has no usable platform")
    );
}

#[test]
fn default_token_accepted() {
    parse("PRJ_PLATFORMS\nDEFAULT\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn armv5_abiv2_token_accepted() {
    parse("PRJ_PLATFORMS\nARMV5_ABIV2\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn exports_retained() {
    let b = parse("PRJ_EXPORTS\nfoo.h\nPRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.exports, ["foo.h"]);
}

/// §1.6: `PRJ_EXPORTS` comes from the pass with no macros, `PRJ_MMPFILES` from the one
/// with the platform's.
#[test]
fn each_section_comes_from_its_own_pass() {
    let b = BldInf::parse("PRJ_EXPORTS\nfoo.h\n", "PRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.exports, ["foo.h"]);
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
}
