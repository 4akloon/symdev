//! `Mutex` and `MutexGuard`: `std`'s shape over a binary `RSemaphore`.

use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicI32, Ordering};

use symbian_sys::euser::RHandleBase_Close;
use symbian_sys::thread::{
    EOWNER_PROCESS, RSemaphore, RSemaphore_CreateLocal, RSemaphore_Signal, RSemaphore_Wait,
    RSemaphore_WaitTimeout,
};

use super::Once;
use crate::io::{Error, ErrorKind, Result};

/// The timeout `try_lock` passes, in microseconds.
///
/// It is **1 and not 0**, and that is observed rather than chosen: `RSemaphore::Wait(0)`
/// on an empty semaphore does not return — 0 means "no timeout", not "do not wait", and
/// a probe that called it never came back. `Wait(1)` on an empty semaphore returns
/// `KErrTimedOut` (−33) straight away, and with a token available it returns
/// `KErrNone` (experiment 80).
const TRY_LOCK_TIMEOUT_MICROSECONDS: i32 = 1;

/// A mutual exclusion lock, as `std::sync::Mutex`.
///
/// ```ignore
/// let counter = Arc::new(Mutex::new(0u32));
/// *counter.lock()? += 1;
/// ```
///
/// # What it is made of, and why that primitive
///
/// A counting `RSemaphore` created with one token. Experiment 72 recommended
/// `RFastLock`, and that recommendation does not survive contact with `try_lock`:
/// **neither `RFastLock` nor `RMutex` has a timed or trying wait**, and `RSemaphore` is
/// the only primitive in 9.3 that does. A `Mutex` whose `lock` and `try_lock` used
/// different kernel objects would not be a mutex at all, so the whole type is built on
/// the one primitive that can do both.
///
/// A one-token semaphore is non-recursive, which is exactly Rust's contract: locking
/// twice from the same thread deadlocks, and deadlock is not undefined behaviour. It is
/// also process-local, which is all a Rust `Mutex` needs.
///
/// # How it differs from `std`
///
/// - **It does not poison, and says so.** `std::sync::Mutex` marks itself poisoned when
///   a thread panics holding the lock, so the next `lock()` returns
///   `Err(PoisonError)`. Here `panic = "abort"` (design spec §3): a panic ends the
///   whole process, so no other thread can ever observe a lock whose data a panic left
///   half-written. There is nothing for poisoning to protect against, so
///   [`PoisonError`](std::sync::PoisonError), `is_poisoned` and `clear_poison` are
///   absent rather than stubbed.
/// - **`lock()` still returns a `Result`**, because the kernel handle is created on
///   first use and creating it can fail. The error is an [`Error`] carrying the
///   `RSemaphore::CreateLocal` code, not a poison flag, so `?` reads the same as it
///   does in `std` and means something honest.
/// - **`try_lock` fails with [`ErrorKind::TimedOut`]**, where `std` says `WouldBlock`.
///   That is what actually happened: the only non-blocking acquisition this OS offers
///   is a one-microsecond timed wait, and it timed out.
/// - **`new` is `const`** as in `std`, so a `Mutex` can be a `static`; the kernel
///   handle is created by the first [`lock`](Self::lock) through a [`Once`].
/// - `get_mut`, `into_inner` and `lock_arc` are not here yet; nothing needed them.
pub struct Mutex<T: ?Sized> {
    /// Creates the semaphore on first use, so that `new` can be `const`.
    created: Once,
    /// `KErrNone` once the semaphore exists, otherwise the `CreateLocal` error.
    /// Meaningful only after `created` has completed.
    status: AtomicI32,
    semaphore: UnsafeCell<RSemaphore>,
    data: UnsafeCell<T>,
}

// SAFETY: the data is reachable only through a `MutexGuard`, which exists only while
// this thread holds the one token of the semaphore, so at most one thread has a
// reference at a time. That makes the `Mutex` shareable whenever the data can move
// between threads, which is `T: Send` — exactly `std`'s bound.
unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
// SAFETY: sending the `Mutex` sends the data; the semaphore is a process-local kernel
// handle usable from any thread of the process (`EOwnerProcess`).
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

impl<T> Mutex<T> {
    /// A new, unlocked mutex holding `value`.
    pub const fn new(value: T) -> Self {
        Self {
            created: Once::new(),
            status: AtomicI32::new(0),
            semaphore: UnsafeCell::new(RSemaphore::null()),
            data: UnsafeCell::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    /// The semaphore, created if this is the first use.
    fn semaphore(&self) -> Result<*mut RSemaphore> {
        self.created.call_once(|| {
            // SAFETY: `call_once` runs this on one thread and every other caller waits
            // for it to finish, so nothing else touches the handle while it is being
            // created. `CreateLocal` is a non-leaving euser member with `this` as
            // argument 0 (experiment 78); one token makes it a mutex.
            let code = unsafe { RSemaphore_CreateLocal(self.semaphore.get(), 1, EOWNER_PROCESS) };
            self.status.store(code, Ordering::Release);
        });
        let code = self.status.load(Ordering::Acquire);
        if code == 0 {
            Ok(self.semaphore.get())
        } else {
            Err(Error::from_raw_os_error(code))
        }
    }

    /// Takes the lock, blocking until it is free.
    ///
    /// Locking twice from the same thread deadlocks, as it does in `std`.
    pub fn lock(&self) -> Result<MutexGuard<'_, T>> {
        let semaphore = self.semaphore()?;
        // SAFETY: a created, process-local semaphore; `Wait` is non-leaving and takes
        // `this` as argument 0. It returns only once this thread holds the one token.
        unsafe { RSemaphore_Wait(semaphore) };
        Ok(MutexGuard { mutex: self })
    }

    /// Takes the lock if it is free, and otherwise fails with [`ErrorKind::TimedOut`].
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>> {
        let semaphore = self.semaphore()?;
        // SAFETY: as `lock`; the timed overload returns `KErrTimedOut` instead of
        // blocking when no token is available.
        let code = unsafe { RSemaphore_WaitTimeout(semaphore, TRY_LOCK_TIMEOUT_MICROSECONDS) };
        if code == 0 {
            Ok(MutexGuard { mutex: self })
        } else {
            Err(Error::from_raw_os_error(code))
        }
    }

    /// Whether this mutex's kernel handle failed to be created.
    ///
    /// Not `std`'s `is_poisoned` — there is no poisoning here — but the same shape of
    /// question, and the only lasting failure a `Mutex` on this platform can have.
    pub fn is_usable(&self) -> bool {
        self.semaphore().is_ok()
    }
}

impl<T: ?Sized> Drop for Mutex<T> {
    fn drop(&mut self) {
        if self.created.is_completed() && self.status.load(Ordering::Acquire) == 0 {
            // SAFETY: the mutex is being dropped, so no guard can exist and no other
            // thread holds a reference; `RSemaphore::Close` is `RHandleBase::Close`,
            // non-leaving and safe on a handle that was created.
            unsafe { RHandleBase_Close(self.semaphore.get().cast()) };
        }
    }
}

impl<T: Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: fmt::Debug> fmt::Debug for Mutex<T> {
    /// Prints the data when the lock is free, as `std` does, and says so when it is not
    /// — a `Debug` that blocks would be a trap.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.try_lock() {
            Ok(guard) => f.debug_struct("Mutex").field("data", &*guard).finish(),
            Err(e) if e.kind() == ErrorKind::TimedOut => {
                f.debug_struct("Mutex").field("data", &"<locked>").finish()
            }
            Err(e) => f.debug_struct("Mutex").field("error", &e).finish(),
        }
    }
}

/// Exclusive access to a [`Mutex`]'s data, released when it is dropped.
pub struct MutexGuard<'a, T: ?Sized> {
    mutex: &'a Mutex<T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: this guard exists only while this thread holds the semaphore's one
        // token, so no other reference to the data can be alive.
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: as `deref`, and `&mut self` makes this the only reference from here.
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: the guard was handed out only after a successful `Wait` on this
        // semaphore, so returning the token is exactly balanced.
        unsafe { RSemaphore_Signal(self.mutex.semaphore.get()) };
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}
