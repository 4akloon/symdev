//! What Symbian's own thread-local storage really does on this platform.
//!
//! These cases talk to `UserSvr::Dll*Tls` directly, below everything
//! `symbian_std::thread::LocalKey` builds on it, because the design of `LocalKey`
//! turns entirely on the answers: how many slots there are, what they are keyed by,
//! and whether two threads see the same one. Experiment 88 is this file's output.

use core::ffi::c_void;

use symbian_std::test_report::Report;
use symbian_std::thread;
use symbian_sys::tls::{
    UserSvr_DllFreeTls, UserSvr_DllSetTls, UserSvr_DllSetTlsWithUid, UserSvr_DllTls,
    UserSvr_DllTlsWithUid,
};

/// Handles the probe stores under. Arbitrary — that is the point of the probe — but
/// deliberately not [`symbian_sys::tls::SYMBIAN_STD_TLS_HANDLE`], which
/// `symbian_std::thread_local!` owns and this file must not disturb.
const H1: i32 = 0x7072_6F31;
const H2: i32 = 0x7072_6F32;
const H3: i32 = 0x7072_6F33;
const H4: i32 = 0x7072_6F40;

/// How many distinct handles the probe tries to hold at once.
const MANY: i32 = 64;

/// A value that is a plausible pointer and never dereferenced.
fn marker(n: u32) -> *mut c_void {
    (0x1000_0000u32 + n * 4) as *mut c_void
}

fn set(handle: i32, value: *mut c_void) -> i32 {
    // SAFETY: a euser static taking two scalars. The pointer is stored and never
    // followed by the kernel, so a synthetic address is a legitimate argument.
    unsafe { UserSvr_DllSetTls(handle, value) }
}

fn get(handle: i32) -> *mut c_void {
    // SAFETY: as `set`; reads back this thread's slot or null.
    unsafe { UserSvr_DllTls(handle) }
}

/// The single-threaded facts: does a set come back, and are two handles two slots?
pub fn one_thread(report: &mut Report) {
    report.check_detail(
        "nothing is stored under an untouched handle",
        get(H3).is_null(),
        format_args!("{:#x}", get(H3) as usize),
    );

    let rc = set(H1, marker(1));
    report.check_detail("UserSvr::DllSetTls succeeds", rc == 0, format_args!("{rc}"));
    report.check_detail(
        "and DllTls reads the same pointer back",
        get(H1) == marker(1),
        format_args!("{:#x} wanted {:#x}", get(H1) as usize, marker(1) as usize),
    );

    set(H2, marker(2));
    report.check_detail(
        "a second handle does not overwrite the first",
        get(H1) == marker(1) && get(H2) == marker(2),
        format_args!("h1={:#x} h2={:#x}", get(H1) as usize, get(H2) as usize),
    );

    set(H1, marker(3));
    report.check("a repeated set replaces the value", get(H1) == marker(3));

    // SAFETY: a euser static taking one scalar; it forgets the slot and never touches
    // what the pointer referred to.
    unsafe { UserSvr_DllFreeTls(H1) };
    report.check_detail(
        "DllFreeTls empties that handle and leaves the other",
        get(H1).is_null() && get(H2) == marker(2),
        format_args!("h1={:#x} h2={:#x}", get(H1) as usize, get(H2) as usize),
    );
    // SAFETY: as above.
    unsafe { UserSvr_DllFreeTls(H2) };
}

/// How many slots one thread can hold at once — the question the design turns on.
pub fn how_many_slots(report: &mut Report) {
    let mut set_failed_at = -1;
    for n in 0..MANY {
        if set(H4 + n, marker(100 + n as u32)) != 0 {
            set_failed_at = n;
            break;
        }
    }
    let mut readable = 0;
    for n in 0..MANY {
        if get(H4 + n) == marker(100 + n as u32) {
            readable += 1;
        }
    }
    report.check_detail(
        "one thread holds many slots at once",
        set_failed_at < 0 && readable == MANY,
        format_args!("{readable} of {MANY} readable, first set failure at {set_failed_at}"),
    );
    for n in 0..MANY {
        // SAFETY: a euser static taking one scalar.
        unsafe { UserSvr_DllFreeTls(H4 + n) };
    }
}

/// Whether the two-argument overloads pair with the one-argument ones.
pub fn the_uid_overloads(report: &mut Report) {
    const UID: i32 = 0x0e00_0690;
    // SAFETY: euser statics taking scalars, as above.
    let rc = unsafe { UserSvr_DllSetTlsWithUid(H1, UID, marker(7)) };
    // SAFETY: as above.
    let with_uid = unsafe { UserSvr_DllTlsWithUid(H1, UID) };
    report.check_detail(
        "the uid overloads round-trip",
        rc == 0 && with_uid == marker(7),
        format_args!("rc={rc} value={:#x}", with_uid as usize),
    );
    report.check_detail(
        "and the one-argument read of the same handle",
        true,
        format_args!(
            "{:#x} (marker is {:#x}; equal means the uid is ignored on read)",
            get(H1) as usize,
            marker(7) as usize
        ),
    );

    set(H2, marker(8));
    // SAFETY: as above.
    let cross = unsafe { UserSvr_DllTlsWithUid(H2, H2) };
    report.check_detail(
        "a one-argument set read with uid = handle",
        true,
        format_args!("{:#x} (marker is {:#x})", cross as usize, marker(8) as usize),
    );
    // SAFETY: as above.
    unsafe {
        UserSvr_DllFreeTls(H1);
        UserSvr_DllFreeTls(H2);
    }
}

/// The decisive one: is a slot per thread, or shared by the whole process?
pub fn per_thread(report: &mut Report) {
    set(H1, marker(11));

    let worker = thread::spawn(|| {
        let seen_before = get(H1);
        set(H1, marker(12));
        let seen_after = get(H1);
        (seen_before as usize, seen_after as usize)
    });
    let Some(handle) = report.checked("a worker thread starts", worker) else {
        return;
    };
    let Some((before, after)) = report.checked("and joins", handle.join()) else {
        return;
    };

    report.check_detail(
        "a new thread does not inherit the creator's slot",
        before == 0,
        format_args!("the worker read {before:#x} before storing anything"),
    );
    report.check_detail(
        "the worker's own store is visible to itself",
        after == marker(12) as usize,
        format_args!("{after:#x}"),
    );
    report.check_detail(
        "and it did not overwrite the creator's slot",
        get(H1) == marker(11),
        format_args!(
            "{:#x} wanted {:#x}",
            get(H1) as usize,
            marker(11) as usize
        ),
    );
    // SAFETY: a euser static taking one scalar.
    unsafe { UserSvr_DllFreeTls(H1) };
}
