use super::{BUILTIN_URL, SourceSpec};
use crate::Auth;

fn https(base: &str) -> SourceSpec {
    SourceSpec::new("public", base, Auth::None).unwrap()
}

#[test]
fn adds_the_trailing_slash_and_names_the_index() {
    let s = https("https://pub-1.r2.dev");
    assert_eq!(s.base, "https://pub-1.r2.dev/");
    assert_eq!(s.index_url(), "https://pub-1.r2.dev/index.toml");
    assert!(!s.is_file());
    let f = SourceSpec::new("m", "file:///srv/m/", Auth::None).unwrap();
    assert_eq!(f.index_url(), "file:///srv/m/index.toml");
    assert!(f.is_file());
}

#[test]
fn an_invalid_name_or_url_names_itself() {
    let e = SourceSpec::new("Bad", "file:///x", Auth::None).unwrap_err();
    assert!(e.to_string().contains("`Bad`"), "{e}");
    let e = SourceSpec::new("m", "ftp://x/", Auth::None).unwrap_err();
    assert!(e.to_string().contains("ftp://x/"), "{e}");
}

#[test]
fn the_built_in_url_is_a_valid_public_source() {
    assert!(BUILTIN_URL.is_some());
    let source = SourceSpec::builtin().expect("the built-in URL parses as a source");
    assert_eq!(source.name, "public");
    assert_eq!(source.auth, Auth::None);
    assert_eq!(
        source.index_url(),
        format!("{}index.toml", BUILTIN_URL.unwrap_or(""))
    );
}

#[test]
fn resolves_a_relative_path_against_the_index_directory() {
    assert_eq!(
        https("https://x/b/").resolve("gcce/1/a.tar.gz").unwrap(),
        "https://x/b/gcce/1/a.tar.gz"
    );
    let mirror = SourceSpec::new("m", "file:///srv/mirror/", Auth::None).unwrap();
    assert_eq!(
        mirror.resolve("a..b.tar.gz").unwrap(),
        "file:///srv/mirror/a..b.tar.gz"
    );
}

#[test]
fn refuses_paths_that_leave_the_index_directory() {
    for relative in [
        "../a",
        "/a",
        "https://evil/a",
        "a/../b",
        "a/./b",
        "a//b",
        "",
        "a\\b",
    ] {
        let e = https("https://x/b/")
            .resolve(relative)
            .unwrap_err()
            .to_string();
        assert!(e.contains(&format!("`{relative}`")), "{relative}: {e}");
    }
}

#[test]
fn refuses_query_fragment_and_whitespace() {
    for relative in ["a?x=1", "a#f", "a b", "a\nb"] {
        assert!(
            https("https://x/b/").resolve(relative).is_err(),
            "{relative:?}"
        );
    }
}

#[test]
fn refuses_a_base_without_a_trailing_slash() {
    let mut s = https("https://x/b/");
    s.base = "https://x/b".into();
    let e = s.resolve("a").unwrap_err().to_string();
    assert!(e.contains("https://x/b"), "{e}");
}
