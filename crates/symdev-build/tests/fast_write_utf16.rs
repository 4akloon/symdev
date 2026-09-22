//! Text known at compile time as UTF-16 (experiment 106): `utf16!`, `Utf16Str`, and the
//! fast `write!`'s literal pieces, which a `Buf16`-like destination appends from the
//! UTF-16 in the image instead of transcoding the `&str` at run time.

#[macro_use]
mod fast_write_support;

use std::fmt::Write as _;

use fast_write_support::{HostBuf, Mode, Rec};
use symbian_fmt::__private::{encode_utf16, utf16_len};
use symbian_fmt::{Utf16Str, utf16};

/// Every width of UTF-8 at both of its edges, the surrogate range's neighbours, and
/// the last code point.
const EDGES: &str = "\0\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{fffd}\u{ffff}\u{10000}\u{1f600}\u{10ffff}";

#[test]
fn the_const_encoder_is_str_encode_utf16() {
    let owned: String = (0u32..0x11_0000)
        .step_by(401)
        .filter_map(char::from_u32)
        .collect();
    for s in ["", "a", "Hello from Rust SDK", "\u{e9}t\u{e9}", "\u{1f600}", EDGES, &owned] {
        let want: Vec<u16> = s.encode_utf16().collect();
        assert_eq!(utf16_len(s), want.len(), "{s:?}");
        let got = encode_utf16::<8192>(s);
        assert!(want.len() < got.len() && got[want.len()..].iter().all(|u| *u == 0));
        assert_eq!(&got[..want.len()], want.as_slice(), "{s:?}");
    }
}

const GREETING_STR: &str = "Hello from Rust SDK";
const GREETING: Utf16Str = utf16!(GREETING_STR);
const MIXED: Utf16Str = utf16!(concat!("caf\u{e9} ", "\u{1f600}"));
const EDGES16: Utf16Str = utf16!(EDGES);

#[test]
fn utf16_takes_a_literal_a_const_and_a_concat_at_compile_time() {
    for (u, s) in [
        (GREETING, GREETING_STR),
        (MIXED, "caf\u{e9} \u{1f600}"),
        (EDGES16, EDGES),
        (utf16!(""), ""),
    ] {
        assert_eq!(u.as_str(), s);
        assert_eq!(u.units(), s.encode_utf16().collect::<Vec<_>>().as_slice());
    }
    // It reads as the `str`: `len`, comparison, `Display` with and without flags,
    // and `Debug`.
    assert_eq!((GREETING.len(), &*GREETING == GREETING_STR), (19, true));
    assert_eq!(MIXED.chars().count(), 6);
    assert_eq!(
        format!("[{GREETING}] [{MIXED:>9}] [{MIXED:?}]"),
        format!("[{GREETING_STR}] [{:>9}] [{:?}]", MIXED.as_str(), MIXED.as_str())
    );
}

#[test]
fn a_utf16_str_argument_is_its_str_to_every_destination() {
    for mode in fast_write_support::modes() {
        let mut core = Rec::new(mode);
        let core_result = core::write!(core, "<{}|{}>", GREETING_STR, MIXED.as_str());
        let mut fast = Rec::new(mode);
        let fast_result = symbian_fmt::write!(fast, "<{GREETING}|{}>", MIXED);
        assert_eq!((fast.calls, fast_result), (core.calls, core_result), "{mode:?}");
    }
    for cap in 0..32 {
        let mut core = HostBuf::new(cap);
        let core_result = core::write!(core, "<{}|{}>", GREETING_STR, MIXED.as_str());
        let mut fast = HostBuf::new(cap);
        let fast_result = symbian_fmt::write!(fast, "<{GREETING}|{}>", MIXED);
        assert_eq!((&fast.units, fast_result), (&core.units, core_result), "{cap}");
    }
    // A spec sends the invocation to `core::write!`, which pads the `str`.
    assert_eq!(same!("[{:>24}]", GREETING), format!("[{GREETING_STR:>24}]"));
    let mut s = String::new();
    symbian_fmt::write!(s, "{GREETING} ({} chars)", GREETING.len()).unwrap();
    assert_eq!(s, "Hello from Rust SDK (19 chars)");
}

#[test]
fn literal_pieces_are_appended_from_their_utf16() {
    let n = 42;
    let mut fast = HostBuf::new(64);
    symbian_fmt::write!(fast, "a\u{e9}{n}\u{1f600}{GREETING}!").unwrap();
    assert_eq!(fast.text(), "a\u{e9}42\u{1f600}Hello from Rust SDK!");
    // "a\u{e9}", "\u{1f600}", "!" and the `Utf16Str` argument.
    assert_eq!((fast.native_literals, fast.native_numbers), (4, 1));
    let mut core = HostBuf::new(64);
    core::write!(core, "a\u{e9}{n}\u{1f600}{}!", GREETING_STR).unwrap();
    assert_eq!((core.units, core.native_literals), (fast.units, 0));
    // A `writeln!` newline, an escape and a folded literal argument are text too.
    let mut ln = HostBuf::new(64);
    symbian_fmt::writeln!(ln, "{{{}}}", "x").unwrap();
    assert_eq!((ln.text().as_str(), ln.native_literals), ("{x}\n", 1));
}

#[test]
fn literal_pieces_fail_as_core_fails_at_every_capacity() {
    // Two-unit characters straddle every capacity, and a failed piece writes nothing.
    assert_eq!(
        same!("\u{1f600}\u{e9}{}\u{1f600}\u{10ffff}", -7),
        "\u{1f600}\u{e9}-7\u{1f600}\u{10ffff}"
    );
    same!("{}{}", EDGES, 'x');
    same!("{EDGES}\u{7ff}\u{800}{}", 3u64);
    // And under every `Rec` mode, the literal's `write_str` is the one `core` makes.
    for mode in [Mode::Never, Mode::AtCall(1), Mode::Capacity(3)] {
        let mut core = Rec::new(mode);
        let core_result = core::write!(core, "\u{e9}t\u{e9} {}", 1);
        let mut fast = Rec::new(mode);
        let fast_result = symbian_fmt::write!(fast, "\u{e9}t\u{e9} {}", 1);
        assert_eq!((fast.calls, fast_result), (core.calls, core_result));
    }
}
