use super::super::tokens::CppSource;
use super::*;

fn run(defs: &[&str], text: &str) -> String {
    let mut m = CppMacros::default();
    for d in defs {
        m.define(&CppSource::tokens(d)).unwrap();
    }
    let out: String = m
        .expand(&CppSource::tokens(text))
        .unwrap()
        .iter()
        .map(CppToken::text)
        .collect();
    out.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn expands_object_and_function_macros() {
    assert_eq!(run(&["A 1", "B (A|2)"], "x = B;"), "x=(1|2);");
    assert_eq!(
        run(&["C(n,r) CTRL { c=(n); v=r; }"], "C(3, 0xff)"),
        "CTRL{c=(3);v=0xff;}"
    );
    assert_eq!(run(&["F(x) x"], "F"), "F");
}

#[test]
fn stringizes_pastes_and_stops_recursion() {
    assert_eq!(run(&["S(x) #x"], "S(ab)"), "\"ab\"");
    assert_eq!(run(&["P(a,b) a##b", "ab 7"], "P(a,b)"), "7");
    assert_eq!(run(&["R R+1"], "R"), "R+1");
}
