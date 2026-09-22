//! The fast `write!` on the phone (experiment 100): the same invocations through
//! `core::write!` and `symbian_std::write!` into a `Buf16` of every small capacity,
//! compared unit for unit together with the `fmt::Result`, and what each costs in
//! ticks over 100 000 rounds.
//!
//! The host tests compare the two macros call for call on the `fmt::Write` path
//! (`crates/symdev-build/tests/fast_write*.rs`). What only the phone can check is the
//! `Buf16` path, where the numbers are euser's `TDes16::AppendNum` in ROM and a
//! buffer that is too small has to fail exactly where `core` fails — a lone `-`
//! included.
#![no_std]

extern crate alloc;

mod cases;
mod cost;

use symbian_std::io::Result;
use symbian_std::test_report::Report;

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("fmt");
    cases::run(&mut report);
    cost::run(&mut report);
    Ok(if report.finish()? { 0 } else { 1 })
}
