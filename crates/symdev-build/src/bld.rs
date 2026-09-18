use std::path::PathBuf;

use crate::model::BldInf;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParseError(pub(crate) String);

enum Section {
    None,
    Platforms,
    Exports,
    MmpFiles,
    TestMmpFiles,
}

const USABLE_PLATFORMS: &[&str] = &["GCCE", "ARMV5", "ARMV5_ABIV2", "DEFAULT"];

const PREPROCESSOR: &[&str] = &["if", "ifdef", "ifndef", "elif", "else", "endif", "include"];

fn usable_platform(tok: &str) -> bool {
    USABLE_PLATFORMS.iter().any(|p| tok.eq_ignore_ascii_case(p))
}

fn known_directive(tok: &str) -> bool {
    tok.eq_ignore_ascii_case("PRJ_PLATFORMS")
        || tok.eq_ignore_ascii_case("PRJ_EXPORTS")
        || tok.eq_ignore_ascii_case("PRJ_MMPFILES")
        || tok.eq_ignore_ascii_case("PRJ_TESTMMPFILES")
}

fn preprocessor(tok: &str) -> bool {
    let Some(name) = tok.strip_prefix('#') else {
        return false;
    };
    PREPROCESSOR.iter().any(|p| name.eq_ignore_ascii_case(p))
}

impl BldInf {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut mmp_files = Vec::new();
        let mut test_mmp_files = Vec::new();
        let mut exports = Vec::new();
        let mut platforms: Option<Vec<String>> = None;
        let mut section = Section::None;

        for raw in text.lines() {
            let line = match raw.find("//") {
                Some(i) => &raw[..i],
                None => raw,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with('#') {
                let Some(tok) = line.split_whitespace().next() else {
                    continue;
                };
                if preprocessor(tok) {
                    return Err(ParseError(format!("unsupported preprocessor: {tok}")));
                }
                continue;
            }

            let Some(tok) = line.split_whitespace().next() else {
                continue;
            };
            if known_directive(tok) {
                if tok.eq_ignore_ascii_case("PRJ_PLATFORMS") {
                    section = Section::Platforms;
                    platforms.get_or_insert_with(Vec::new);
                } else if tok.eq_ignore_ascii_case("PRJ_EXPORTS") {
                    section = Section::Exports;
                } else if tok.eq_ignore_ascii_case("PRJ_MMPFILES") {
                    section = Section::MmpFiles;
                } else {
                    section = Section::TestMmpFiles;
                }
                continue;
            }

            if tok.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("PRJ_")) {
                return Err(ParseError(format!("unknown directive: {tok}")));
            }

            match section {
                Section::None => {
                    return Err(ParseError(format!("unknown directive: {tok}")));
                }
                Section::Platforms => {
                    platforms
                        .get_or_insert_with(Vec::new)
                        .extend(line.split_whitespace().map(str::to_string));
                }
                Section::Exports => exports.push(line.to_string()),
                Section::MmpFiles => mmp_files.push(PathBuf::from(line)),
                Section::TestMmpFiles => test_mmp_files.push(PathBuf::from(line)),
            }
        }

        if let Some(tokens) = &platforms {
            if !tokens.iter().any(|t| usable_platform(t)) {
                return Err(ParseError("PRJ_PLATFORMS has no usable platform".into()));
            }
        }

        Ok(Self {
            mmp_files,
            test_mmp_files,
            exports,
        })
    }
}

#[test]
fn omitted_platforms_collects_mmp() {
    let b = BldInf::parse("PRJ_MMPFILES\nhello.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("hello.mmp")]);
    assert!(b.test_mmp_files.is_empty());
}

#[test]
fn gcce_token_accepted() {
    BldInf::parse("PRJ_PLATFORMS\nWINSCW ARMV5 GCCE\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn winscw_only_rejected() {
    assert!(BldInf::parse("PRJ_PLATFORMS\nWINSCW\nPRJ_MMPFILES\na.mmp\n").is_err());
}

#[test]
fn ifdef_is_parse_error() {
    assert!(BldInf::parse("#ifdef EKA2\nPRJ_MMPFILES\na.mmp\n#endif\n").is_err());
}

#[test]
fn unknown_directive_errors() {
    assert!(BldInf::parse("PRJ_EXTENSIONS\n").is_err());
}

#[test]
fn test_mmpfiles_collected_separately() {
    let b = BldInf::parse("PRJ_MMPFILES\na.mmp\nPRJ_TESTMMPFILES\nt.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("a.mmp")]);
    assert_eq!(b.test_mmp_files, [std::path::PathBuf::from("t.mmp")]);
}

#[test]
fn comments_are_ignored() {
    let b = BldInf::parse("# comment\nPRJ_MMPFILES // trailing\nhello.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("hello.mmp")]);
}

#[test]
fn empty_platforms_errors() {
    let err = BldInf::parse("PRJ_PLATFORMS\n\nPRJ_MMPFILES\na.mmp\n").unwrap_err();
    assert!(
        err.to_string()
            .contains("PRJ_PLATFORMS has no usable platform")
    );
}

#[test]
fn default_token_accepted() {
    BldInf::parse("PRJ_PLATFORMS\nDEFAULT\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn armv5_abiv2_token_accepted() {
    BldInf::parse("PRJ_PLATFORMS\nARMV5_ABIV2\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn exports_retained() {
    let b = BldInf::parse("PRJ_EXPORTS\nfoo.h\nPRJ_MMPFILES\na.mmp\n").unwrap();
    assert_eq!(b.exports, ["foo.h"]);
}
