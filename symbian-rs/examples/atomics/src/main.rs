//! Atomics, `Arc`, `Mutex`, `Once` and threads (design spec §6a, §11 step 72).
//!
//! Two threads hammer one `AtomicU32` and one `Arc<Mutex<u32>>`, and beside them a
//! deliberately racy load-then-store on a third counter. The first two must come out
//! **exactly** right; the third must not, and that is the point — it is the same
//! program, the same threads and the same yields, differing only in whether the
//! read-modify-write happens as one operation. If the racy one also came out exact the
//! test would be proving nothing about the machine.
//!
//! Everything below is `std`'s vocabulary: `AtomicU32`, `Arc`, `Mutex`, `Once`,
//! `thread::spawn`, `JoinHandle::join`. The only Symbian name in the file is the one
//! that asks the SDK's compiler-runtime archive whether its lock really exists — and
//! the point of asking is that on this CPU there is no atomic instruction at all, so
//! every line here is a kernel call underneath.
//!
//! It reports through [`symbian_std::test_report`], which writes
//! `E:\symdev\results\<uid3>.json`; `symdev test --emulator` reads that back off the
//! emulated drive and fails the build if any case failed.
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

extern crate alloc;

use core::sync::atomic::{AtomicU32, Ordering};

use symbian_std::io::ErrorKind;
use symbian_std::sync::{self, Arc, Mutex, Once};
use symbian_std::test_report::{Report, detail};
use symbian_std::thread;

/// Increments per thread. Every one is a kernel `Wait`/`Signal` pair on this CPU, so
/// this is chosen to finish inside the harness's timeout, not as a benchmark.
const ITERATIONS: u32 = 2_000;

/// How often a thread yields, so EKA2L1 actually interleaves the two. Without a yield
/// it never preempts a tight loop and even the racy counter comes out exact, which
/// would make the comparison meaningless (experiment 72).
const YIELD_EVERY: u32 = 8;

/// Incremented with `fetch_add`: one atomic read-modify-write.
static ATOMIC: AtomicU32 = AtomicU32::new(0);

/// Incremented with a load, a yield and a store: three operations that are each atomic
/// and together are not. This is the control.
static RACY: AtomicU32 = AtomicU32::new(0);

/// Proves `Once` runs its closure exactly once across two threads.
static ONCE: Once = Once::new();
static ONCE_RUNS: AtomicU32 = AtomicU32::new(0);

/// A `Mutex` in a `static`, which is possible only because `Mutex::new` is `const`.
static SHARED: Mutex<u32> = Mutex::new(0);

/// The work both threads do.
fn hammer(shared: &Arc<Mutex<u32>>) {
    for i in 0..ITERATIONS {
        ATOMIC.fetch_add(1, Ordering::SeqCst);

        // Deliberately not atomic as a whole: read, let the other thread in, write.
        let seen = RACY.load(Ordering::SeqCst);
        if i % YIELD_EVERY == 0 {
            thread::yield_now();
        }
        RACY.store(seen + 1, Ordering::SeqCst);

        if let Ok(mut value) = shared.lock() {
            *value += 1;
        }
        if let Ok(mut value) = SHARED.lock() {
            *value += 1;
        }
        ONCE.call_once(|| {
            ONCE_RUNS.fetch_add(1, Ordering::SeqCst);
        });
    }
}

/// The single-threaded half: the lock's own bootstrap, then every atomic operation and
/// `Arc`, before a second thread exists to complicate it.
fn check_single_threaded(report: &mut Report) {
    // The lock behind every atomic is created on first use by `User::LockedInc`, the
    // one atomic euser exports. Before any atomic operation there is no lock at all.
    report.check(
        "no atomic lock before the first atomic",
        sync::atomic_lock_status() == 0,
    );

    ATOMIC.store(5, Ordering::SeqCst);
    report.check(
        "one atomic operation creates the lock",
        sync::atomic_lock_status() == 1,
    );
    report.check(
        "and it is a real kernel handle",
        sync::atomic_lock_handle() != 0,
    );

    report.check("store then load", ATOMIC.load(Ordering::SeqCst) == 5);
    report.check(
        "fetch_add returns the old value",
        ATOMIC.fetch_add(2, Ordering::SeqCst) == 5,
    );
    report.check(
        "compare_exchange takes on a match",
        ATOMIC.compare_exchange(7, 9, Ordering::SeqCst, Ordering::SeqCst) == Ok(7),
    );
    report.check(
        "and reports what was there on a miss",
        ATOMIC.compare_exchange(7, 11, Ordering::SeqCst, Ordering::SeqCst) == Err(9),
    );
    ATOMIC.store(0, Ordering::SeqCst);

    // `Arc` exists only because the target now declares 32-bit atomics: at
    // `max-atomic-width: 0` there is no `alloc::sync` at all.
    let counted = Arc::new(7u32);
    let second = Arc::clone(&counted);
    report.check("Arc clones", *counted + *second == 14);
    report.check(
        "and counts strong references",
        Arc::strong_count(&counted) == 2,
    );
    drop(second);
    report.check("and drops one", Arc::strong_count(&counted) == 1);
}

/// `Mutex`'s own surface, before any thread exists.
fn check_mutex(report: &mut Report) {
    match SHARED.lock() {
        Err(_) => report.fail("the static Mutex locks", "it did not"),
        Ok(mut held) => {
            report.check("the static Mutex locks", true);
            *held = 1;
            report.check(
                "try_lock on a held Mutex times out",
                SHARED.try_lock().err().map(|e| e.kind()) == Some(ErrorKind::TimedOut),
            );
        }
    }
    report.check(
        "try_lock on a free Mutex succeeds",
        SHARED.try_lock().is_ok(),
    );
    if let Ok(mut value) = SHARED.lock() {
        *value = 0;
    }
}

/// One thread returning a value, before two threads race over one counter.
fn check_one_thread(report: &mut Report) {
    match thread::spawn(|| 6u32 * 7) {
        Err(_) => report.fail("a thread returns its value", "it was not created"),
        Ok(handle) => match handle.join() {
            Err(_) => report.fail("a thread returns its value", "join failed"),
            Ok(value) => report.check("a thread returns its value", value == 42),
        },
    }
}

/// The two-thread run, and the counts it has to produce.
fn check_two_threads(report: &mut Report) {
    let shared = Arc::new(Mutex::new(0u32));
    let worker_copy = Arc::clone(&shared);
    let Some(worker) = report.checked(
        "a second thread starts",
        thread::spawn(move || hammer(&worker_copy)),
    ) else {
        return;
    };
    hammer(&shared);
    report.checked("and joins", worker.join());

    let expected = ITERATIONS * 2;
    let atomic = ATOMIC.load(Ordering::SeqCst);
    let racy = RACY.load(Ordering::SeqCst);
    report.check_detail(
        "fetch_add from two threads loses nothing",
        atomic == expected,
        detail!("{atomic} of {expected}"),
    );
    report.check_detail(
        "a load-then-store beside it does lose updates",
        racy < expected,
        detail!("{racy} of {expected}; exact here would mean the threads never raced"),
    );

    let through_static = SHARED.lock().map(|v| *v).unwrap_or(0);
    report.check_detail(
        "a static Mutex counts every increment",
        through_static == expected,
        detail!("{through_static} of {expected}"),
    );
    let through_arc = shared.lock().map(|v| *v).unwrap_or(0);
    report.check_detail(
        "an Arc<Mutex<_>> counts every increment",
        through_arc == expected,
        detail!("{through_arc} of {expected}"),
    );
    let runs = ONCE_RUNS.load(Ordering::SeqCst);
    report.check_detail(
        "Once ran exactly once for both threads",
        runs == 1,
        detail!("{runs} runs"),
    );
    report.check(
        "the Arc is back to one reference",
        Arc::strong_count(&shared) == 1,
    );
}

/// The heap still works after a thread has exited, which is the whole reason a worker
/// is created with its own heap and switched onto this one (experiment 80).
fn check_heap_survived(report: &mut Report) {
    let mut grown = alloc::vec::Vec::new();
    for i in 0..64u32 {
        grown.push(i);
    }
    report.check(
        "the heap still allocates after a thread has exited",
        grown.iter().sum::<u32>() == 2016,
    );
}

fn main() -> i32 {
    let mut report = Report::new("atomics");
    check_single_threaded(&mut report);
    check_mutex(&mut report);
    check_one_thread(&mut report);
    check_two_threads(&mut report);
    check_heap_survived(&mut report);
    match report.finish() {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => e.raw_os_error().unwrap_or(-1),
    }
}

symbian_runtime::entry!(main);
