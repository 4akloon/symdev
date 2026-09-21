//! `thread::park` / `unpark` over an `RSemaphore` with no tokens.
//!
//! A `Parker` belongs to one thread and is a binary signal, not a counter: `unpark`
//! twice before a `park` must leave exactly one `park` un-blocked. The semaphore alone
//! cannot say that, so the usual three-state atomic sits in front of it and the
//! semaphore is only touched when a thread really has to sleep. That also means an
//! `unpark` of a thread that is not parked costs one compare-exchange and no kernel
//! call at all.
//!
//! `sys::sync::once::queue` and `sys::sync::rwlock::queue` — the portable
//! implementations this target uses for `Once` and `RwLock` — are built on
//! `thread::park`, so this is what makes those two real rather than spin locks.

use super::super::lazy_handle::LazyHandle;
use crate::pin::Pin;
use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crate::sync::atomic::{Atomic, AtomicI8};
use crate::time::Duration;
use symbian_sys::thread::{
    EOWNER_PROCESS, RSemaphore, RSemaphore_CreateLocal, RSemaphore_Signal, RSemaphore_Wait,
    RSemaphore_WaitTimeout,
};

const EMPTY: i8 = 0;
const PARKED: i8 = -1;
const NOTIFIED: i8 = 1;

/// The longest `RSemaphore::Wait` can be asked for: the timeout is a `TInt` of
/// microseconds. A longer `park_timeout` returns early, which `thread::park_timeout`
/// permits — it is documented to wake spuriously.
const MAX_WAIT_MICROSECONDS: u128 = i32::MAX as u128;

pub struct Parker {
    state: Atomic<i8>,
    semaphore: LazyHandle<RSemaphore>,
}

impl Parker {
    /// # Safety
    /// `parker` must point to writable, uninitialised or dead memory for a `Parker`.
    pub unsafe fn new_in_place(parker: *mut Parker) {
        // SAFETY: the caller's contract; nothing is read from `parker` first.
        unsafe {
            parker.write(Parker {
                state: AtomicI8::new(EMPTY),
                semaphore: LazyHandle::new(RSemaphore::NULL),
            })
        }
    }

    #[inline]
    fn semaphore(&self) -> *mut RSemaphore {
        self.semaphore.get(|handle| {
            // SAFETY: `LazyHandle` runs this once before any other thread reaches the
            // handle. Zero tokens: the first `Wait` blocks until an `unpark` signals.
            unsafe { RSemaphore_CreateLocal(handle, 0, EOWNER_PROCESS) }
        })
    }

    /// # Safety
    /// Must only be called by the thread that owns this `Parker`.
    pub unsafe fn park(self: Pin<&Self>) {
        // A notification that arrived before this call is consumed without sleeping.
        if self.state.fetch_sub(1, Acquire) == NOTIFIED {
            return;
        }
        loop {
            // SAFETY: a created, process-local semaphore; `Wait` is non-leaving and
            // takes the handle as argument 0.
            unsafe { RSemaphore_Wait(self.semaphore()) };
            if self.state.compare_exchange(NOTIFIED, EMPTY, Acquire, Relaxed).is_ok() {
                return;
            }
            // The semaphore was signalled by an `unpark` whose state change this thread
            // has not observed yet; going round again is the same as a spurious wakeup.
        }
    }

    /// # Safety
    /// Must only be called by the thread that owns this `Parker`.
    pub unsafe fn park_timeout(self: Pin<&Self>, timeout: Duration) {
        if self.state.fetch_sub(1, Acquire) == NOTIFIED {
            return;
        }
        let micros = timeout.as_micros().min(MAX_WAIT_MICROSECONDS) as i32;
        // `Wait(0)` means "no timeout" on this platform (observed, experiment 80), so a
        // zero duration has to ask for the shortest wait there is instead of none.
        // SAFETY: as `park`.
        unsafe { RSemaphore_WaitTimeout(self.semaphore(), micros.max(1)) };
        // Whether the token arrived or the wait expired, the state is what decides:
        // anything other than `NOTIFIED` leaves with `EMPTY`, which is a wakeup the
        // caller must treat as spurious.
        self.state.swap(EMPTY, Acquire);
    }

    pub fn unpark(self: Pin<&Self>) {
        if self.state.swap(NOTIFIED, Release) == PARKED {
            // SAFETY: a created, process-local semaphore; `Signal` is non-leaving and
            // takes the handle as argument 0. The owning thread is inside `Wait`, or
            // is about to be, and this token is what releases it.
            unsafe { RSemaphore_Signal(self.semaphore()) };
        }
    }
}
