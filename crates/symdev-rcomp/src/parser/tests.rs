use super::*;
use crate::lexer::RssLexer;

fn parse(src: &str) -> Vec<RssItem> {
    RssParser::new(
        RssLexer::new(src.as_bytes(), "t.rss")
            .unwrap()
            .tokens()
            .unwrap(),
    )
    .items()
    .unwrap()
}

#[test]
fn parses_struct_members_and_resource_values() {
    let items = parse(
        "NAME TEST\nenum X { EA = 1, EB, };\n\
         STRUCT S { LEN BYTE STRUCT items[]; BUF<9> b; LTEXT t(32) = \"\"; WORD w[] = { 1, 2 }; }\n\
         RESOURCE S r_x { items = { IN { y = 5; } }; b = \"a\" <0x2029> \"b\"; w = EA | 0x100; }\n",
    );
    assert_eq!(items[0], RssItem::Name("TEST".into()));
    let RssItem::Struct(s) = &items[2] else {
        panic!("{items:?}")
    };
    assert_eq!(s.members[0].len_prefix, Some(RssWidth::Byte));
    assert_eq!(s.members[0].array, Some(None));
    assert!(matches!(s.members[1].max_len, Some(RssExpr::Int(9))));
    assert!(matches!(s.members[2].max_len, Some(RssExpr::Int(32))));
    let RssItem::Resource(r) = &items[3] else {
        panic!("{items:?}")
    };
    assert_eq!(r.name.as_deref(), Some("r_x"));
    assert!(matches!(r.value.fields[1].1, RssValue::Text(ref p) if p.len() == 3));
}
