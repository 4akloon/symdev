use super::*;

fn kinds(src: &str) -> Vec<RssToken> {
    RssLexer::new(src.as_bytes(), "t.rss")
        .tokens()
        .unwrap()
        .into_iter()
        .map(|t| t.token)
        .collect()
}

#[test]
fn lexes_resource_statement_with_comments_and_markers() {
    let toks = kinds(
        "# 1 \"a.rss\"\nNAME TEST // id\n/* c */ RESOURCE S r { b = 0x10; t = \"H\\\"i\"; d = -0.25; c = 'A'; }\n",
    );
    assert_eq!(toks[0], RssToken::Ident("NAME".into()));
    assert!(toks.contains(&RssToken::Int(16)));
    assert!(toks.contains(&RssToken::Str(vec![0x48, 0x22, 0x69])));
    assert!(toks.contains(&RssToken::Real(0.25)));
    assert!(toks.contains(&RssToken::Char(0x41)));
}

#[test]
fn line_markers_set_file_and_line() {
    let toks = RssLexer::new(b"# 7 \"Z:\\\\x\\\\y.rh\" 1\nfoo\n", "t")
        .tokens()
        .unwrap();
    assert_eq!(toks[0].file, "Z:\\x\\y.rh");
    assert_eq!(toks[0].line, 7);
}
