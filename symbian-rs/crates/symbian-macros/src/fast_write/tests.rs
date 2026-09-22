//! The pure half of `write_pieces!`: literals, segments, plans and the expansion text.
//! What the expansion *does* is tested on the host against `core::write!`, call for
//! call, in `crates/symdev-build/tests/fast_write*.rs`.

use super::expand::expansion;
use super::literal::{folded_integer, string_value};
use super::plan::{Explicit, Piece, Plan, Source};
use super::template::{Hole, Segment, segments};

fn positional(n: usize) -> Vec<Explicit> {
    vec![Explicit { name: None, folded: None }; n]
}

fn named(name: &str) -> Explicit {
    Explicit { name: Some(name.into()), folded: None }
}

fn plan(format: &str, explicit: &[Explicit]) -> Option<Plan> {
    Plan::new(&segments(format)?, explicit, false)
}

#[test]
fn string_literals_read_as_their_values() {
    assert_eq!(string_value(r#""a\n\t\\\"\x41\u{e9}\u{1_F600}""#).as_deref(), Some("a\n\t\\\"A\u{e9}\u{1f600}"));
    assert_eq!(string_value("\"one\\\n     two\"").as_deref(), Some("onetwo"));
    assert_eq!(string_value(r####"r##"a "# b"##"####).as_deref(), Some("a \"# b"));
    assert_eq!(string_value(r#"r"\n""#).as_deref(), Some("\\n"));
    for not_a_str in [r#"b"x""#, r#"c"x""#, "'x'", "5", r#""\x80""#, r#""\q""#] {
        assert_eq!(string_value(not_a_str), None, "{not_a_str}");
    }
}

#[test]
fn integer_literals_fold_only_when_they_fit_their_type() {
    let cases = [
        ("5", Some("5")), ("0x10", Some("16")), ("0b101", Some("5")), ("0o17", Some("15")),
        ("1_000", Some("1000")), ("255u8", Some("255")), ("256u8", None), ("2147483647", Some("2147483647")),
        ("2147483648", None), ("18446744073709551615u64", Some("18446744073709551615")),
        ("4294967296usize", None), ("1.5", None), ("1e3", None), ("5f32", None), ("0x1f32", Some("7986")),
    ];
    for (source, want) in cases {
        assert_eq!(folded_integer(source).as_deref(), want, "{source}");
    }
}

#[test]
fn only_plain_holes_are_taken_apart() {
    assert_eq!(
        segments("a{{{}}}b{0}{x}{:}").unwrap(),
        vec![
            Segment::Text("a{".into()),
            Segment::Hole(Hole::Next),
            Segment::Text("}b".into()),
            Segment::Hole(Hole::Index(0)),
            Segment::Hole(Hole::Name("x".into())),
            Segment::Hole(Hole::Next),
        ]
    );
    for core_only in ["{:5}", "{:?}", "{:x}", "{0:.2}", "{:>1$}", "{", "}", "{ }", "{self}", "{_}", "{r#x}", "{01}", "{é}"] {
        assert_eq!(segments(core_only), None, "{core_only}");
    }
}

#[test]
fn plans_follow_format_args_rules() {
    let p = plan("{1}-{}-{n}-{x}-{x}", &[positional(2), vec![named("n")]].concat()).unwrap();
    assert_eq!(
        p.slots,
        vec![Source::Explicit(0), Source::Explicit(1), Source::Explicit(2), Source::Capture("x".into())]
    );
    assert_eq!(
        p.pieces,
        vec![
            Piece::Arg(1), Piece::Text("-".into()), Piece::Arg(0), Piece::Text("-".into()), Piece::Arg(2),
            Piece::Text("-".into()), Piece::Arg(3), Piece::Text("-".into()), Piece::Arg(3),
        ]
    );
    // Rejected by `format_args!`, so left to it.
    assert_eq!(plan("{}", &positional(2)), None, "an unused argument");
    assert_eq!(plan("{2}", &positional(2)), None, "an index past the end");
    assert_eq!(plan("{}{}", &positional(1)), None, "too few arguments");
    assert_eq!(plan("{n}", &[named("n"), Explicit { name: None, folded: None }]), None, "positional after named");
    assert_eq!(plan("{n}", &[named("n"), named("n")]), None, "a name given twice");
    assert_eq!(plan("{}", &[Explicit { name: None, folded: Some(String::new()) }]), None, "nothing to write");
}

#[test]
fn folded_literals_merge_into_the_text() {
    let folded = |text: &str| Explicit { name: None, folded: Some(text.into()) };
    let p = Plan::new(&segments("a{}b{}c{}").unwrap(), &[folded("X"), positional(1)[0].clone(), folded("")], true).unwrap();
    assert_eq!(p.pieces, vec![Piece::Text("aXb".into()), Piece::Arg(0), Piece::Text("c\n".into())]);
    assert_eq!(p.slots, vec![Source::Explicit(1)]);
}

#[test]
fn the_expansion_calls_the_destination_once_and_probes_every_piece() {
    let text = expansion(&plan("n={} {x}", &positional(1)).unwrap());
    assert_eq!(text.matches("__symbian_fmt_enter").count(), 1, "{text}");
    assert!(text.contains("(@dst).__symbian_fmt_enter((@slot0, @slot1, ), |__d, (__a0, __a1, )|"), "{text}");
    assert_eq!(text.matches("Probe::of").count(), 4, "{text}");
    assert!(text.contains("Probe::of(&*__d, \"n=\")"), "{text}");
    assert!(text.ends_with("::core::result::Result::Ok(()) }) }"), "{text}");
}
