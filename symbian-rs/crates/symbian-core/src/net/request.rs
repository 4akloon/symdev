//! `blocking`: issue a Symbian asynchronous request and wait for it, which is the whole
//! of the "async runtime" a `std`-shaped socket API needs.
//!
//! `RSocket::Connect`, `Send`, `RecvOneOrMore`, `Accept` and `Shutdown` return `void` and
//! report through a `TRequestStatus&`. The blocking form of that pair — issue, then
//! `User::WaitForRequest` — is a Symbian idiom, not a workaround: it blocks the calling
//! thread on its own request semaphore until the socket server completes the status.
//! There is no `CActive`, no `CActiveScheduler` and no executor anywhere in this crate
//! — and, since step 73, a scheduler on this thread is the one thing that makes this
//! function refuse: the two ways of waiting for a `TRequestStatus` must not share a
//! thread. See [`blocking`].
use symbian_sys::active::CActiveScheduler_Current;
use symbian_sys::esock::{TRequestStatus, User_WaitForRequest};

use crate::ErrorKind;
use crate::error::{Result, SymbianError, check};

/// Issues one asynchronous request through `issue` and blocks until it completes,
/// returning the `TInt` the server wrote — or the error, if it is negative.
///
/// `issue` is handed a live `TRequestStatus*` already set to `KRequestPending` and must
/// pass it to exactly one Symbian call that takes ownership of it. It must not keep the
/// pointer: the storage dies when this function returns, and by then the request has
/// completed.
///
/// # Why the caller cannot hold the status
///
/// `User::WaitForRequest` waits on the *thread's* request semaphore and then checks
/// whether this particular status completed, so a thread that has two requests
/// outstanding and waits for one of them can consume the other's signal. Keeping the
/// status inside this function makes that unrepresentable: one request is issued, waited
/// for and finished before anything else can be.
///
/// # Why it refuses to run under an active scheduler
///
/// The other thing on this thread that may have requests outstanding is a
/// `CActiveScheduler`, and it has them *by design* — that is what it is for. Mixing the
/// two breaks the accounting in both directions: this wait can consume the semaphore
/// signal of an active object, so the scheduler then blocks although a request has
/// completed, and the scheduler can consume the signal this wait needs, so this call
/// never returns. Neither failure says anything; the program simply stops.
///
/// So when `CActiveScheduler::Current()` answers with a scheduler, this returns
/// `KErrInUse` instead of waiting. The fix is one of two things: **await** the
/// asynchronous form (`symbian_async`, step 73) on this thread, or make the blocking
/// call on a thread of its own — a worker created with `symbian_std::thread::spawn` has
/// no scheduler and may block freely. In an Avkon application the scheduler is CONE's
/// and blocking the UI thread was always a bug.
///
/// The check is one euser call and no state of this crate's own: the scheduler is a
/// per-thread object and `Current()` is the platform's own answer to "is anything else
/// waiting for a completion here".
pub fn blocking(issue: impl FnOnce(*mut TRequestStatus)) -> Result<i32> {
    // SAFETY: a plain euser static with no arguments, reading the calling thread's own
    // scheduler slot; null when there is none, which is the case for every program
    // that does not use `symbian_async` or the Avkon framework.
    if !unsafe { CActiveScheduler_Current() }.is_null() {
        return Err(SymbianError::of(ErrorKind::InUse));
    }
    // `TRequestStatus::new` is already `KRequestPending`; a zeroed status would read as
    // `KErrNone`, so a request the server rejected before looking at it would come back
    // as a success.
    let mut status = TRequestStatus::new();
    issue(&raw mut status);
    // SAFETY: the status is a live, 4-aligned `TRequestStatus` of the measured size that
    // `issue` has just handed to the socket server, and this stack frame outlives the
    // wait. `User::WaitForRequest` is a non-leaving `static` member of `User` taking one
    // reference (so experiment 78's `this` question does not even arise), and it returns
    // only once this status is no longer `KRequestPending`.
    unsafe { User_WaitForRequest(&raw mut status) };
    check(status.status)
}
