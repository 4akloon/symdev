//! [`Request`]: one outstanding Symbian asynchronous request, as a thing a `Future` can
//! poll.
//!
//! Every asynchronous call on this platform has the same shape — it takes a
//! `TRequestStatus&` and completes it later — so this is the only type that has to know
//! about `CActive` at all. What differs between one service and the next is two things:
//! how the request is *issued*, and how it is *cancelled*. The first is a closure the
//! caller passes once; the second is the [`Source`] trait, which is also what keeps the
//! service's handle alive for exactly as long as the kernel might still write into the
//! status.
//!
//! A second asynchronous source is therefore a `Source` implementation and a `Future`
//! around `Request`, with no new C++ and nothing new in the executor: `RSocket`'s
//! `Connect`, `Send`, `RecvOneOrMore` and `Accept` all take a `TRequestStatus&`, and
//! `RSocket::CancelAll` is their `cancel`.
use alloc::boxed::Box;
use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::marker::PhantomData;
use core::task::{Context, Poll, Waker};

use symbian_core::{ErrorKind, Result, SymbianError};
use symbian_sys::active::{
    SymRsActiveVTable, symrs_active_destroy, symrs_active_issued, symrs_active_new,
    symrs_active_status,
};
use symbian_sys::thread::TRequestStatus;

use crate::executor::Executor;

/// `CActive::EPriorityStandard` (`e32base.h` line 1615). Every request this crate makes
/// runs at the same priority, so completions are handled in the order the scheduler
/// found them rather than in an order this SDK invented.
const PRIORITY: i32 = 0;

/// What holds a Symbian asynchronous request open, and how to take it back.
///
/// The implementor owns the handle — `RTimer`, `RSocket` — and must keep it alive; the
/// [`Request`] keeps the `Source` beside the request status so that neither can outlive
/// the other. Both of its obligations are about the kernel's view of the status:
///
/// - [`Source::cancel`] must complete the outstanding request **immediately**, which is
///   what `RTimer::Cancel` and `RSocket::CancelAll` do. `CActive::Cancel` consumes that
///   completion itself; a `cancel` that does not produce one hangs the thread.
/// - `Drop` must not run while a request is outstanding. It cannot: `Request`'s own
///   `Drop` cancels first, and the `Source` lives inside it.
pub trait Source {
    /// Take the request back. Called from `DoCancel`, with C++ frames below.
    fn cancel(&self);
}

/// The heap block the C++ side points at. It never moves: the `Box` is what the
/// `CActive`'s opaque context is, and the `TRequestStatus` it is asked to fill lives in
/// the C++ object, not here.
struct State<S: Source> {
    source: S,
    /// The completion code, once `RunL` has run. `None` while the request is
    /// outstanding.
    result: Cell<Option<i32>>,
    /// Who to wake. Replaced on a poll that arrives with a different waker, as the
    /// `Future` contract requires.
    waker: RefCell<Option<Waker>>,
    /// The `CSymRsActive` this state is the context of. It is here rather than beside
    /// the `Box` so that the completion callback, which is handed only the context, can
    /// tell the executor which active object's `RunL` is on the stack.
    active: Cell<*mut c_void>,
}

/// One outstanding request, owning the active object that receives its completion and
/// the service handle that produces it.
pub struct Request<S: Source> {
    state: Box<State<S>>,
}

impl<S: Source> Request<S> {
    /// Issues one request and returns it ready to be polled.
    ///
    /// `issue` is handed the source and the `TRequestStatus` of a `CActive` that is
    /// already on this thread's scheduler; it must pass that pointer to exactly one
    /// Symbian call. Unlike `symbian_core::net::blocking`, the status **outlives the
    /// call** — that is the whole difference between the two models, and it is why this
    /// one needs the scheduler to decide which completion belongs to whom.
    ///
    /// # Errors
    ///
    /// `KErrNotReady` when no `CActiveScheduler` is installed on this thread: the fix is
    /// `block_on`, which installs one, or — in an Avkon application — spawning onto the
    /// scheduler CONE has already installed. `KErrNoMemory` when the active object
    /// cannot be allocated.
    pub fn issue(
        source: S,
        waker: &Waker,
        issue: impl FnOnce(&S, *mut TRequestStatus),
    ) -> Result<Self> {
        Executor::get().bind()?;
        let mut state = Box::new(State {
            source,
            result: Cell::new(None),
            waker: RefCell::new(Some(waker.clone())),
            active: Cell::new(core::ptr::null_mut()),
        });
        let context = (&raw mut *state).cast::<c_void>();
        // SAFETY: the table is a `const` of function pointers with C ABI and the
        // `'static` lifetime the shim needs, and `context` points at a heap block this
        // `Request` owns and will not free before it destroys the active object.
        let active = unsafe { symrs_active_new(&Vtable::<S>::TABLE, context, PRIORITY) };
        if active.is_null() {
            return Err(SymbianError::of(ErrorKind::NoMemory));
        }
        state.active.set(active);
        let request = Self { state };
        // SAFETY: `active` is the object just created, so its `iStatus` is live and
        // belongs to it. `issue` hands that pointer to one Symbian call, and
        // `symrs_active_issued` is `SetActive()` after the request has been made, which
        // is the order `CActive` documents.
        unsafe {
            issue(&request.state.source, symrs_active_status(active));
            symrs_active_issued(active);
        }
        Ok(request)
    }

    /// The completion code, or `Pending` while the request is outstanding.
    pub fn poll(&self, cx: &mut Context<'_>) -> Poll<i32> {
        if let Some(code) = self.state.result.get() {
            return Poll::Ready(code);
        }
        let mut slot = self.state.waker.borrow_mut();
        if !slot.as_ref().is_some_and(|w| w.will_wake(cx.waker())) {
            *slot = Some(cx.waker().clone());
        }
        Poll::Pending
    }

    /// The service handle, for a caller that must read something off it — the bytes a
    /// socket transferred, say — after the request has completed.
    pub fn source(&self) -> &S {
        &self.state.source
    }
}

impl<S: Source> Drop for Request<S> {
    fn drop(&mut self) {
        // SAFETY: `active` is this request's own object and nothing else refers to it.
        // `symrs_active_destroy` cancels an outstanding request before deleting, which
        // is what stops the kernel writing into a status that has gone away.
        //
        // The one case where it may not be deleted here is dropping a request from
        // inside its *own* `RunL`, where the C++ object is on the stack below us: the
        // executor takes it instead and deletes it once that frame has returned.
        let active = self.state.active.get();
        unsafe {
            if Executor::get().defer_destroy(active) {
                return;
            }
            symrs_active_destroy(active);
        }
    }
}

/// The vtable the shim calls back through, one `const` per source type so that both
/// callbacks are ordinary monomorphised functions with no dispatch of their own.
struct Vtable<S: Source>(PhantomData<S>);

impl<S: Source> Vtable<S> {
    const TABLE: SymRsActiveVTable = SymRsActiveVTable {
        run: Self::run,
        cancel: Self::cancel,
    };

    /// `CSymRsActive::RunL`. Stores the completion, marks the waker and drives the
    /// executor. It returns `KErrNone` always: there is nothing here that can fail, and
    /// the shim's `User::LeaveIfError` after it exists for a callback that one day can.
    unsafe extern "C" fn run(context: *mut c_void, status: i32) -> i32 {
        // SAFETY: `context` is the `State<S>` of the request that owns the active
        // object the scheduler is running, and that request is alive — it cancels and
        // destroys the object before freeing this block. Nothing else holds a reference
        // to it: the executor never re-enters a `RunL`, so no poll is in progress.
        let state = unsafe { &*context.cast::<State<S>>() };
        state.result.set(Some(status));
        if let Some(waker) = state.waker.borrow().as_ref() {
            waker.wake_by_ref();
        }
        Executor::get().completed(state.active.get());
        0
    }

    /// `CSymRsActive::DoCancel`.
    unsafe extern "C" fn cancel(context: *mut c_void) {
        // SAFETY: as above; `Cancel()` is only ever called by this request's own `Drop`
        // or by the executor's shutdown, both of which hold the block alive.
        let state = unsafe { &*context.cast::<State<S>>() };
        state.source.cancel();
    }
}
