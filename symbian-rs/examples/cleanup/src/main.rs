//! The regression test for the thread's cleanup stack in a `no_std` program.
//!
//! Symbian's cleanup stack is per thread and does not exist until somebody calls
//! `CTrapCleanup::New()`. Nothing warns you: the first `CleanupStack::PushL` anywhere
//! below — including inside an SDK call's own `TRAP`, where the application never
//! sees the call — panics `E32USER-CBase 69` and takes the thread with it, and a
//! panic is not a leave, so no `TRAP` catches it.
//!
//! For a while only `std` installed one. This program is the measurement that found
//! it and the test that keeps it found: it calls `RFs::GetDir`, which uses the
//! cleanup stack inside efsrv's own trap harness. Before
//! `symbian_runtime::start` installed the handler it died here, writing no result at
//! all, and the emulator log said
//! `Thread Main panicked with category: E32USER-CBase and exit code: 69` — visible
//! only with `Kernel:trace` in the emulator's `log-filter`.
//!
//! It goes through `fs::read_dir`, which is `RFs::GetDir` underneath. What is under
//! test is the runtime, not the file API: if the entry point stops installing the
//! handler, this dies again with no result file.
#![no_std]

extern crate alloc;

use symbian_std::fs;
use symbian_std::io::Result;
use symbian_std::test_report::{Report, detail};

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("cleanup");
    report.check("reached main", true);
    // Without a cleanup stack the thread dies inside this call and the report below is
    // never written — which is the whole measurement.
    match fs::read_dir("E:\\symdev") {
        Ok(dir) => {
            let count = dir.iter().count();
            report.check_detail("GetDir returned", true, detail!("{count} entries"));
        }
        Err(e) => report.check_detail(
            "GetDir returned",
            false,
            detail!("error {}", e.raw_os_error().unwrap_or(0)),
        ),
    }
    Ok(if report.finish()? { 0 } else { 1 })
}
