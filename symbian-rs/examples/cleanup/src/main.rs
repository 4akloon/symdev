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
//! It reaches for `symbian-sys` directly rather than a friendly wrapper on purpose:
//! there is no `read_dir` in the `no_std` shape yet, and what is under test is the
//! runtime, not the file API. When one arrives this should use it.
#![no_std]

extern crate alloc;

use symbian_core::Result;
use symbian_core::des::{Buf16, DesC16};
use symbian_std::test_report::Report;
use symbian_sys::efsrv::{CDir, CDir_Count, ESORT_NONE, KENTRY_ATT_MATCH_MASK, RFs_GetDir};
use symbian_sys::efsrv::{KFILE_SERVER_DEFAULT_MESSAGE_SLOTS, RFs, RFs_Connect};

/// `RFs::GetDir` is the call `std`'s own cleanup note names as the one that dies
/// without a trap handler. Returns the entry count, or the error code.
fn count_entries() -> i32 {
    let mut fs = RFs { handle: 0 };
    let code = unsafe { RFs_Connect(&mut fs, KFILE_SERVER_DEFAULT_MESSAGE_SLOTS) };
    if code != 0 {
        return code;
    }
    let mut pattern: Buf16<256> = Buf16::new();
    if pattern.push_str("E:\\symdev\\*").is_err() {
        return -6;
    }
    let mut dir: *mut CDir = core::ptr::null_mut();
    let code = unsafe {
        RFs_GetDir(
            &mut fs,
            pattern.as_tdesc16(),
            KENTRY_ATT_MATCH_MASK,
            ESORT_NONE,
            &mut dir,
        )
    };
    if code != 0 {
        return code;
    }
    unsafe { CDir_Count(dir) }
}

#[symbian_std::main]
fn main() -> Result<i32> {
    let mut report = Report::new("cleanup");
    report.check("reached main", true);
    // If there is no cleanup stack, the thread dies inside this call and the report
    // below is never written — which is the whole measurement.
    let count = count_entries();
    report.check_detail(
        "GetDir returned",
        count >= 0,
        format_args!("count/err = {count}"),
    );
    Ok(if report.finish()? { 0 } else { 1 })
}
