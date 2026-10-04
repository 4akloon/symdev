//! The harness of a `tests/*.rs` with `harness = false` (design spec §7): `libtest` needs
//! `std` and a host; this runs the cases on the phone and writes the report `symdev`
//! reads, marking each case before it runs so a panic is attributed.
//!
//! ```ignore
//! #![no_std]
//! #![no_main]
//! use symbian_test::{Evidence, ensure};
//!
//! #[symbian_test::tests]
//! mod smoke {
//!     use super::*;
//!
//!     #[test]
//!     fn adds() -> Result<(), Evidence> {
//!         ensure(1 + 1 == 2, "1 + 1 is 2")
//!     }
//! }
//! ```
#![no_std]
extern crate alloc;

mod evidence;

pub use evidence::{Evidence, ensure};
pub use symbian_macros::tests;

use symbian_std::test_report::Report;

/// One `#[test] fn` of the module, as `#[symbian_test::tests]` lists it.
pub struct Case {
    pub name: &'static str,
    pub run: fn() -> Result<(), Evidence>,
}

/// Runs `cases` in order. Before the first, every case is written as `pending`; before
/// each, that case as `running`; after it, its verdict. A panic ends the process, so the
/// file then names the case it hit, and the runner marks the rest as not run.
#[doc(hidden)]
pub fn __run(app: &str, uid3: u32, cases: &[Case]) -> i32 {
    let mut report = Report::with_uid3(app, uid3);
    for case in cases {
        report.pending(case.name);
    }
    let _ = report.save();
    for case in cases {
        report.running(case.name);
        let _ = report.save();
        match (case.run)() {
            Ok(()) => report.settle(case.name, true, ""),
            Err(e) => report.settle(case.name, false, e.text()),
        }
        let _ = report.save();
    }
    // The verdict is the report's; the runner reads it after the process ends.
    0
}
