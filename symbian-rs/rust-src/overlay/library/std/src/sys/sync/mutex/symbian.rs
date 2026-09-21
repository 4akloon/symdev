//! `Mutex` over a binary `RSemaphore` (experiment 80).
//!
//! # Why a semaphore and not `RMutex` or `RFastLock`
//!
//! `std`'s `Mutex` needs `try_lock`, and **`RSemaphore` is the only primitive on 9.3
//! with a timed wait**: `RFastLock` and `RMutex` have a bare `Wait()` and nothing else,
//! so a mutex built on either could not answer `try_lock` at all. A one-token semaphore
//! is also non-recursive, which is exactly Rust's contract — a second `lock` from the
//! same thread deadlocks, and a deadlock is not undefined behaviour. Whether `RMutex`
//! is recursive is not stated in `e32std.h` and has never been observed, which is a
//! second reason not to build this on it.
//!
//! The timeout `try_lock` passes is **1 microsecond and not 0**, and that is observed
//! rather than chosen: `RSemaphore::Wait(0)` on an empty semaphore does not return — 0
//! means "no timeout", not "do not wait", and a probe that called it never came back.
//! `Wait(1)` on an empty semaphore returns `KErrTimedOut` (-33) straight away, and with
//! a token available it returns `KErrNone` (experiment 80).

use super::super::lazy_handle::LazyHandle;
use symbian_sys::thread::{
    EOWNER_PROCESS, RSemaphore, RSemaphore_CreateLocal, RSemaphore_Signal, RSemaphore_Wait,
    RSemaphore_WaitTimeout,
};

/// What `try_lock` waits for. See the module documentation for why it is not 0.
const TRY_LOCK_MICROSECONDS: i32 = 1;

pub struct Mutex {
    semaphore: LazyHandle<RSemaphore>,
}

impl Mutex {
    #[inline]
    pub const fn new() -> Mutex {
        Mutex { semaphore: LazyHandle::new(RSemaphore::NULL) }
    }

    #[inline]
    fn semaphore(&self) -> *mut RSemaphore {
        self.semaphore.get(|handle| {
            // SAFETY: `LazyHandle` runs this once, on one thread, before any other
            // thread can reach the handle. `CreateLocal` is a non-leaving euser member
            // taking `this` as argument 0 (experiment 78); one token makes it a mutex.
            unsafe { RSemaphore_CreateLocal(handle, 1, EOWNER_PROCESS) }
        })
    }

    #[inline]
    pub fn lock(&self) {
        // SAFETY: a created, process-local semaphore. `Wait` is non-leaving, takes the
        // handle as argument 0, and consumes the one token or blocks until it is back.
        unsafe { RSemaphore_Wait(self.semaphore()) };
    }

    /// # Safety
    /// The calling thread must hold the lock.
    #[inline]
    pub unsafe fn unlock(&self) {
        // SAFETY: as `lock`; the caller holds the token this returns.
        unsafe { RSemaphore_Signal(self.semaphore()) };
    }

    #[inline]
    pub fn try_lock(&self) -> bool {
        // SAFETY: as `lock`. `Wait(TInt)` returns `KErrNone` when it took the token and
        // `KErrTimedOut` when the microsecond passed without one.
        unsafe { RSemaphore_WaitTimeout(self.semaphore(), TRY_LOCK_MICROSECONDS) == 0 }
    }
}
