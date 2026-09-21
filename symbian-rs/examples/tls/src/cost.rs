//! What one `thread_local!` access costs on this machine, beside the two things it
//! could be compared with: the bare kernel call underneath it, and one atomic
//! operation, which is what any other way of keeping per-thread state would pay.
//!
//! The clock is `SystemTime`, not `Instant`: `Instant` is `User::TickCount` at
//! 15.625 ms (experiment 85) and `TTime::UniversalTime` steps by 1 µs, which is the
//! resolution a per-operation figure needs. It is a wall clock, so a device clock
//! change during the loop would spoil a reading; nothing sets the clock here.
//!
//! These are **emulator** numbers and say nothing about an E52.

use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

use symbian_std::test_report::Report;
use symbian_std::thread_local;
use symbian_std::time::SystemTime;
use symbian_sys::tls::{SYMBIAN_STD_TLS_HANDLE, UserSvr_DllTls};

/// Enough iterations that the loop takes tens of milliseconds, so that the 1 µs clock
/// and the loop's own overhead are both small against the total.
const ROUNDS: u32 = 200_000;

thread_local! {
    /// The key the measurement reads. It is the first key this thread stores, so the
    /// table walk finds it at the front — the best case, and the honest one to quote
    /// beside "a linear scan" as the cost model.
    static MEASURED: Cell<u32> = const { Cell::new(0) };
}

/// The control: an atomic, which on this CPU is a `Wait`/`Signal` pair on the
/// process-wide `RFastLock` the compiler-runtime archive owns (experiment 80).
static ATOMIC: AtomicU32 = AtomicU32::new(0);

/// Nanoseconds per iteration of `body`, or `None` if the clock went backwards.
fn per_iteration(body: impl Fn()) -> Option<u64> {
    let start = SystemTime::now();
    for _ in 0..ROUNDS {
        body();
    }
    let elapsed = SystemTime::now().duration_since(start).ok()?;
    // Not `as_nanos`: it is `u128`, and 128-bit division and formatting cost this
    // program a kilobyte of `compiler_builtins` for nothing (experiment 85).
    let nanos = elapsed.as_secs() * 1_000_000_000 + u64::from(elapsed.subsec_nanos());
    Some(nanos / u64::from(ROUNDS))
}

/// Measures the access, the kernel call under it and one atomic, and reports all three.
pub fn measure(report: &mut Report) {
    // Initialise before the loop, so that what is measured is an access and not a
    // first use.
    MEASURED.with(|c| c.set(0));

    let Some(access) = per_iteration(|| MEASURED.with(|c| c.set(c.get() + 1))) else {
        report.fail(
            "a thread_local access is measurable",
            "the clock went backwards",
        );
        return;
    };
    let Some(kernel) = per_iteration(|| {
        // SAFETY: a euser static taking one scalar; it reads this thread's own slot
        // and the value is discarded.
        let raw = unsafe { UserSvr_DllTls(SYMBIAN_STD_TLS_HANDLE) };
        core::hint::black_box(raw);
    }) else {
        report.fail("the kernel call is measurable", "the clock went backwards");
        return;
    };
    let Some(atomic) = per_iteration(|| {
        ATOMIC.fetch_add(1, Ordering::SeqCst);
    }) else {
        report.fail("an atomic is measurable", "the clock went backwards");
        return;
    };

    report.check_detail(
        "one thread_local access costs about one kernel call",
        access > 0 && kernel > 0,
        format_args!("{access} ns/access, of which UserSvr::DllTls is {kernel} ns"),
    );
    report.check_detail(
        "and less than one atomic operation",
        access < atomic,
        format_args!("{access} ns against {atomic} ns for AtomicU32::fetch_add"),
    );
    report.check_detail(
        "the counter really was incremented every time",
        MEASURED.with(Cell::get) == ROUNDS,
        format_args!("{} of {ROUNDS}", MEASURED.with(Cell::get)),
    );
}
