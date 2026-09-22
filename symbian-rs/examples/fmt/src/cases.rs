//! Every case runs at every capacity from 0 to 24 code units and at 64, so each one is
//! cut short at every piece boundary and inside every piece.

use alloc::string::String;
use core::fmt::{self, Write as _};
use core::hint::black_box;

use symbian_core::{Buf16, DesC16};
use symbian_std::test_report::Report;

/// A type with its own `Display`, which the fast path must leave to `core::fmt`.
struct Custom(i32);

impl fmt::Display for Custom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        fmt::Display::fmt(&self.0, f)?;
        f.write_char('>')
    }
}

const NAMES: [&str; 12] = [
    "text and escapes",
    "str, String and char",
    "i8, i16 and i32 at their edges",
    "i64 and isize at their edges",
    "u8, u16 and u32 at their edges",
    "u64 and usize at their edges",
    "positional, named and captured",
    "a user Display between fast pieces",
    "a format spec is core::write! whole",
    "writeln! with and without arguments",
    "literal arguments folded as rustc folds them",
    "negative numbers cut after the sign",
];

/// Compares one invocation through both macros into two fresh `Buf16<N>`s.
macro_rules! compare {
    ($diff:expr; $($t:tt)*) => {{
        let mut by_core = Buf16::<N>::new();
        let core_result = ::core::write!(by_core, $($t)*);
        let mut by_fast = Buf16::<N>::new();
        let fast_result = ::symbian_std::write!(by_fast, $($t)*);
        if core_result != fast_result || by_core.units() != by_fast.units() {
            $diff += 1;
        }
    }};
}

macro_rules! compare_ln {
    ($diff:expr; $($t:tt)*) => {{
        let mut by_core = Buf16::<N>::new();
        let core_result = ::core::writeln!(by_core $($t)*);
        let mut by_fast = Buf16::<N>::new();
        let fast_result = ::symbian_std::writeln!(by_fast $($t)*);
        if core_result != fast_result || by_core.units() != by_fast.units() {
            $diff += 1;
        }
    }};
}

/// Every case at capacity `N`; `diff[i]` counts the invocations of case `i` that
/// came out differently.
fn at<const N: usize>(diff: &mut [u32; 12]) {
    let s = black_box("slice");
    let owned = String::from(black_box("owned"));
    let (c, e, smile) = black_box(('c', '\u{e9}', '\u{1f600}'));
    compare!(diff[0]; "plain");
    compare!(diff[0]; "{{}} and {{{{ \u{e9}");
    compare!(diff[1]; "[{}] [{}] [{}]", s, owned, black_box(""));
    compare!(diff[1]; "{}{}{}", c, e, smile);
    compare!(diff[2]; "{}|{}|{}", black_box(i8::MIN), black_box(i8::MAX), black_box(0i8));
    compare!(diff[2]; "{}|{}", black_box(i16::MIN), black_box(i16::MAX));
    compare!(diff[2]; "{}|{}|{}", black_box(i32::MIN), black_box(i32::MAX), black_box(-1i32));
    compare!(diff[3]; "{}|{}", black_box(i64::MIN), black_box(i64::MAX));
    compare!(diff[3]; "{}|{}", black_box(-4_294_967_296i64), black_box(isize::MIN));
    compare!(diff[4]; "{}|{}|{}", black_box(u8::MAX), black_box(u16::MAX), black_box(u32::MAX));
    compare!(diff[4]; "{}", black_box(0u32));
    compare!(diff[5]; "{}|{}", black_box(u64::MAX), black_box(usize::MAX));
    compare!(diff[5]; "{}", black_box(9_223_372_036_854_775_808u64));
    let (x, name) = black_box((7, "who"));
    compare!(diff[6]; "{1}{0}{1}", x, name);
    compare!(diff[6]; "{x}-{name}-{n}", n = black_box(-3));
    compare!(diff[7]; "a{}b{}c", Custom(black_box(-12)), x);
    compare!(diff[8]; "[{:5}] [{:<4}] [{:05}] {:x} {:#x}", x, name, -x, 255, 255);
    compare!(diff[8]; "{:?} {:?}", name, smile);
    compare_ln!(diff[9];);
    compare_ln!(diff[9]; , "v={}", black_box(-12));
    compare!(diff[10]; "a{}b{}c", "s", 42);
    compare!(diff[10]; "a{}b", 'c');
    compare!(diff[11]; "ab{}", black_box(-123_456));
}

pub fn run(report: &mut Report) {
    let mut diff = [0u32; 12];
    macro_rules! every {
        ($($n:literal)*) => {$( at::<$n>(&mut diff); )*};
    }
    every!(0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 64);
    for (name, differ) in NAMES.iter().zip(diff) {
        report.check_detail(
            name,
            differ == 0,
            format_args!("{differ} mismatches over 26 capacities"),
        );
    }
}
