//! A spawned task: a future the executor owns, and the flag that says it needs
//! polling.
use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::sync::Arc;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Waker};

use crate::waker::Readiness;

pub(crate) struct Task {
    /// `None` once the future has finished, and briefly while it is being polled — the
    /// future is taken out so that nothing holds a borrow across the poll, because a
    /// future may spawn another task or drop a request, and both reach back into the
    /// executor.
    future: RefCell<Option<Pin<Box<dyn Future<Output = ()>>>>>,
    /// Whether the future has finished, kept separately because `future` is also empty
    /// for the duration of a poll.
    finished: core::cell::Cell<bool>,
    ready: Arc<Readiness>,
    /// Built once. Each `Waker` clone is an `Arc` clone and therefore a kernel call, so
    /// the task holds one and lends it out.
    waker: Waker,
}

impl Task {
    pub(crate) fn new(future: impl Future<Output = ()> + 'static) -> Rc<Self> {
        let ready = Readiness::new();
        let waker = ready.waker();
        Rc::new(Self {
            future: RefCell::new(Some(Box::pin(future))),
            finished: core::cell::Cell::new(false),
            ready,
            waker,
        })
    }

    /// Whether a wake has arrived since the last poll. It does not clear the flag;
    /// [`Self::poll`] does, so that a wake *during* a poll is not lost.
    pub(crate) fn is_ready(&self) -> bool {
        !self.finished.get() && self.ready.is_set()
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.finished.get()
    }

    /// One poll, if the task has been woken. Dropping the finished future here is what
    /// cancels any request it still held.
    pub(crate) fn poll(&self) {
        if self.finished.get() || !self.ready.take() {
            return;
        }
        let Some(mut future) = self.future.borrow_mut().take() else {
            // Already being polled further down the stack. The flag was taken above, so
            // mark it again: whoever is polling will see it on their next round.
            self.ready.mark();
            return;
        };
        let mut cx = Context::from_waker(&self.waker);
        if future.as_mut().poll(&mut cx).is_pending() {
            *self.future.borrow_mut() = Some(future);
        } else {
            self.finished.set(true);
        }
    }
}
