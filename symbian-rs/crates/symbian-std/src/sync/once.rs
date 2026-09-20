//! `Once`: `std`'s shape, over the atomics the SDK's compiler-runtime archive provides.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::thread;

/// Nothing has run.
const INCOMPLETE: u32 = 0;
/// Some thread is running the closure.
const RUNNING: u32 = 1;
/// The closure has returned.
const COMPLETE: u32 = 2;

/// A one-time initialisation, as `std::sync::Once`.
///
/// ```ignore
/// static START: Once = Once::new();
/// START.call_once(|| { /* ... */ });
/// ```
///
/// # How it differs from `std`
///
/// - **No poisoning, and nothing to poison.** `std` marks a `Once` poisoned when the
///   closure panics, so later callers see the failure instead of an uninitialised
///   value. Here `panic = "abort"` (design spec §3): a panic ends the process, so there
///   is no "later" for a poisoned state to be observed from. `call_once_force`,
///   `OnceState` and `is_poisoned` therefore do not exist rather than being faked.
/// - **A waiting thread spins with a yield** instead of parking. `std` blocks on a
///   futex; the SDK's only waiting primitives are kernel handles, and giving every
///   `Once` a kernel handle would need a `Once` to create it. `thread::yield_now` is
///   `User::After(0)`, which is the reschedule point experiment 72 used to make the
///   emulator interleave two threads, so the spin does hand the CPU over.
///
/// The state word is an ordinary `AtomicU32`, which on this CPU means a call into the
/// SDK's `__atomic_*` archive and therefore one process-wide `RFastLock` per operation.
/// A completed `Once` still costs that on every `call_once`, so a hot path should keep
/// the value rather than ask again.
pub struct Once {
    state: AtomicU32,
}

impl Once {
    /// A `Once` that has not run. `const`, so it can be a `static`.
    pub const fn new() -> Self {
        Self {
            state: AtomicU32::new(INCOMPLETE),
        }
    }

    /// Runs `f` exactly once for this `Once`, blocking other callers until it returns.
    pub fn call_once<F: FnOnce()>(&self, f: F) {
        if self.state.load(Ordering::Acquire) == COMPLETE {
            return;
        }
        match self.state.compare_exchange(
            INCOMPLETE,
            RUNNING,
            Ordering::Acquire,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                f();
                self.state.store(COMPLETE, Ordering::Release);
            }
            // Another thread is running it; wait for it to publish COMPLETE. It cannot
            // fail to: the only way out of `f` other than returning is ending the
            // process.
            Err(RUNNING) => {
                while self.state.load(Ordering::Acquire) != COMPLETE {
                    thread::yield_now();
                }
            }
            // COMPLETE already, or a value nothing writes.
            Err(_) => {}
        }
    }

    /// Whether [`call_once`](Self::call_once) has returned, as `std::sync::Once`.
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::Acquire) == COMPLETE
    }
}

impl Default for Once {
    fn default() -> Self {
        Self::new()
    }
}
