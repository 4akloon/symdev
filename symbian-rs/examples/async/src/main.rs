//! `async`/`await` on Symbian's active objects (design spec §11 step 73).
//!
//! The criterion this example exists for is one line of [`checks::two_at_once`]: two
//! 300 ms timers awaited **together** finish in about 300 ms, not 600, and the shorter
//! of two finishes first. Both numbers are measured with `symbian_std::time::Instant`
//! and written to `E:\symdev\async73\measured.txt` beside the report, so the claim is a
//! figure and not an assertion.
//!
//! Everything else here is the shape of the crate: `block_on` for a console
//! application, `spawn` for a task that joins the scheduler someone else runs, `join`
//! and `race`, and the refusals that keep the blocking model and this one apart.
//!
//! No peer, no network and no clock setting: an `RTimer` is all it needs, so this runs
//! anywhere `symdev test --emulator` runs.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::time::Duration;

use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::{Evidence, Report, detail};
use symbian_std::time::Instant;

mod checks;

const DIR: &str = "E:\\symdev\\async73";
const NOTES: &str = "E:\\symdev\\async73\\measured.txt";

/// The system tick is 15 625 µs (experiment 85) and `RTimer::After` rounds to a tick
/// boundary at both ends, so a 300 ms measurement may land two ticks either side.
pub(crate) const SLACK_MS: u64 = 2 * 16 + 10;

/// A `Duration` in whole milliseconds, without `as_millis`: that one goes through
/// `u128` division, which drags 128-bit arithmetic into the image for a number that
/// fits in 32 bits (experiment 85 measured 1 081 bytes for it).
pub(crate) fn millis(duration: Duration) -> u64 {
    duration.as_secs() * 1000 + u64::from(duration.subsec_millis())
}

/// An `Instant`, or a failed case saying why there is none. Every measurement here
/// starts with one, and `Instant::now` is fallible on this platform because the tick
/// period is read from the board at run time.
pub(crate) fn start_of(report: &mut Report, case: &str) -> Option<Instant> {
    match Instant::now() {
        Ok(instant) => Some(instant),
        Err(error) => {
            report.check_detail(case, false, detail!("Instant::now: {}", error.shown()));
            None
        }
    }
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("async");
    let mut notes = String::new();
    checks::run(&mut report, &mut notes);
    report.checked("the measurements are written out", write_notes(&notes));
    Ok(if report.finish()? { 0 } else { 1 })
}

/// Leaves every measured millisecond on the drive, where the host can read it back and
/// quote the number rather than the claim.
fn write_notes(notes: &str) -> Result<()> {
    fs::create_dir_all(DIR)?;
    fs::write(NOTES, notes.as_bytes())
}
