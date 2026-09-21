//! Thread-local storage (design spec §11 step 76, the half `time` left open).
//!
//! Three groups, in the order the report prints them: what the platform's own
//! `UserSvr::Dll*Tls` really does ([`probe`]), what `thread_local!` does with it
//! ([`keys`]), and what one access costs ([`cost`]).
//!
//! It reports through [`symbian_std::test_report`], which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.
#![no_std]
#![no_main]

extern crate alloc;

mod cost;
mod keys;
mod probe;

use symbian_std::test_report::Report;

fn main() -> i32 {
    let mut report = Report::new("tls");
    // The platform first, because what `thread_local!` is allowed to assume is
    // whatever these cases observe.
    probe::one_thread(&mut report);
    probe::how_many_slots(&mut report);
    probe::the_uid_overloads(&mut report);
    probe::per_thread(&mut report);
    // Then the thing itself. The order matters: each group leaves this thread holding
    // one more thread-local than the last, and the counts below say so.
    keys::one_thread(&mut report);
    keys::two_threads(&mut report);
    keys::reentrancy(&mut report);
    cost::measure(&mut report);
    keys::main_thread_teardown(&mut report);
    match report.finish() {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

symbian_runtime::entry!(main);
