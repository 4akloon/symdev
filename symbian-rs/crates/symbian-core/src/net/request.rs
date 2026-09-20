//! `blocking`: issue a Symbian asynchronous request and wait for it, which is the whole
//! of the "async runtime" a `std`-shaped socket API needs.
//!
//! `RSocket::Connect`, `Send`, `RecvOneOrMore`, `Accept` and `Shutdown` return `void` and
//! report through a `TRequestStatus&`. The blocking form of that pair — issue, then
//! `User::WaitForRequest` — is a Symbian idiom, not a workaround: it blocks the calling
//! thread on its own request semaphore until the socket server completes the status.
//! There is no `CActive`, no `CActiveScheduler` and no executor anywhere in this crate.
use symbian_sys::esock::{TRequestStatus, TRequestStatusStorage, User_WaitForRequest};

use crate::error::{Result, check};

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
pub fn blocking(issue: impl FnOnce(*mut TRequestStatus)) -> Result<i32> {
    let mut status = TRequestStatusStorage::zeroed();
    // A request the server rejects before it looks at the status would otherwise be read
    // as `KErrNone`, because zeroed storage is success.
    status.set_pending();
    issue(status.as_request_status());
    // SAFETY: the status is a live, 4-aligned `TRequestStatus` of the measured size that
    // `issue` has just handed to the socket server, and this stack frame outlives the
    // wait. `User::WaitForRequest` is a non-leaving `static` member of `User` with one
    // reference argument (experiment 78's ABI does not even arise: there is no `this`).
    // It returns only once this status is no longer `KRequestPending`.
    unsafe { User_WaitForRequest(status.as_request_status()) };
    check(status.code())
}
