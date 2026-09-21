//! The scheduler a console application owns, as an RAII value.
//!
//! This type exists only inside [`crate::block_on`], and deliberately: it is the half
//! of the crate a **GUI** application must never reach. CONE creates and installs
//! `CCoeScheduler` before any application code runs and `CCoeEnv` is itself a `CActive`
//! on it, so installing a second scheduler panics and stopping CONE's would end the
//! application (`docs/research/avkon-rust-spec.md` §1.3). An Avkon application uses
//! [`crate::spawn`], which adds to whatever scheduler is already there and never starts
//! or stops it.
use core::ffi::c_void;
use core::ptr::null_mut;

use symbian_core::{Result, check};
use symbian_sys::active::{
    symrs_scheduler_install, symrs_scheduler_start, symrs_scheduler_uninstall,
};

use crate::executor::Executor;

pub(crate) struct Scheduler {
    handle: *mut c_void,
}

impl Scheduler {
    /// `new CActiveScheduler` and `CActiveScheduler::Install`.
    ///
    /// # Errors
    ///
    /// `KErrInUse` when this thread already has one — which is what a GUI application
    /// gets, and the signal to use [`crate::spawn`] instead. `KErrNoMemory` on a full
    /// heap.
    pub(crate) fn install() -> Result<Self> {
        let mut handle: *mut c_void = null_mut();
        // SAFETY: an out-pointer to a live local; the shim refuses to install a second
        // scheduler rather than letting `Install` panic.
        check(unsafe { symrs_scheduler_install(&raw mut handle) })?;
        Ok(Self { handle })
    }

    /// Runs the scheduler until a completion stops it. Every `RunL` in the program —
    /// and so every poll — happens underneath this call.
    pub(crate) fn run(&self) -> Result<()> {
        let executor = Executor::get();
        executor.set_started(true);
        // SAFETY: a scheduler is installed (this one), and `Start` is TRAPped in the
        // shim, so a leave from any `RunL` comes back as a code instead of crossing
        // this Rust frame as a C++ exception.
        let code = unsafe { symrs_scheduler_start() };
        executor.set_started(false);
        // Anything the last `RunL` could not delete while it was on the stack.
        executor.collect();
        check(code)?;
        Ok(())
    }
}

impl Drop for Scheduler {
    fn drop(&mut self) {
        // Every active object must be gone before the scheduler it is queued on is:
        // dropping the tasks cancels and deletes theirs.
        Executor::get().shutdown();
        // SAFETY: `handle` is the scheduler this value installed and nothing else
        // refers to it; the shim uninstalls before deleting.
        unsafe { symrs_scheduler_uninstall(self.handle) };
    }
}
