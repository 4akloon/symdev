//! The cases `examples/time` reports, kept out of `main.rs` so that neither file
//! grows past the 300 lines this SDK allows.
use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::time::Ttime;
use symbian_core::user::after;
use symbian_std::test_report::Report;
use symbian_std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::{NOT_AFTER, NOT_BEFORE, SAMPLES, SAMPLES_PER_SLEEP};

/// `duration` in whole microseconds, and in whole milliseconds.
///
/// Not `Duration::as_micros`/`as_millis`, which return `u128`: 128-bit division and
/// 128-bit formatting are `compiler_builtins` routines this image would otherwise
/// never carry, and experiment 85 measured what they cost.
fn micros(duration: Duration) -> u64 {
    duration
        .as_secs()
        .saturating_mul(1_000_000)
        .saturating_add(duration.subsec_micros() as u64)
}

fn millis(duration: Duration) -> u64 {
    micros(duration) / 1_000
}

/// Measures one `User::After` sleep with an `Instant` and checks it lands within
/// `slack` of what was asked for.
///
/// `slack` is stated by the caller rather than derived, because the resolution is the
/// platform's: one tick is 15 625 µs, so nothing here can be tighter than that plus
/// whatever `User::After` itself rounds up by.
pub(crate) fn measure_sleep(
    report: &mut Report,
    notes: &mut String,
    asked_micros: i32,
    slack: Duration,
) {
    let Some(started) = report.checked("Instant::now before a sleep", Instant::now()) else {
        return;
    };
    after(asked_micros);
    let elapsed = started.elapsed();
    let asked = Duration::from_micros(asked_micros as u64);
    let low = asked.saturating_sub(slack);
    let high = asked.saturating_add(slack);
    let _ = writeln!(
        notes,
        "after={asked_micros} elapsed_micros={} window={}..{}",
        micros(elapsed),
        micros(low),
        micros(high)
    );
    let mut name = String::new();
    let _ = write!(
        name,
        "a {} ms sleep measures {} ms, within +/-{} ms",
        millis(asked),
        millis(elapsed),
        millis(slack)
    );
    report.check(&name, elapsed >= low && elapsed <= high);
}

/// Takes `SAMPLES` instants in a row and checks that not one of them is earlier than
/// the one before it.
pub(crate) fn never_goes_backwards(report: &mut Report, notes: &mut String) {
    let Some(mut previous) = report.checked("Instant::now before the samples", Instant::now())
    else {
        return;
    };
    let first = previous;
    let mut backwards = 0u32;
    let mut moved = 0u32;
    for i in 0..SAMPLES {
        if i % SAMPLES_PER_SLEEP == 0 {
            // `User::After` rounds up to a tick boundary, so every one of these moves
            // the counter at least once.
            after(2_000);
        }
        let Ok(current) = Instant::now() else {
            backwards += 1;
            continue;
        };
        match current.checked_duration_since(previous) {
            None => backwards += 1,
            Some(d) if !d.is_zero() => moved += 1,
            Some(_) => {}
        }
        previous = current;
    }
    let _ = writeln!(
        notes,
        "samples={SAMPLES} backwards={backwards} moved={moved} \
         span_micros={} first_ticks={} last_ticks={} period_micros={}",
        micros(previous.duration_since(first)),
        first.raw_ticks(),
        previous.raw_ticks(),
        previous.tick_period_micros()
    );
    let mut name = String::new();
    let _ = write!(
        name,
        "{SAMPLES} instants over {} ms, none of them backwards",
        millis(previous.duration_since(first))
    );
    report.check(&name, backwards == 0);
    // A run in which the counter never ticked would pass the line above without
    // having tested anything, so the tick transitions are a case of their own.
    let mut name = String::new();
    let _ = write!(name, "and the counter really moved, {moved} times");
    report.check(&name, moved > 0);
}

/// `Instant` and `SystemTime` arithmetic, all of it `std`'s.
pub(crate) fn arithmetic(report: &mut Report) {
    let Some(now) = report.checked("Instant::now for the arithmetic", Instant::now()) else {
        return;
    };
    let second = Duration::from_secs(1);
    if let Some(later) = report.checked("Instant::checked_add", now.checked_add(second).ok_or(())) {
        report.check(
            "an Instant a second on is a second on",
            later - now == second,
        );
        report.check(
            "and the other way round is None",
            later.checked_duration_since(now).is_some()
                && now.checked_duration_since(later).is_none(),
        );
        report.check(
            "so duration_since saturates to zero",
            now.duration_since(later).is_zero(),
        );
    }
    report.check(
        "checked_sub gives the second back",
        now.checked_sub(second)
            .and_then(|earlier| now.checked_duration_since(earlier))
            == Some(second),
    );
    report.check(
        "a duration past half the wrap window is None",
        now.checked_add(Duration::from_secs(60 * 60 * 24 * 400))
            .is_none(),
    );

    let wall = SystemTime::now();
    let five = Duration::from_secs(5);
    report.check(
        "SystemTime + 5s is 5s later",
        (wall + five).duration_since(wall) == Ok(five),
    );
    report.check(
        "and 5s earlier is an Err that says how far",
        wall.duration_since(wall + five).map_err(|e| e.duration()) == Err(five),
    );
    report.check("SystemTime is Ord", wall < wall + five);
    report.check(
        "SystemTime saturates instead of panicking",
        wall.checked_add(Duration::MAX).is_none(),
    );
}

/// The wall clock: where it says we are, and that it is UTC.
pub(crate) fn wall_clock(report: &mut Report, notes: &mut String) {
    let now = SystemTime::now();
    let Some(since_epoch) =
        report.checked("duration_since(UNIX_EPOCH)", now.duration_since(UNIX_EPOCH))
    else {
        return;
    };
    let home = Ttime::home().micros_since_year_zero();
    let _ = writeln!(
        notes,
        "unix_micros={} unix_secs={} ttime_universal={} ttime_home={} home_minus_universal={}",
        micros(since_epoch),
        since_epoch.as_secs(),
        now.as_ttime().micros_since_year_zero(),
        home,
        home - now.as_ttime().micros_since_year_zero()
    );
    let mut name = String::new();
    let _ = write!(
        name,
        "the clock says {} Unix seconds, which is this century",
        since_epoch.as_secs()
    );
    report.check(
        &name,
        since_epoch.as_secs() > NOT_BEFORE && since_epoch.as_secs() < NOT_AFTER,
    );
    report.check(
        "UNIX_EPOCH is 1970-01-01 in TTime microseconds",
        UNIX_EPOCH.as_ttime().micros_since_year_zero() == 62_168_256_000_000_000,
    );
    report.check("elapsed since a moment ago is Ok", now.elapsed().is_ok());
}

/// The property the whole split between the two types exists for: setting the device
/// clock moves `SystemTime` and does not move `Instant`.
pub(crate) fn a_clock_change(report: &mut Report, notes: &mut String) {
    let Some(started) = report.checked("Instant::now before the clock change", Instant::now())
    else {
        return;
    };
    let before = SystemTime::now();
    let hour = Duration::from_secs(3600);
    let Some(moved) = before.checked_add(hour) else {
        return;
    };
    let outcome = moved.as_ttime().set_universal();
    let _ = writeln!(
        notes,
        "set_universal={:?} before={} ",
        outcome.map_err(|e| e.code()),
        before.as_ttime().micros_since_year_zero()
    );
    if outcome.is_err() {
        // The platform refused; say which platform and which code, and do not pretend
        // the property was tested.
        let mut name = String::new();
        let _ = write!(
            name,
            "the device clock cannot be set from here ({:?}), so a clock change is untested",
            outcome.map_err(|e| e.code())
        );
        report.check(&name, true);
        return;
    }
    let after_change = SystemTime::now();
    let elapsed = started.elapsed();
    let _ = writeln!(
        notes,
        "after_change={} instant_elapsed_micros={}",
        after_change.as_ttime().micros_since_year_zero(),
        micros(elapsed)
    );
    report.check(
        "the wall clock jumped an hour",
        after_change
            .duration_since(before)
            .unwrap_or(Duration::ZERO)
            >= hour,
    );
    report.check(
        "and the Instant did not move with it",
        elapsed < Duration::from_secs(10),
    );
    // Put it back, allowing for however long this took.
    let restored = before.checked_add(elapsed).unwrap_or(before);
    report.checked("the clock is put back", restored.as_ttime().set_universal());
}
