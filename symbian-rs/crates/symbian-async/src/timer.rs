//! `sleep`: the first real asynchronous source, over `RTimer`.
//!
//! `RTimer::After(TRequestStatus&, TTimeIntervalMicroSeconds32)` is the cheapest honest
//! one on this platform — a kernel timer, no server session, one handle — and it is
//! what makes `sleep(duration).await` a thing an application can write.
//!
//! It is not `symbian_std::thread::sleep`, and the difference is the whole point of
//! this crate: `thread::sleep` is `User::After`, which stops the thread, so nothing
//! else on it runs; this one suspends **one future** and leaves the scheduler free to
//! run everything else.
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use symbian_core::{ErrorKind, Result, SymbianError, check};
use symbian_sys::euser::RHandleBase_Close;
use symbian_sys::thread::TRequestStatus;
use symbian_sys::time::{RTimer, RTimer_After, RTimer_Cancel, RTimer_CreateLocal};

use crate::request::{Request, Source};

/// The longest sleep `RTimer::After` can express: its interval is a `TInt` of
/// microseconds, so 2 147 483 647 µs — 35 minutes 47 seconds. A longer one is an error
/// rather than a silently shorter sleep.
pub const MAX_SLEEP_MICROS: u128 = i32::MAX as u128;

/// An `RTimer` handle, owned for as long as a request can complete against it.
struct Timer {
    /// euser takes `this` as a mutable pointer on every member, and `Source::cancel`
    /// has only `&self`. The cell is the honest way to say that the kernel — not Rust —
    /// decides when this word changes.
    handle: UnsafeCell<RTimer>,
}

impl Timer {
    fn create() -> Result<Self> {
        let handle = UnsafeCell::new(RTimer::null());
        // SAFETY: a live, uniquely owned `RTimer` of the measured size (4 bytes, one
        // handle word). `CreateLocal` is non-leaving and returns a system error code.
        check(unsafe { RTimer_CreateLocal(handle.get()) })?;
        Ok(Self { handle })
    }

    fn after(&self, status: *mut TRequestStatus, micros: i32) {
        // SAFETY: `status` is the `iStatus` of a live `CActive` on this thread's
        // scheduler, and `handle` is an open timer this value owns. No other reference
        // to either exists: the executor is single-threaded and never re-enters a poll.
        unsafe { RTimer_After(self.handle.get(), status, micros) };
    }
}

impl Source for Timer {
    fn cancel(&self) {
        // SAFETY: as above. `RTimer::Cancel` completes an outstanding `After` at once
        // with `KErrCancel`, which is what `CActive::Cancel` then consumes.
        unsafe { RTimer_Cancel(self.handle.get()) };
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // SAFETY: the request that could complete against this timer has already been
        // cancelled — `Request`'s own `Drop` runs before its `Source`'s. `Close` is
        // safe on a handle that was never opened.
        unsafe { RHandleBase_Close(self.handle.get().cast()) };
    }
}

/// A sleep that has not finished, and its timer.
///
/// The timer is created and the request issued on the **first poll**, not when `sleep`
/// is called, so a `Sleep` that is never awaited costs nothing and holds no handle.
pub struct Sleep {
    interval: Result<i32>,
    request: Option<Request<Timer>>,
}

/// Completes after `duration` has passed, without blocking the thread.
///
/// ```ignore
/// sleep(Duration::from_millis(300)).await?;
/// ```
///
/// # Errors
///
/// `KErrOverflow` for a duration longer than [`MAX_SLEEP_MICROS`]; `KErrNotReady` when
/// there is no scheduler on this thread to complete the request (see
/// [`crate::block_on`]); `KErrCancel` if the request is cancelled from elsewhere. Every
/// one of them arrives from `.await`, because the handle is opened on the first poll.
pub fn sleep(duration: core::time::Duration) -> Sleep {
    let micros = duration.as_micros();
    Sleep {
        interval: if micros > MAX_SLEEP_MICROS {
            Err(SymbianError::of(ErrorKind::Overflow))
        } else {
            Ok(micros as i32)
        },
        request: None,
    }
}

impl Future for Sleep {
    type Output = Result<()>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Sleep` is `Unpin`: the address that has to stay still is the request's heap
        // block, which `Request` owns, not this value.
        let this = self.get_mut();
        let request = match &this.request {
            Some(request) => request,
            None => {
                let interval = match this.interval {
                    Ok(interval) => interval,
                    Err(error) => return Poll::Ready(Err(error)),
                };
                let timer = match Timer::create() {
                    Ok(timer) => timer,
                    Err(error) => return Poll::Ready(Err(error)),
                };
                match Request::issue(timer, cx.waker(), |timer, status| {
                    timer.after(status, interval)
                }) {
                    Ok(request) => this.request.insert(request),
                    Err(error) => return Poll::Ready(Err(error)),
                }
            }
        };
        request.poll(cx).map(|code| check(code).map(|_| ()))
    }
}
