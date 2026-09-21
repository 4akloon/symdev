//! A kernel handle created on first use, so that a `Mutex`, a `Condvar` or a `Parker`
//! can still have a `const fn new()` and live in a `static`.
//!
//! Symbian has no static initialiser for a kernel object: `RSemaphore::CreateLocal` is
//! a call, and a `const` context cannot make one. Every other platform with the same
//! problem solves it the same way; this is the smallest version of it, and it is here
//! rather than in each primitive because all three need exactly this and nothing more.
//!
//! It deliberately does **not** use [`crate::sys::sync::Once`]: that is built on
//! `thread::park`, which is built on a `Parker`, which is one of the three things this
//! creates. The state machine below needs nothing but a compare-exchange, which on this
//! target is a libcall over one process-wide `RFastLock` (experiment 80).

#![allow(dead_code, reason = "only the Symbian backends use this")]

use crate::cell::UnsafeCell;
use crate::hint::spin_loop;
use crate::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crate::sync::atomic::{Atomic, AtomicI32};

const UNINIT: i32 = 0;
const BUSY: i32 = 1;
const READY: i32 = 2;

pub(super) struct LazyHandle<T> {
    state: Atomic<i32>,
    handle: UnsafeCell<T>,
}

// SAFETY: the handle is written exactly once, by the thread that wins the
// `UNINIT -> BUSY` exchange, and is published with a `Release` store that every reader
// sees through an `Acquire` load. After that it is a kernel handle, which euser
// documents as usable from any thread of the process (`EOwnerProcess`), and the
// primitives above only ever pass it back to euser.
unsafe impl<T: Send> Sync for LazyHandle<T> {}
// SAFETY: as above.
unsafe impl<T: Send> Send for LazyHandle<T> {}

impl<T> LazyHandle<T> {
    /// `null` is the handle's zero value — a `TInt` of 0, which is what a
    /// default-constructed `RHandleBase` holds.
    pub(super) const fn new(null: T) -> Self {
        Self { state: AtomicI32::new(UNINIT), handle: UnsafeCell::new(null) }
    }

    /// The handle, creating it on the first call.
    ///
    /// `create` is the euser `CreateLocal` for this handle and returns a system error
    /// code. A failure has nowhere to go — `Mutex::lock` returns `()` and nothing may
    /// unwind — so it ends the process the way every other unrecoverable runtime
    /// failure in `std` does, with the code in the message.
    #[inline]
    pub(super) fn get(&self, create: impl FnOnce(*mut T) -> i32) -> *mut T {
        if self.state.load(Acquire) == READY {
            return self.handle.get();
        }
        self.init(create)
    }

    #[cold]
    fn init(&self, create: impl FnOnce(*mut T) -> i32) -> *mut T {
        match self.state.compare_exchange(UNINIT, BUSY, Acquire, Acquire) {
            Ok(_) => {
                let code = create(self.handle.get());
                if code != 0 {
                    rtabort!("could not create a Symbian kernel handle ({code})");
                }
                self.state.store(READY, Release);
            }
            // Another thread is creating it. The window is one kernel call wide, so a
            // spin is cheaper than anything that would need a second primitive — and a
            // second primitive is exactly what is not available here.
            Err(_) => {
                while self.state.load(Relaxed) != READY {
                    spin_loop();
                }
            }
        }
        self.handle.get()
    }
}
