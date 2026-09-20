//! Serialising the heap, from the moment a second thread exists and not before.
//!
//! Symbian's own heap may or may not lock internally: `RAllocator::TFlags` has an
//! `ESingleThreaded` bit (`e32cmn.h` line 2613), the flags field is protected, and
//! `RHeap` is not even declared in this SDK's headers, so nothing running on this host
//! can read what the process heap was created with. Two threads allocating at once did
//! not corrupt it in the emulator — 400 interleaved alloc/free pairs each, with forced
//! yields, followed by a 16 kB allocation that walks the whole free list (experiment
//! 80) — but "it did not break in one test" is not the same as "it is locked", so the
//! allocator takes a lock of its own rather than trusting that.
//!
//! It costs nothing until [`serialise_across_threads`] is called, which
//! `symbian_std::thread::spawn` does before it creates the first thread. Until then
//! every allocation reads one `TInt` and takes no lock, so a single-threaded program —
//! which on this phone is most of them — pays a load and nothing else.
//!
//! The flag is a plain `TInt` and **not** an `AtomicBool` on purpose. An atomic would
//! pull the SDK's compiler-runtime archive into every program that allocates, which
//! costs 756 bytes (experiment 80), and it would make every allocation take the
//! atomics' own global lock as well as this one.

use core::cell::UnsafeCell;

use symbian_sys::thread::{
    EOWNER_PROCESS, RFastLock, RFastLock_CreateLocal, RFastLock_Signal, RFastLock_Wait,
};

/// A `static` this module mutates through a raw pointer. Every use states its own
/// ordering argument; there is no `Mutex` to reach for below the allocator.
struct Shared<T>(UnsafeCell<T>);

// SAFETY: `ENABLED` is written once, on the process's only thread, before the thread
// that could observe it exists — so the write happens-before every read through the
// kernel's own thread creation. `LOCK` is written in the same place and read only
// after `ENABLED` says it is there.
unsafe impl<T> Sync for Shared<T> {}

impl<T> Shared<T> {
    const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    const fn get(&self) -> *mut T {
        self.0.get()
    }
}

/// All zero is what `RFastLock`'s own inline constructor builds (`e32cmn.inl` 3158).
static LOCK: Shared<RFastLock> = Shared::new(RFastLock::null());

/// 0 until [`serialise_across_threads`] succeeds, 1 after.
static ENABLED: Shared<i32> = Shared::new(0);

/// Makes every allocation and deallocation take one process-wide lock.
///
/// Idempotent, and once on it never goes off again: a thread may outlive the handle
/// that created it, so there is no safe moment to turn it back off.
///
/// Returns `KErrNone`, or the `RFastLock::CreateLocal` error, in which case nothing
/// changes and the caller must not create the thread.
///
/// # Safety
/// Must be called while the process still has exactly one thread. That is what makes
/// the plain write to `ENABLED` sound, and it is why `symbian_std::thread::spawn` calls
/// it before `RThread::Create` rather than from inside the new thread.
pub unsafe fn serialise_across_threads() -> i32 {
    // SAFETY: single-threaded by the caller's contract.
    unsafe {
        if core::ptr::read_volatile(ENABLED.get()) == 1 {
            return 0;
        }
        let rc = RFastLock_CreateLocal(LOCK.get(), EOWNER_PROCESS);
        if rc == 0 {
            core::ptr::write_volatile(ENABLED.get(), 1);
        }
        rc
    }
}

/// Whether the heap is currently serialised.
pub fn is_serialised() -> bool {
    // SAFETY: a plain aligned `TInt` load; volatile so the allocator cannot cache it
    // across the call that turns it on.
    unsafe { core::ptr::read_volatile(ENABLED.get()) == 1 }
}

/// Held across one heap operation, and only once serialisation is on.
///
/// `RFastLock` is **not** recursive — a second `Wait` from the owning thread blocks for
/// ever (observed, experiment 72) — so no code under this guard may allocate. That is
/// why `SymbianHeap`'s trait methods take the guard and then call unlocked helpers,
/// instead of calling one another.
pub(crate) struct HeapGuard(bool);

impl HeapGuard {
    /// `#[inline(never)]` on both halves, for size. Inlined into all four `GlobalAlloc`
    /// methods the guard cost `examples/alloc` 183 bytes; as one out-of-line copy it
    /// costs 154 (experiment 80). That 154 is the price of not corrupting the heap the
    /// moment a program has two threads, and it is paid only by a program that
    /// allocates — `hello` and `hello-raw` are unchanged.
    #[inline(never)]
    pub(crate) fn enter() -> Self {
        let on = is_serialised();
        if on {
            // SAFETY: `ENABLED` is 1 only after `CreateLocal` succeeded on `LOCK`, and
            // `RFastLock::Wait` is a non-leaving euser member with `this` as argument 0.
            unsafe { RFastLock_Wait(LOCK.get()) };
        }
        Self(on)
    }
}

impl Drop for HeapGuard {
    #[inline(never)]
    fn drop(&mut self) {
        if self.0 {
            // SAFETY: this guard exists only after a successful `Wait` on the same lock.
            unsafe { RFastLock_Signal(LOCK.get()) };
        }
    }
}
