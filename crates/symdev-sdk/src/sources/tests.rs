use super::Sources;
use crate::{Auth, SdkError, SourceSpec};

const PATH: &str = "/home/u/.config/symdev/sources.toml";

fn public() -> SourceSpec {
    SourceSpec::new("public", "https://pub-1.r2.dev/", Auth::None).unwrap()
}

fn names(sources: &Sources) -> Vec<&str> {
    sources.list.iter().map(|s| s.name.as_str()).collect()
}

fn bad(text: &str) -> String {
    match Sources::parse(Some(text), PATH, Some(&public())) {
        Err(SdkError::BadSources { path, detail }) => {
            assert_eq!(path, PATH);
            detail
        }
        other => panic!("expected BadSources for {text:?}, got {other:?}"),
    }
}

const PRIVATE: &str = r#"
[[source]]
name = "private"
url = "https://acct.r2.cloudflarestorage.com/symdev-private"
auth = "s3"

[[source]]
name = "mirror-2"
url = "file:///srv/mirror/"
"#;

#[test]
fn without_a_file_only_the_builtin_source_is_used() {
    let sources = Sources::parse(None, PATH, Some(&public())).unwrap();
    assert_eq!(sources.list, [public()]);
}

#[test]
fn without_a_file_or_a_builtin_there_is_no_source() {
    let sources = Sources::parse(None, PATH, None).unwrap();
    assert!(sources.list.is_empty());
}

#[test]
fn listed_sources_follow_the_builtin_in_file_order() {
    let sources = Sources::parse(Some(PRIVATE), PATH, Some(&public())).unwrap();
    assert_eq!(names(&sources), ["public", "private", "mirror-2"]);
    let private = &sources.list[1];
    assert_eq!(private.auth, Auth::S3);
    assert_eq!(
        private.base,
        "https://acct.r2.cloudflarestorage.com/symdev-private/"
    );
    assert_eq!(sources.list[2].auth, Auth::None);
    assert_eq!(sources.list[2].base, "file:///srv/mirror/");
}

#[test]
fn builtin_false_disables_the_builtin_source() {
    let text = format!("builtin = false\n{PRIVATE}");
    let sources = Sources::parse(Some(&text), PATH, Some(&public())).unwrap();
    assert_eq!(names(&sources), ["private", "mirror-2"]);
    let only = Sources::parse(Some("builtin = false\n"), PATH, Some(&public())).unwrap();
    assert!(only.list.is_empty());
}

#[test]
fn a_duplicate_name_is_refused() {
    let text = format!("{PRIVATE}\n[[source]]\nname = \"private\"\nurl = \"file:///x\"\n");
    assert!(bad(&text).contains("`private`"));
    let builtin_name = "[[source]]\nname = \"public\"\nurl = \"file:///x\"\n";
    assert!(bad(builtin_name).contains("`public`"));
}

#[test]
fn a_name_outside_lowercase_digits_and_dashes_is_refused() {
    for name in ["Bad_Name", "", "a b", "a/b"] {
        let text = format!("[[source]]\nname = \"{name}\"\nurl = \"file:///x\"\n");
        assert!(bad(&text).contains("[a-z0-9-]"), "{name:?}");
    }
}

#[test]
fn only_https_local_http_and_file_urls_are_accepted() {
    for url in [
        "ftp://example.com/",
        "http://example.com/",
        "http://localhost.evil.com/",
        "https:///nohost",
        "file://relative/dir",
        "/srv/mirror",
    ] {
        let text = format!("[[source]]\nname = \"m\"\nurl = \"{url}\"\n");
        assert!(bad(&text).contains(url), "{url}");
    }
    for url in [
        "http://127.0.0.1:8080/b/",
        "http://localhost:9000",
        "http://localhost/",
        "https://pub-1.r2.dev",
    ] {
        let text = format!("[[source]]\nname = \"m\"\nurl = \"{url}\"\n");
        Sources::parse(Some(&text), PATH, None).unwrap();
    }
}

#[test]
fn s3_auth_needs_an_http_url() {
    let text = "[[source]]\nname = \"m\"\nurl = \"file:///x/\"\nauth = \"s3\"\n";
    assert!(bad(text).contains("s3"));
}

#[test]
fn an_unknown_auth_or_key_is_refused() {
    bad("[[source]]\nname = \"m\"\nurl = \"file:///x/\"\nauth = \"basic\"\n");
    bad("[[source]]\nname = \"m\"\nurl = \"file:///x/\"\nauht = \"s3\"\n");
    bad("bultin = false\n");
}

#[test]
fn a_toml_syntax_error_names_the_file() {
    bad("[[source]\n");
}
