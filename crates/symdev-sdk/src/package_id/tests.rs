use std::path::PathBuf;

use super::PackageId;
use crate::SdkError;

fn reason(id: &str) -> &'static str {
    match PackageId::parse(id) {
        Err(SdkError::InvalidId { id: shown, reason }) => {
            assert_eq!(shown, id);
            reason
        }
        other => panic!("`{id:?}` should be invalid, got {other:?}"),
    }
}

#[test]
fn parses_a_two_segment_id() {
    let id = PackageId::parse("gcce;12.1.0").unwrap();
    assert_eq!(id.as_str(), "gcce;12.1.0");
    assert_eq!(id.kind(), "gcce");
    assert_eq!(id.segments().collect::<Vec<_>>(), ["gcce", "12.1.0"]);
    assert_eq!(id.relative_path(), PathBuf::from("gcce").join("12.1.0"));
}

#[test]
fn parses_a_three_segment_id() {
    let id = PackageId::parse("sdk;s60-3rd-fp2;1.1").unwrap();
    assert_eq!(id.kind(), "sdk");
    assert_eq!(id.relative_path(), PathBuf::from("sdk/s60-3rd-fp2/1.1"));
}

#[test]
fn rejects_an_empty_id() {
    assert_eq!(reason(""), "is empty");
}

#[test]
fn rejects_a_single_segment() {
    assert_eq!(reason("gcce"), "needs a kind and a version");
}

#[test]
fn rejects_an_empty_segment() {
    assert_eq!(reason("gcce;;1"), "has an empty segment");
    assert_eq!(reason("gcce;1;"), "has an empty segment");
}

#[test]
fn rejects_dot_segments() {
    assert_eq!(reason("gcce;.."), "has a `.` or `..` segment");
    assert_eq!(reason("gcce;."), "has a `.` or `..` segment");
}

#[test]
fn rejects_path_separators_and_nul() {
    let path = "has a `/`, `\\` or NUL in a segment";
    assert_eq!(reason("gcce;a/b"), path);
    assert_eq!(reason("gcce;a\\b"), path);
    assert_eq!(reason("gcce;a\0b"), path);
}

#[test]
fn rejects_whitespace_and_control_characters() {
    let space = "has whitespace or a control character";
    assert_eq!(reason("gcce; 1"), space);
    assert_eq!(reason("gcce;1\t"), space);
    assert_eq!(reason("gcce;1\u{7}"), space);
}

#[test]
fn display_and_from_str_round_trip() {
    let id: PackageId = "sdk;s60-3rd-fp2;1.1".parse().unwrap();
    assert_eq!(id.to_string(), "sdk;s60-3rd-fp2;1.1");
    assert_eq!(id.to_string().parse::<PackageId>().unwrap(), id);
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
struct Wrapper {
    id: PackageId,
}

#[test]
fn serializes_as_the_plain_string() {
    let w = Wrapper {
        id: PackageId::parse("gcce;12.1.0").unwrap(),
    };
    let text = toml::to_string(&w).unwrap();
    assert_eq!(text.trim(), r#"id = "gcce;12.1.0""#);
    assert_eq!(toml::from_str::<Wrapper>(&text).unwrap(), w);
}

#[test]
fn deserializing_an_invalid_id_fails_with_the_reason() {
    let e = toml::from_str::<Wrapper>(r#"id = "gcce;..""#).unwrap_err();
    assert!(
        e.to_string().contains("invalid package id `gcce;..`"),
        "{e}"
    );
}

#[test]
fn quotes_itself_as_one_shell_word() {
    // `;` separates shell commands: an unquoted id in a suggested command would run
    // `symdev sdk install gcce` and then `12.1.0`.
    let id = PackageId::parse("gcce;12.1.0").unwrap();
    assert_eq!(id.shell_word(), "'gcce;12.1.0'");
    let quote = PackageId::parse("x;it's").unwrap();
    assert_eq!(quote.shell_word(), r"'x;it'\''s'");
}
