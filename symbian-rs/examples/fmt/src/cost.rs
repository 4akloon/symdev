//! What one invocation costs through each macro, in `NanoTicks` (1 000 µs each inside
//! EKA2L1, so the loop is long enough to move the counter). The emulator's ticks are
//! not the phone's; the ratio between the two loops is what is worth reading.

use alloc::string::String;
use core::fmt::Write as _;
use core::hint::black_box;

use symbian_core::Buf16;
use symbian_core::time::NanoTicks;
use symbian_std::test_report::Report;

const ROUNDS: u32 = 100_000;
/// A `const`, not a literal: rustc would fold a literal into the format string.
const WORD: &str = "abc";

fn ticks(mut body: impl FnMut(u32)) -> u32 {
    let start = NanoTicks::now().raw();
    for i in 0..ROUNDS {
        body(black_box(i));
    }
    NanoTicks::now().raw().wrapping_sub(start)
}

pub fn run(report: &mut Report) {
    let mut buf = Buf16::<64>::new();
    let core_buf = ticks(|i| {
        buf.clear();
        let _ = ::core::write!(buf, "i={} neg={} s={}", i, -(i as i32), WORD);
    });
    let fast_buf = ticks(|i| {
        buf.clear();
        let _ = ::symbian_std::write!(buf, "i={} neg={} s={}", i, -(i as i32), WORD);
    });
    let mut text = String::with_capacity(64);
    let core_string = ticks(|i| {
        text.clear();
        let _ = ::core::write!(text, "i={} neg={} s={}", i, -(i as i32), WORD);
    });
    let fast_string = ticks(|i| {
        text.clear();
        let _ = ::symbian_std::write!(text, "i={} neg={} s={}", i, -(i as i32), WORD);
    });
    report.check_detail(
        "ticks for 100000 writes into a Buf16",
        true,
        format_args!("core {core_buf}, fast {fast_buf}"),
    );
    report.check_detail(
        "ticks for 100000 writes into a String",
        true,
        format_args!("core {core_string}, fast {fast_string}"),
    );
}
