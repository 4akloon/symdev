//! Throwaway HAL probe for experiment 85 — NOT the finished example.
//!
//! `HAL::Get` lives in `hal.dll`, which is not on this SDK's link line, but euser
//! exports `_ZN7UserSvr6HalGetEiPv` — `UserSvr::HalGet(TInt, TAny*)` — which is what
//! `HAL::Get` calls. If that works, the nanokernel tick period and the fast counter's
//! frequency and direction are readable from any board instead of hard-coded.
//!
//! The self-check that says the attribute numbering is right and not a guess:
//! attribute 14 (`ESystemTickPeriod`) must come back as 15 625, the number
//! `UserHal::TickPeriod` already gave.
#![no_std]

extern crate alloc;

use alloc::string::String;
use core::fmt::Write as _;

use symbian_core::time::SystemTicks;
use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::Report;

const DIR: &str = "E:\\symdev\\time76";
const PROBE: &str = "E:\\symdev\\time76\\hal.txt";

/// `HALData::TAttribute` ordinals, counted out of `hal_data.h`'s enum.
const ATTRIBUTES: [(&str, i32); 5] = [
    ("ESystemTickPeriod", 14),
    ("EMemoryRAM", 15),
    ("ENanoTickPeriod", 92),
    ("EFastCounterFrequency", 93),
    ("EFastCounterCountsUp", 94),
];

unsafe extern "C" {
    #[link_name = "_ZN7UserSvr6HalGetEiPv"]
    fn UserSvr_HalGet(attribute: i32, value: *mut i32) -> i32;
}

fn probe() -> Result<()> {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "userhal_tick_period={:?}",
        SystemTicks::period_micros().map_err(|e| e.code())
    );
    fs::create_dir_all(DIR)?;
    fs::write(PROBE, out.as_bytes())?;

    for (name, attribute) in ATTRIBUTES {
        let mut value: i32 = i32::MIN;
        // SAFETY (probe): `UserSvr::HalGet(TInt, TAny*)` per the mangled name; the
        // out-pointer is a valid, aligned, exclusively borrowed `i32` for the call.
        let code = unsafe { UserSvr_HalGet(attribute, &mut value) };
        let _ = writeln!(out, "{name}({attribute}) code={code} value={value}");
        // One attribute at a time, so a call that takes the process down still leaves
        // everything before it on the drive.
        fs::write(PROBE, out.as_bytes())?;
    }
    Ok(())
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("time");
    report.checked("HAL probe wrote its measurements", probe());
    Ok(if report.finish()? { 0 } else { 1 })
}
