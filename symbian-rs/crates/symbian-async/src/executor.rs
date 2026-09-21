//! The executor: a task list, a poll round, and the two places a completion can drive
//! it from.
//!
//! There is exactly one executor in a process and it belongs to one thread — the thread
//! whose `CActiveScheduler` it was bound to on its first request. That is not a
//! simplification, it is what an active scheduler *is*: the scheduler is a per-thread
//! object, `CActiveScheduler::Current()` answers differently on every thread, and a
//! `CActive` may only be added to the one on its own thread. Binding is therefore
//! checked rather than assumed, and a second thread that tries to use this executor
//! gets `KErrInUse` instead of corrupting the task list.
use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::ffi::c_void;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicU32, Ordering};

use symbian_core::{ErrorKind, Result, SymbianError};
use symbian_sys::active::{CActiveScheduler_Current, symrs_active_destroy, symrs_scheduler_stop};

use crate::task::Task;
use crate::waker::Readiness;

/// The process's one executor.
///
/// # Safety of the `Sync` claim
///
/// `Executor` holds `Rc`s and `RefCell`s, which are not `Sync`, and this is a `static`,
/// which must be. The claim is kept by the binding above: every path that reaches this
/// value — issuing a request, spawning, a completion callback, a poll round — happens
/// on the thread that owns the bound scheduler, and [`Executor::bind`] refuses any
/// other thread before it can touch the interior. Nothing here is ever reached from a
/// `Waker`, which is the one part of this crate another thread may legitimately hold;
/// waking is an atomic store and nothing more (see [`crate::waker`]).
static EXECUTOR: Executor = Executor::new();

// SAFETY: see the note above — the interior is only ever reached from the thread the
// executor is bound to, and `bind` is what enforces that.
unsafe impl Sync for Executor {}

pub(crate) struct Executor {
    tasks: RefCell<Vec<Rc<Task>>>,
    /// The `CActiveScheduler*` this executor is bound to, as a word, or 0 before the
    /// first request. An atomic because it is the one field a foreign thread may read.
    bound: AtomicU32,
    /// The readiness of the future `block_on` is polling on its own stack, if any. It
    /// is not a task: `block_on` owns it, so that a future given to `block_on` need not
    /// be `'static`.
    root: RefCell<Option<Arc<Readiness>>>,
    /// Whether `CActiveScheduler::Start()` is running, so that a completion knows
    /// whether stopping it is legal.
    started: Cell<bool>,
    /// The active object whose `RunL` is on the stack, or null.
    running: Cell<*mut c_void>,
    /// An active object that asked to be destroyed from inside its own `RunL`, waiting
    /// for that frame to return. At most one can exist, because `RunL` is never
    /// re-entered.
    grave: Cell<*mut c_void>,
}

impl Executor {
    const fn new() -> Self {
        Self {
            tasks: RefCell::new(Vec::new()),
            bound: AtomicU32::new(0),
            root: RefCell::new(None),
            started: Cell::new(false),
            running: Cell::new(null_mut()),
            grave: Cell::new(null_mut()),
        }
    }

    pub(crate) fn get() -> &'static Self {
        &EXECUTOR
    }

    /// Binds this executor to the scheduler installed on the calling thread, or
    /// confirms it is already bound to it.
    ///
    /// # Errors
    ///
    /// `KErrNotReady` when the thread has no scheduler — the fix is [`crate::block_on`],
    /// which installs one, or, in an Avkon application, [`crate::spawn`] onto the one
    /// CONE installed. `KErrInUse` when another thread's scheduler owns this executor.
    pub(crate) fn bind(&self) -> Result<()> {
        // SAFETY: a plain euser static with no arguments; it reads the calling thread's
        // own scheduler slot and returns null when there is none.
        let current = unsafe { CActiveScheduler_Current() } as usize as u32;
        if current == 0 {
            return Err(SymbianError::of(ErrorKind::NotReady));
        }
        match self
            .bound
            .compare_exchange(0, current, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => Ok(()),
            Err(existing) if existing == current => Ok(()),
            Err(_) => Err(SymbianError::of(ErrorKind::InUse)),
        }
    }

    /// Adds a task and polls it once, which is what gets its first request issued.
    pub(crate) fn spawn(&self, task: Rc<Task>) {
        self.tasks.borrow_mut().push(task.clone());
        task.poll();
        self.reap();
    }

    /// Remembers the readiness of the future `block_on` polls on its own stack, so that
    /// a completion knows when to stop the scheduler. Returns the previous one, which a
    /// nested `block_on` would have to put back — it cannot nest, because it installs a
    /// scheduler and a second `Install` is `KErrInUse`.
    pub(crate) fn set_root(&self, root: Option<Arc<Readiness>>) -> Option<Arc<Readiness>> {
        self.root.replace(root)
    }

    pub(crate) fn set_started(&self, started: bool) {
        self.started.set(started);
    }

    /// A request completed: drive every task that is now ready, then stop the scheduler
    /// if the future `block_on` is waiting for has been woken.
    ///
    /// This runs inside `CSymRsActive::RunL`, so the one thing it must not do is delete
    /// the active object below it on the stack — hence [`Self::defer_destroy`].
    pub(crate) fn completed(&self, active: *mut c_void) {
        // The previous `RunL` has returned by now, so anything it left behind can go.
        self.collect();
        let previous = self.running.replace(active);
        self.run_ready();
        self.running.set(previous);
        if self.started.get() && self.root.borrow().as_ref().is_some_and(|r| r.is_set()) {
            // SAFETY: `started` is only true between `symrs_scheduler_start` and its
            // return, which is exactly when `Stop()` is legal.
            unsafe { symrs_scheduler_stop() };
        }
    }

    /// Polls every task whose waker has been marked, until none is.
    fn run_ready(&self) {
        loop {
            let ready: Vec<Rc<Task>> = self
                .tasks
                .borrow()
                .iter()
                .filter(|task| task.is_ready())
                .cloned()
                .collect();
            if ready.is_empty() {
                break;
            }
            // The borrow above is released before any polling: a task may spawn another
            // one, and a future may drop a request, and both reach back in here.
            for task in ready {
                task.poll();
            }
            self.reap();
        }
    }

    /// Forgets the tasks that have finished. Their futures are already dropped; this
    /// only releases the `Rc`s.
    fn reap(&self) {
        self.tasks.borrow_mut().retain(|task| !task.is_finished());
    }

    /// Whether `active` must not be deleted yet, because its own `RunL` is on the stack
    /// below the caller. At most one object is ever in this state.
    pub(crate) fn defer_destroy(&self, active: *mut c_void) -> bool {
        if active.is_null() || active != self.running.get() {
            return false;
        }
        self.grave.set(active);
        true
    }

    /// Destroys whatever [`Self::defer_destroy`] kept. Safe to call at any point where
    /// no `RunL` is on the stack: the top of the next completion, `block_on`'s loop
    /// after `Start()` returns, and shutdown.
    pub(crate) fn collect(&self) {
        let active = self.grave.replace(null_mut());
        if !active.is_null() {
            // SAFETY: the `RunL` that forbade this has returned, the request that owned
            // the object has been dropped, and nothing else refers to it.
            unsafe { symrs_active_destroy(active) };
        }
    }

    /// Drops every task and everything waiting to be destroyed, and unbinds. Called
    /// when the scheduler this executor was bound to goes away, because every active
    /// object must be gone before then.
    pub(crate) fn shutdown(&self) {
        let tasks = core::mem::take(&mut *self.tasks.borrow_mut());
        drop(tasks);
        self.collect();
        self.root.replace(None);
        self.started.set(false);
        self.running.set(null_mut());
        self.bound.store(0, Ordering::Release);
    }
}
