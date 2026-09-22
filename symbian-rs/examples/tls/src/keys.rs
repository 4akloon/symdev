//! What `thread_local!` has to do to be worth the name.
//!
//! Every case here is written the way it would be against `std`: a `thread_local!`
//! block, `with`, `try_with`, a `Cell` and a `RefCell`. Nothing in this file names a
//! Symbian API, which is the point — the platform's one kernel call is
//! `symbian_std::thread`'s problem, not an application's.

use alloc::string::String;
use core::cell::{Cell, RefCell};
use core::sync::atomic::{AtomicU32, Ordering};

use symbian_std::test_report::{Report, detail};
use symbian_std::thread;
use symbian_std::thread_local;

/// How many times `NAME`'s initialiser has run, across every thread. Lazy
/// initialisation is only observable from inside the initialiser, so it is counted
/// there.
static NAME_INITS: AtomicU32 = AtomicU32::new(0);

/// How many `Witness` values have been dropped. A thread-local whose destructor never
/// runs is a leak, and this is the only thing that can see it happen.
static DROPS: AtomicU32 = AtomicU32::new(0);

/// A value that says so when it is dropped.
struct Witness(u32);

impl Drop for Witness {
    fn drop(&mut self) {
        DROPS.fetch_add(self.0, Ordering::SeqCst);
    }
}

thread_local! {
    /// The `const { … }` initialiser form, as `std` spells it.
    static COUNTER: Cell<u32> = const { Cell::new(0) };
    /// A second key in the same block, with a heap value and a lazy initialiser.
    static NAME: RefCell<String> = {
        NAME_INITS.fetch_add(1, Ordering::SeqCst);
        RefCell::new(String::from("unnamed"))
    };
}

thread_local! {
    /// A third key, in a block of its own, so that two `thread_local!` invocations are
    /// exercised and not only two keys in one.
    static WITNESS: Witness = Witness(1);
}

// A key whose initialiser asks for the key, which `std` reports as a panic and this
// crate as a `symbian_std::thread::AccessError`.
thread_local! {
    static SELF_REFERENTIAL: Cell<u32> = {
        let seen = SELF_REFERENTIAL.try_with(|c| c.get()).is_ok();
        Cell::new(u32::from(seen))
    };
}

/// One thread, three keys: lazy once, independent, and readable again.
pub fn one_thread(report: &mut Report) {
    report.check_detail(
        "an untouched thread holds no thread-local",
        thread::live_thread_locals() == 0,
        detail!("{} live", thread::live_thread_locals()),
    );

    report.check(
        "a const initialiser gives the initial value",
        COUNTER.with(Cell::get) == 0,
    );
    COUNTER.set_to(42);
    report.check("and the value stays set", COUNTER.with(Cell::get) == 42);

    report.check_detail(
        "a lazy initialiser runs on first use",
        NAME_INITS.load(Ordering::SeqCst) == 0,
        detail!("{} before the first use", NAME_INITS.load(Ordering::SeqCst)),
    );
    NAME.with(|name| name.borrow_mut().push_str("-main"));
    let runs = NAME_INITS.load(Ordering::SeqCst);
    report.check_detail("and exactly once", runs == 1, detail!("{runs} runs"));
    NAME.with(|name| {
        let _ = name.borrow_mut();
    });
    let runs = NAME_INITS.load(Ordering::SeqCst);
    report.check_detail(
        "however often it is read",
        runs == 1,
        detail!("{runs} runs"),
    );
    report.check(
        "the lazy value is what the initialiser built",
        NAME.with(|name| name.borrow().as_str() == "unnamed-main"),
    );

    report.check(
        "a third key in its own block initialises",
        WITNESS.with(|w| w.0) == 1,
    );
    report.check_detail(
        "and the three keys are three values",
        thread::live_thread_locals() == 3,
        detail!("{} live", thread::live_thread_locals()),
    );
    report.check(
        "none of them disturbed the others",
        COUNTER.with(Cell::get) == 42 && NAME.with(|n| n.borrow().len()) == 12,
    );
}

/// The re-entrancy guard, which is the only way an initialiser can fail here.
pub fn reentrancy(report: &mut Report) {
    let value = SELF_REFERENTIAL.with(Cell::get);
    report.check_detail(
        "an initialiser that asks for its own key is refused, not looped",
        value == 0,
        detail!("the initialiser saw try_with succeed: {}", value == 1),
    );
    report.check(
        "and the key works normally afterwards",
        SELF_REFERENTIAL.try_with(Cell::get) == Ok(0),
    );
}

/// Two threads, one key each: the case the whole slice exists for.
pub fn two_threads(report: &mut Report) {
    COUNTER.set_to(42);
    let before_drops = DROPS.load(Ordering::SeqCst);

    let worker = thread::spawn(|| {
        let first_seen = COUNTER.with(Cell::get);
        COUNTER.set_to(7);
        NAME.with(|name| name.borrow_mut().push_str("-worker"));
        let witness = WITNESS.with(|w| w.0);
        (
            first_seen,
            COUNTER.with(Cell::get),
            NAME.with(|n| n.borrow().len()),
            thread::live_thread_locals(),
            witness,
        )
    });
    let Some(handle) = report.checked("a worker thread starts", worker) else {
        return;
    };
    let Some((first_seen, worker_counter, worker_name_len, live, witness)) =
        report.checked("and joins", handle.join())
    else {
        return;
    };

    report.check_detail(
        "a worker starts from the initialiser, not from the creator's value",
        first_seen == 0,
        detail!("it saw {first_seen}, the creator had 42"),
    );
    report.check_detail(
        "a worker's own value is its own",
        worker_counter == 7,
        detail!("{worker_counter}"),
    );
    report.check_detail(
        "and the creator's is untouched by it",
        COUNTER.with(Cell::get) == 42,
        detail!("{}", COUNTER.with(Cell::get)),
    );
    report.check_detail(
        "the lazy initialiser ran again for the worker",
        NAME_INITS.load(Ordering::SeqCst) == 2,
        detail!(
            "{} runs across both threads",
            NAME_INITS.load(Ordering::SeqCst)
        ),
    );
    report.check_detail(
        "the worker's heap value is its own too",
        worker_name_len == 14 && NAME.with(|n| n.borrow().len()) == 12,
        detail!(
            "worker {worker_name_len}, main {}",
            NAME.with(|n| n.borrow().len())
        ),
    );
    report.check_detail(
        "and it held its own three keys",
        live == 3 && witness == 1,
        detail!("{live} live on the worker, witness {witness}"),
    );

    let dropped = DROPS.load(Ordering::SeqCst) - before_drops;
    report.check_detail(
        "the worker's thread-locals were dropped when it ended",
        dropped == 1,
        detail!("{dropped} dropped"),
    );
    report.check_detail(
        "and the creator's survived the sweep",
        thread::live_thread_locals() == 3 && WITNESS.with(|w| w.0) == 1,
        detail!("{} live", thread::live_thread_locals()),
    );
}

/// The main thread's own teardown, which nothing calls for it.
pub fn main_thread_teardown(report: &mut Report) {
    let before = DROPS.load(Ordering::SeqCst);
    thread::drop_thread_locals();
    report.check_detail(
        "drop_thread_locals drops this thread's values too",
        DROPS.load(Ordering::SeqCst) - before == 1,
        detail!("{} dropped", DROPS.load(Ordering::SeqCst) - before),
    );
    report.check_detail(
        "and the thread holds nothing afterwards",
        thread::live_thread_locals() == 0,
        detail!("{} live", thread::live_thread_locals()),
    );
    report.check(
        "an access after the sweep is an error, not a new value",
        COUNTER.try_with(Cell::get).is_err(),
    );
    report.check_detail(
        "and it says which error",
        COUNTER.try_with(Cell::get).err().map(|e| e.reason()) == Some(-13),
        detail!("{:?}", COUNTER.try_with(Cell::get).err()),
    );
    thread::drop_thread_locals();
    report.check(
        "a second sweep is harmless",
        thread::live_thread_locals() == 0,
    );
}

/// `LocalKey<Cell<T>>::set` is a `std` convenience this crate does not have; the
/// example writes it once rather than repeating the closure.
trait SetTo {
    fn set_to(&'static self, value: u32);
}

impl SetTo for symbian_std::thread::LocalKey<Cell<u32>> {
    fn set_to(&'static self, value: u32) {
        self.with(|cell| cell.set(value));
    }
}
