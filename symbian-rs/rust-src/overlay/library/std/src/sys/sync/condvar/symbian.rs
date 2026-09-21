//! `Condvar` over `RCondVar`, the kernel condition variable of Symbian OS 9.3.
//!
//! # Why there is a second mutex in here
//!
//! `RCondVar::Wait` takes an `RMutex&` and does the one thing that makes a condition
//! variable work: it releases that mutex and blocks as a single, uninterruptible step.
//! It will only take an `RMutex`, and [`super::super::mutex::Mutex`] is an
//! `RSemaphore`, because a semaphore is the only primitive on this OS with a timed wait
//! and `std`'s `Mutex` needs `try_lock`.
//!
//! So the condition variable brings its own `RMutex`, used for nothing but pairing with
//! `RCondVar`, and hands the caller's mutex back and forth around it:
//!
//! ```text
//! wait(user):   inner.Wait()  →  user.unlock()  →  cv.Wait(inner)  →  inner.Signal()  →  user.lock()
//! notify:       inner.Wait()  →  cv.Signal()    →  inner.Signal()
//! ```
//!
//! **No wakeup is lost**, and the ordering is what proves it. A waiter takes `inner`
//! *while it still holds the user's mutex*, and only then releases the user's mutex. A
//! notifier can therefore not reach `inner.Wait()` before the waiter has taken `inner`
//! — it would still be blocked on the user's mutex, which is where the condition it is
//! signalling about was changed — and once the waiter has `inner`, the notifier's
//! `inner.Wait()` blocks until `cv.Wait` has released it, by which point the waiter is
//! queued on the condition variable and `Signal` reaches it.
//!
//! A notification that arrives when nobody is waiting is dropped, exactly as
//! `pthread_cond_signal` drops one, which is why the caller must re-check its condition
//! in a loop. `std::sync::Condvar` documents that requirement and also permits spurious
//! wakeups, so nothing here needs to promise more.

use super::super::lazy_handle::LazyHandle;
use super::super::mutex::Mutex;
use crate::time::Duration;
use symbian_sys::sync::{
    RCondVar, RCondVar_Broadcast, RCondVar_CreateLocal, RCondVar_Signal, RCondVar_TimedWait,
    RCondVar_Wait, RMutex, RMutex_CreateLocal, RMutex_Signal, RMutex_Wait,
};
use symbian_sys::thread::EOWNER_PROCESS;

/// `KErrTimedOut` (`e32err.h`), what `RCondVar::TimedWait` returns when it expires.
const KERR_TIMED_OUT: i32 = -33;

/// The longest `TimedWait` can be asked for: its timeout is a `TInt` of microseconds,
/// so a little over 35 minutes. A longer wait is done in slices of this length, and
/// each slice that expires is reported to the caller as a wakeup that was not a
/// notification — which is a spurious wakeup, and legal.
const MAX_WAIT_MICROSECONDS: u128 = i32::MAX as u128;

pub struct Condvar {
    condvar: LazyHandle<RCondVar>,
    inner: LazyHandle<RMutex>,
}

impl Condvar {
    #[inline]
    pub const fn new() -> Condvar {
        Condvar { condvar: LazyHandle::new(RCondVar::NULL), inner: LazyHandle::new(RMutex::NULL) }
    }

    #[inline]
    fn handles(&self) -> (*mut RCondVar, *mut RMutex) {
        let condvar = self.condvar.get(|handle| {
            // SAFETY: `LazyHandle` runs this once before any other thread can reach the
            // handle; `CreateLocal` is a non-leaving euser member taking `this` as
            // argument 0 (experiment 78).
            unsafe { RCondVar_CreateLocal(handle, EOWNER_PROCESS) }
        });
        // SAFETY: as above.
        let inner = self.inner.get(|handle| unsafe { RMutex_CreateLocal(handle, EOWNER_PROCESS) });
        (condvar, inner)
    }

    #[inline]
    pub fn notify_one(&self) {
        let (condvar, inner) = self.handles();
        // SAFETY: created, process-local kernel handles; all three are non-leaving
        // euser members taking `this` as argument 0. The mutex is taken and released
        // by this thread, in that order, so the pairing `RCondVar` requires holds.
        unsafe {
            RMutex_Wait(inner);
            RCondVar_Signal(condvar);
            RMutex_Signal(inner);
        }
    }

    #[inline]
    pub fn notify_all(&self) {
        let (condvar, inner) = self.handles();
        // SAFETY: as `notify_one`.
        unsafe {
            RMutex_Wait(inner);
            RCondVar_Broadcast(condvar);
            RMutex_Signal(inner);
        }
    }

    /// # Safety
    /// The calling thread must hold `mutex`.
    pub unsafe fn wait(&self, mutex: &Mutex) {
        let (condvar, inner) = self.handles();
        // SAFETY: the caller holds `mutex`, so unlocking it here is sound; `inner` is
        // taken before that and released after `RCondVar::Wait` has reacquired it, so
        // this thread's lock and unlock of `inner` are balanced. See the module
        // documentation for why this order loses no wakeup.
        unsafe {
            RMutex_Wait(inner);
            mutex.unlock();
            RCondVar_Wait(condvar, inner);
            RMutex_Signal(inner);
        }
        mutex.lock();
    }

    /// # Safety
    /// The calling thread must hold `mutex`.
    pub unsafe fn wait_timeout(&self, mutex: &Mutex, dur: Duration) -> bool {
        let (condvar, inner) = self.handles();
        let micros = dur.as_micros().min(MAX_WAIT_MICROSECONDS) as i32;
        // SAFETY: as `wait`; `TimedWait` releases and reacquires `inner` exactly as
        // `Wait` does, whether or not the timeout expires.
        let code = unsafe {
            RMutex_Wait(inner);
            mutex.unlock();
            let code = RCondVar_TimedWait(condvar, inner, micros.max(1));
            RMutex_Signal(inner);
            code
        };
        mutex.lock();
        code != KERR_TIMED_OUT
    }
}
