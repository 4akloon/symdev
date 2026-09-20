use super::*;

#[test]
fn the_shape_the_device_writes_parses() {
    let text = r#"{"schema":1,"app":"files","uid3":"0xe0000685","passed":1,"failed":1,
        "cases":[{"name":"write","ok":true},{"name":"read","ok":false,"detail":"KErrEof (-25)"}]}"#;
    let json = Json::parse(text).unwrap();
    assert_eq!(json.get("schema").and_then(Json::as_i64), Some(1));
    assert_eq!(json.get("app").and_then(Json::as_str), Some("files"));
    let cases = json.get("cases").and_then(Json::as_array).unwrap();
    assert_eq!(cases.len(), 2);
    assert_eq!(cases[1].get("ok").and_then(Json::as_bool), Some(false));
    assert_eq!(
        cases[1].get("detail").and_then(Json::as_str),
        Some("KErrEof (-25)")
    );
}

#[test]
fn escapes_and_utf8_come_back_as_written() {
    let json = Json::parse(r#"{"d":"a\"b\\c\ndAe — f"}"#).unwrap();
    assert_eq!(
        json.get("d").and_then(Json::as_str),
        Some("a\"b\\c\ndAe — f")
    );
}

#[test]
fn a_truncated_document_is_an_error_not_an_empty_one() {
    for text in [
        r#"{"schema":1,"cases":[{"name":"a","ok":tr"#,
        r#"{"schema":1,"app":"x"#,
        "",
        "{} trailing",
        r#"{"n":1.5}"#,
    ] {
        assert!(Json::parse(text).is_err(), "parsed {text:?}");
    }
}

#[test]
fn empty_containers_are_fine() {
    assert_eq!(Json::parse("{}").unwrap(), Json::Object(BTreeMap::new()));
    assert_eq!(Json::parse(" [ ] ").unwrap(), Json::Array(Vec::new()));
}
