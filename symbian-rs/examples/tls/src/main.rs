//! Thread-local storage (design spec §11 step 76, the half `time` left open).
//!
//! It reports through [`symbian_std::test_report`], which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.
#![no_std]
#![no_main]

extern crate alloc;

mod probe;

use symbian_std::test_report::Report;

fn main() -> i32 {
    let mut report = Report::new("tls");
    probe::one_thread(&mut report);
    probe::how_many_slots(&mut report);
    probe::the_uid_overloads(&mut report);
    probe::per_thread(&mut report);
    match report.finish() {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

symbian_runtime::entry!(main);
