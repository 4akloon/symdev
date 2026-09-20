//! Throwaway calendar probe for experiment 85 — NOT the finished example.
//!
//! It asks **euser's own** calendar code (`Time::LeapYearsUpTo`, `Time::IsLeapYear`
//! and `TTime`'s const accessors, all of which run the real ROM DLL and not EKA2L1
//! code) what calendar `TTime` keeps, so the Unix epoch offset is established from
//! Symbian rather than from the emulator's idea of what time it is.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::time::Ttime;
use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::Report;

const DIR: &str = "E:\\symdev\\time76";
const PROBE: &str = "E:\\symdev\\time76\\calendar.txt";

/// Candidate offsets, in microseconds, from the `TTime` origin to 1970-01-01T00:00Z.
/// The first is proleptic Gregorian arithmetic (719 528 days), the second is what the
/// emulator's clock implies (719 540 days).
const CANDIDATES: [(&str, i64); 2] = [
    ("proleptic_719528", 62_167_219_200_000_000),
    ("emulator_719540", 62_168_256_000_000_000),
];

unsafe extern "C" {
    #[link_name = "_ZN4Time13LeapYearsUpToEi"]
    fn Time_LeapYearsUpTo(year: i32) -> i32;
    #[link_name = "_ZN4Time10IsLeapYearEi"]
    fn Time_IsLeapYear(year: i32) -> i32;
    #[link_name = "_ZNK5TTime11DayNoInYearEv"]
    fn TTime_DayNoInYear(this: *const i64) -> i32;
    #[link_name = "_ZNK5TTime12DayNoInMonthEv"]
    fn TTime_DayNoInMonth(this: *const i64) -> i32;
    #[link_name = "_ZNK5TTime11DayNoInWeekEv"]
    fn TTime_DayNoInWeek(this: *const i64) -> i32;
    #[link_name = "_ZNK5TTime11DaysInMonthEv"]
    fn TTime_DaysInMonth(this: *const i64) -> i32;
}

fn decode(out: &mut String, name: &str, micros: i64) {
    let time = micros;
    // SAFETY (probe): const, non-virtual, non-static members of a class whose only
    // storage is one TInt64, called with `this` as argument 0 per experiment 78. None
    // of them is an `L` function and each returns a TInt.
    let (year_day, month_day, week_day, month_len) = unsafe {
        (
            TTime_DayNoInYear(&time),
            TTime_DayNoInMonth(&time),
            TTime_DayNoInWeek(&time),
            TTime_DaysInMonth(&time),
        )
    };
    let _ = writeln!(
        out,
        "{name}: micros={micros} day_no_in_year={year_day} day_no_in_month={month_day} \
         day_no_in_week={week_day} days_in_month={month_len}"
    );
}

fn probe() -> Result<()> {
    let mut out = String::new();

    for year in [0, 1, 4, 100, 400, 1582, 1600, 1900, 1970, 2000, 2026] {
        // SAFETY (probe): euser static member functions, scalar in and scalar out,
        // neither of them an `L` function.
        let (leaps, is_leap) = unsafe { (Time_LeapYearsUpTo(year), Time_IsLeapYear(year)) };
        let _ = writeln!(out, "year={year} leap_years_up_to={leaps} is_leap={is_leap}");
    }
    // Write what we have before touching the TTime accessors, so a silent death in one
    // of them still leaves the first half of the evidence on the drive.
    fs::create_dir_all(DIR)?;
    fs::write(PROBE, out.as_bytes())?;

    for (name, offset) in CANDIDATES {
        decode(&mut out, name, offset);
    }
    let now = Ttime::universal().micros_since_year_zero();
    decode(&mut out, "universal_now", now);
    let _ = writeln!(out, "universal_now_raw={now}");

    fs::write(PROBE, out.as_bytes())
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("time");
    report.checked("calendar probe wrote its measurements", probe());
    Ok(if report.finish()? { 0 } else { 1 })
}
