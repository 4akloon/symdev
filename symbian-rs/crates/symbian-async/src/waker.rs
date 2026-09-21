//! The bridge from a completed `TRequestStatus` to a Rust `Waker`, and the one place
//! this crate's thread-safety claim has to be true.
//!
//! [`core::task::Waker`] is `Send + Sync` by declaration: any future may clone one and
//! hand it to whatever completes its work, and the standard `Arc`-based constructor
//! ([`alloc::task::Wake`]) requires the same of the value behind it. So [`Readiness`]
//! holds **nothing but an `AtomicU32`**, and waking is one atomic store. That is a
//! claim this SDK can actually keep — 32-bit atomics are real here since experiment 80
//! — rather than an `unsafe impl Send for` a cell full of raw pointers.
//!
//! Everything that is *not* thread-safe — polling futures, touching the task list,
//! stopping the scheduler — happens in the `RunL` the shim calls, which is on the
//! scheduler's own thread by construction. The waker only leaves a mark.
//!
//! # What that costs, and what it means for another thread
//!
//! Every atomic operation on this device is a kernel call over a process-wide
//! `RFastLock`, about 90 times the cost of a plain increment (experiment 72's table).
//! A wake is one store and a poll round is one swap per task, so a timer completing
//! costs two of them; that is why the executor scans a short list of flags instead of
//! keeping a shared ready queue.
//!
//! A `Waker` sent to **another thread** and woken there is sound — the store is atomic
//! — but it does not interrupt the scheduler this executor runs on: the mark is only
//! seen the next time that thread's scheduler runs something. Waking an executor from
//! another thread needs `RThread::RequestComplete` against a `TRequestStatus` this
//! thread is waiting on, which is **TODO: not observed** and is not offered here.
use alloc::sync::Arc;
use alloc::task::Wake;
use core::sync::atomic::{AtomicU32, Ordering};
use core::task::Waker;

/// One task's "poll me" flag: the whole of what a wake does.
pub(crate) struct Readiness {
    ready: AtomicU32,
}

impl Readiness {
    /// A flag that starts **set**, because a future that has never been polled is
    /// exactly a future that needs polling.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicU32::new(1),
        })
    }

    /// Clears the flag and says whether it was set. `Acquire` pairs with the `Release`
    /// in [`Self::mark`], so everything the waker's thread wrote before waking is
    /// visible to the poll that follows.
    pub(crate) fn take(&self) -> bool {
        self.ready.swap(0, Ordering::Acquire) != 0
    }

    /// Whether a wake has arrived since the last [`Self::take`], without clearing it.
    pub(crate) fn is_set(&self) -> bool {
        self.ready.load(Ordering::Acquire) != 0
    }

    /// The whole of waking.
    pub(crate) fn mark(&self) {
        self.ready.store(1, Ordering::Release);
    }

    /// The `Waker` a future is polled with. Built once per task and handed out by
    /// reference, because each clone of it is an `Arc` clone and therefore a kernel
    /// call.
    pub(crate) fn waker(self: &Arc<Self>) -> Waker {
        Waker::from(self.clone())
    }
}

impl Wake for Readiness {
    fn wake(self: Arc<Self>) {
        self.mark();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.mark();
    }
}
