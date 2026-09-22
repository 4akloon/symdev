//! The fast `write!` against `core::write!`, call for call (experiment 99).
//!
//! Every case runs through both macros into a recording destination under every
//! failure mode ([`fast_write_support::modes`]) and into a `Buf16`-like destination
//! of every capacity; see [`same!`].

// `format!("{}", n = x)` is legal and warned about; the case is here on purpose.
#![allow(named_arguments_used_positionally)]
// Borrowed arguments are cases of their own (`Display for &T`).
#![allow(clippy::useless_borrows_in_formatting)]

#[macro_use]
mod fast_write_support;

use fast_write_support::Custom;

#[test]
fn literal_text_and_brace_escapes() {
    assert_eq!(same!("plain"), "plain");
    assert_eq!(same!("{{}} and {{{{"), "{} and {{");
    assert_eq!(
        same!("tab\tquote\"back\\slash\u{e9}\x41"),
        "tab\tquote\"back\\slash\u{e9}A"
    );
    assert_eq!(same!(r#"raw "{{}}" \n"#), "raw \"{}\" \\n");
    same!("a {{{}}} b", 5);
    same!(
        "line one\
           continued"
    );
}

#[test]
fn positional_named_and_captured() {
    let x = 7;
    let name = "who";
    assert_eq!(same!("{} {}", 1, 2), "1 2");
    assert_eq!(same!("{1} {0} {1}", "a", "b"), "b a b");
    assert_eq!(same!("{0}{}{}", x, name), "77who");
    assert_eq!(same!("{n} {n}", n = x), "7 7");
    assert_eq!(same!("{x} {name}"), "7 who");
    assert_eq!(same!("{x}{}{name}{x}", x + 1), "78who7");
    assert_eq!(same!("{} {n}", x, n = name), "7 who");
    let by_position = same!("{}", n = x);
    assert_eq!(by_position, "7");
    assert_eq!(same!("{:}", x), "7");
}

#[test]
fn strings_and_chars() {
    let s: &str = "slice";
    let owned = String::from("owned");
    let empty = "";
    assert_eq!(
        same!("[{}] [{}] [{}]", s, owned, empty),
        "[slice] [owned] []"
    );
    assert_eq!(same!("{}{}", &owned, &&s), "ownedslice");
    assert_eq!(
        same!("{}{}{}", 'c', '\u{e9}', '\u{1f600}'),
        "c\u{e9}\u{1f600}"
    );
    let mut m = String::from("mut");
    let r = &mut m;
    assert_eq!(same!("{}", r), "mut");
    assert_eq!(same!("{}", empty), "");
}

#[test]
fn every_integer_width_at_its_edges() {
    macro_rules! edges {
        ($($t:ty)*) => {$(
            for v in [<$t>::MIN, <$t>::MAX, 0 as $t, 1 as $t, <$t>::MAX / 10, (<$t>::MIN / 3)] {
                assert_eq!(same!("{}|{v}|", v), format!("{v}|{v}|"), stringify!($t));
            }
        )*};
    }
    edges!(u8 u16 u32 u64 usize i8 i16 i32 i64 isize u128 i128);
    for v in [
        9u64,
        10,
        99,
        100,
        4_294_967_295,
        4_294_967_296,
        10_000_000_000_000_000_000,
    ] {
        same!("{}", v);
    }
    for v in [-1i64, -9, -10, -4_294_967_296, i64::MIN + 1] {
        same!("{}", v);
    }
}

#[test]
fn format_specs_are_core_write_whole() {
    let n = 42;
    let s = "ab";
    assert_eq!(same!("[{:5}]", n), "[   42]");
    assert_eq!(same!("[{:<5}]", n), "[42   ]");
    assert_eq!(same!("[{:05}]", -n), "[-0042]");
    assert_eq!(same!("{:x} {:#x} {:X}", 255, 255, 255), "ff 0xff FF");
    assert_eq!(same!("{:.2}", 1.0f64 / 3.0), "0.33");
    assert_eq!(same!("{:?} {:?}", s, 'q'), "\"ab\" 'q'");
    assert_eq!(same!("{} {:?}", n, s), "42 \"ab\"");
    assert_eq!(same!("{:>1$}", s, 4), "  ab");
    assert_eq!(same!("{:.*}", 1, 2.25f32), "2.2");
    assert_eq!(same!("{n:>4}"), "  42");
    assert_eq!(same!("{0:?}{0}", s), "\"ab\"ab");
}

#[test]
fn types_that_are_not_on_the_list() {
    assert_eq!(same!("{} {}", true, 1.5f64), "true 1.5");
    assert_eq!(same!("[{}]", Custom(-3)), "[<-3>]");
    assert_eq!(same!("{}{}{}", Custom(1), 2, Custom(3)), "<1>2<3>");
    assert_eq!(same!("{}", u128::MAX), u128::MAX.to_string());
    assert_eq!(same!("{}", format_args!("in{}ner", 1)), "in1ner");
    let args = format_args!("{}-{}", 'a', "b");
    assert_eq!(same!("<{}>", args), "<a-b>");
}

#[test]
fn literal_arguments_fold_as_rustc_folds_them() {
    // Folded by rustc: string literals and integer literals that fit their type.
    same!("a{}b", "s");
    same!("a{}b", r"raw");
    same!("a{}b{}c", 5, 0x10);
    same!("a{}b", 1_000u16);
    same!("a{}b", (5));
    same!("a{}b", 2147483647);
    same!("a{}b", 18446744073709551615u64);
    same!("a{0}b{0}", "s");
    same!("a{x}b", x = "s");
    same!("{}{}", "", 'c');
    // Not folded: char, bool, float, a negative number, an out-of-range literal.
    same!("a{}b", 'c');
    same!("a{}b", true);
    same!("a{}b", -1);
    #[allow(overflowing_literals)]
    {
        same!("a{}b", 256u8);
        same!("a{}b", 4294967295);
    }
}

#[test]
fn trailing_commas_and_empty_strings() {
    same!("x",);
    same!("{}", 1,);
    same!("");
    same!("{}", "");
    same!("{}{}", "", "");
}

#[test]
fn writeln_with_and_without_arguments() {
    let v = -12;
    same_ln!();
    same_ln!("");
    same_ln!("done");
    same_ln!("v={}", v);
    same_ln!("v={v} {}", "tail",);
    same_ln!("{:>4}", v);
    same_ln!("{}", "");
}
