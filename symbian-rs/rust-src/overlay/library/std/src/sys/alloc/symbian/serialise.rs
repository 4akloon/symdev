//! Serialising the heap, from the moment a second thread exists and not before
//! (experiment 80, re-hosted).
//!
//! Symbian's own heap may or may not lock internally: `RAllocator::TFlags` has an
//! `ESingleThreaded` bit (`e32cmn.h` line 2613), the flags field is protected, and
//! `RHeap` is not even declared in this SDK's headers, so nothing running on this host
//! can read what the process heap was created with. Two threads allocating at once did
//! not corrupt it in the emulator — 400 interleaved alloc/free pairs each, with forced
//! yields, then a 16 kB allocation that walks the whole free list — but "it did not
//! break in one test" is not "it is locked", so the allocator takes a lock of its own.
//!
//! It costs nothing until [`serialise_across_threads`] is called, which
//! `sys::thread::Thread::new` does before it creates the first thread. Until then every
//! allocation reads one `TInt` and takes no lock.
//!
//! The flag is a plain `TInt` and **not** an `AtomicBool` on purpose: an atomic on this
//! target is a libcall over a process-wide `RFastLock`, so every allocation would take
//! *two* locks instead of one.

use crate::cell::UnsafeCell;
use symbian_sys::thread::{
    EOWNER_PROCESS, RFastLock, RFastLock_CreateLocal, RFastLock_Signal, RFastLock_Wait,
};

/// A `static` this module mutates through a raw pointer. Every use states its own
/// ordering argument; there is no `Mutex` to reach for below the allocator — `Mutex`
/// allocates its kernel handle lazily and would come straight back here.
struct Shared<T>(UnsafeCell<T>);

// SAFETY: `ENABLED` is written once, on the process's only thread, before the thread
// that could observe it exists — so the write happens-before every read, through the
// kernel's own thread creation. `LOCK` is written in the same place and read only after
// `ENABLED` says it is there.
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
static LOCK: Shared<RFastLock> = Shared::new(RFastLock::NULL);

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
/// the plain write to `ENABLED` sound, and it is why `sys::thread` calls it before
/// `RThread::Create` rather than from inside the new thread.
pub unsafe fn serialise_across_threads() -> i32 {
    // SAFETY: single-threaded by the caller's contract.
    unsafe {
        if crate::ptr::read_volatile(ENABLED.get()) == 1 {
            return 0;
        }
        let code = RFastLock_CreateLocal(LOCK.get(), EOWNER_PROCESS);
        if code == 0 {
            crate::ptr::write_volatile(ENABLED.get(), 1);
        }
        code
    }
}

fn is_serialised() -> bool {
    // SAFETY: a plain aligned `TInt` load; volatile so the allocator cannot cache it
    // across the call that turns it on.
    unsafe { crate::ptr::read_volatile(ENABLED.get()) == 1 }
}

/// Held across one heap operation, and only once serialisation is on.
///
/// `RFastLock` is **not** recursive — a second `Wait` from the owning thread blocks for
/// ever (observed, experiment 72) — so no code under this guard may allocate. That is
/// why the four entry points take the guard and then call the unlocked helpers instead
/// of calling one another.
pub(super) struct HeapGuard(bool);

impl HeapGuard {
    /// `#[inline(never)]` on both halves, for size: inlined into all four entry points
    /// the guard cost 183 bytes, as one out-of-line copy it costs 154 (experiment 80).
    #[inline(never)]
    pub(super) fn enter() -> Self {
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
