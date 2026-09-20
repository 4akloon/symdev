//! Time, in `std`'s shape (design spec §6a, §11 step 76).
//!
//! The cases are in [`checks`], and almost every name in them is `std`'s: `Duration`,
//! `Instant`, `SystemTime`, `UNIX_EPOCH`, `duration_since`, `elapsed`, `checked_add`.
//! The Symbian-shaped types appear only where `std` has no name for the thing at all —
//! to pace a measurement (`User::After`), to read the counters the SDK deliberately
//! does not build on, and to set the device clock.
//!
//! It reports through [`symbian_std::test_report`], and writes every raw number it
//! took to `E:\symdev\time76\measured.txt` so the host can compare its own clock with
//! the emulated one.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::time::{NanoTicks, SystemTicks};
use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::Report;
use symbian_std::time::{Duration, SystemTime, UNIX_EPOCH};

mod checks;

const DIR: &str = "E:\\symdev\\time76";
const NOTES: &str = "E:\\symdev\\time76\\measured.txt";

/// A window the wall clock has to be inside for anything to be believable:
/// 2020-01-01 to 2100-01-01, in Unix seconds.
pub(crate) const NOT_BEFORE: u64 = 1_577_836_800;
pub(crate) const NOT_AFTER: u64 = 4_102_444_800;

/// How many `Instant`s to take when checking that the clock never goes backwards.
pub(crate) const SAMPLES: usize = 20_000;

/// How many of those samples to take between one short sleep and the next. Without
/// the sleeps the whole loop finishes inside a single 15 625 µs tick — 20 000 readings
/// of a counter that never moved prove nothing at all, which is what the first run of
/// this example measured.
pub(crate) const SAMPLES_PER_SLEEP: usize = 200;

fn run(report: &mut Report, notes: &mut String) {
    let _ = writeln!(
        notes,
        "tick_period={:?} first_tick={} first_nano={}",
        SystemTicks::period_micros().map_err(|e| e.code()),
        SystemTicks::now().raw(),
        NanoTicks::now().raw()
    );
    // One tick of slack for `User::After`'s own rounding and one for the reading, at
    // the 15 625 µs period the platform reports.
    let slack = Duration::from_micros(2 * 15_625 + 1_000);
    checks::measure_sleep(report, notes, 1_000_000, slack);
    checks::measure_sleep(report, notes, 500_000, slack);
    checks::measure_sleep(report, notes, 100_000, slack);
    checks::never_goes_backwards(report, notes);
    checks::arithmetic(report);
    checks::wall_clock(report, notes);
    checks::a_clock_change(report, notes);
    // The last thing read, so that the host — which takes its own clock the moment
    // `symdev test` returns — has a reading to compare against with only the report
    // write and the emulator's shutdown in between.
    let _ = writeln!(
        notes,
        "unix_micros_at_end={}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs().saturating_mul(1_000_000) + d.subsec_micros() as u64)
            .unwrap_or(0)
    );
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("time");
    let mut notes = String::new();
    run(&mut report, &mut notes);
    report.checked("the measurements are written out", write_notes(&notes));
    Ok(if report.finish()? { 0 } else { 1 })
}

/// Leaves every raw number on the drive, where the host can read it and compare its
/// own clock with the emulated one.
fn write_notes(notes: &str) -> Result<()> {
    fs::create_dir_all(DIR)?;
    fs::write(NOTES, notes.as_bytes())
}
