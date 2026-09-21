//! `blocking`: issue one Symbian asynchronous request and wait for it. That pair is
//! the whole of the "runtime" a `std::net` needs.
//!
//! `RSocket::Connect`, `Send`, `RecvOneOrMore`, `Accept` and `Shutdown` return `void`
//! and report through a `TRequestStatus&`. Issue, then `User::WaitForRequest`, is
//! Symbian's own blocking idiom: it parks the calling thread on its request semaphore
//! until the socket server completes the status. There is no `CActive`, no
//! `CActiveScheduler` and no executor underneath `std::net`.
//!
//! # Why the status never leaves this function
//!
//! `User::WaitForRequest` waits on the **thread's** request semaphore and then checks
//! whether this particular status completed, so a thread with two requests outstanding
//! can consume the other's signal. Keeping the status inside one call makes that
//! unrepresentable.
//!
//! # Why it refuses to run under an active scheduler
//!
//! A `CActiveScheduler` has requests outstanding **by design** — that is what it is
//! for. Mixing it with this wait breaks the accounting both ways: this wait can eat an
//! active object's completion, so the scheduler blocks although its request finished,
//! and the scheduler can eat the completion this wait needs, so the call never returns.
//! Neither failure says anything; the program simply stops.
//!
//! So when `CActiveScheduler::Current()` answers with a scheduler, every blocking
//! socket call returns **`KErrInUse`** instead of waiting. This is the same rule
//! `symbian_core::net::blocking` has carried since step 73, and `std::net` inherits it:
//! **an Avkon application, or anything under `symbian_async::block_on`, gets an error
//! and not a hang.** The fix is to do the socket work on a thread of its own —
//! `std::thread::spawn` gives one with no scheduler — which is what blocking I/O on a
//! UI thread needed anyway.

use crate::io;
use symbian_sys::active::CActiveScheduler_Current;
use symbian_sys::esock::{TRequestStatus, User_WaitForRequest};

/// Issues one asynchronous request through `issue` and blocks until it completes,
/// returning the `TInt` the server wrote — or the error, if it is negative.
///
/// `issue` is handed a live `TRequestStatus*` already set to `KRequestPending` and must
/// pass it to exactly one Symbian call. It must not keep the pointer: the storage dies
/// when this function returns, and by then the request has completed.
pub fn blocking(issue: impl FnOnce(*mut TRequestStatus)) -> io::Result<i32> {
    nothing_else_is_waiting()?;
    // `TRequestStatus::new` is already `KRequestPending`; zeroed storage would read as
    // `KErrNone`, so a request the server rejected before looking at it would come back
    // as a success.
    let mut status = TRequestStatus::new();
    issue(&raw mut status);
    // SAFETY: the status is a live, 4-aligned `TRequestStatus` of the measured size
    // that `issue` has just handed to the socket server, and this stack frame outlives
    // the wait. `User::WaitForRequest` is a non-leaving `static` member of `User`
    // taking one reference, and it returns only once this status is no longer
    // `KRequestPending`.
    unsafe { User_WaitForRequest(&raw mut status) };
    if status.status < 0 {
        Err(io::Error::from_raw_os_error(status.status))
    } else {
        Ok(status.status)
    }
}

/// `KErrInUse` (-14) when this thread has a `CActiveScheduler`. See the module note.
fn nothing_else_is_waiting() -> io::Result<()> {
    // SAFETY: a plain euser static with no arguments, reading the calling thread's own
    // scheduler slot; null when there is none, which is the case for every program
    // that uses neither `symbian_async` nor the Avkon framework.
    if unsafe { CActiveScheduler_Current() }.is_null() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::ResourceBusy,
            "a CActiveScheduler is installed on this thread, and a blocking socket call \
             would eat its completions (KErrInUse): do the socket work on a thread of \
             its own",
        ))
    }
}
