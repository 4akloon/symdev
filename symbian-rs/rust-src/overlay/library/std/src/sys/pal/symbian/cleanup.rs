//! `CTrapCleanup`: the per-thread trap handler and cleanup stack, which nothing on
//! this platform installs for you.
//!
//! # Why `std` has to install one, and what happened without it
//!
//! Symbian's cleanup stack is **per thread** and it does not exist until somebody
//! calls `CTrapCleanup::New()`. A thread without one panics `E32USER-CBase 69`
//! (`EClnNoTrapHandlerInstalled`, `e32panic.h` line 2667) the moment any code reaches
//! `CleanupStack::PushL` — and a panic is not a leave, so no `TRAP` anywhere catches
//! it and the thread simply dies.
//!
//! That is not a theoretical risk. `RFs::GetDir` uses the cleanup stack inside its own
//! trap harness, so `std::fs::read_dir` killed the calling thread with exactly that
//! panic until this existed. The emulator logs it at `Kernel:trace`; at the default
//! filter it is invisible, which is why it looked like the silent death of experiment
//! 76 and was not.
//!
//! So a Symbian `E32Main` conventionally begins with `CTrapCleanup::New()` and every
//! thread it creates does the same. `std::os::symbian::start` **is** this program's
//! `E32Main` body and `sys::thread`'s trampoline is its thread entry, so both do it
//! here, once, for every application — instead of leaving each one to find out.

use symbian_sys::shim::symrs_cleanup_destroy;
use symbian_sys::thread::{CTrapCleanup, CTrapCleanup_New};

/// This thread's trap handler and cleanup stack, uninstalled and freed on drop.
pub struct TrapCleanup(*mut CTrapCleanup);

impl TrapCleanup {
    /// `CTrapCleanup::New()`. `None` on a full heap, which is not fatal by itself: a
    /// program that never reaches the cleanup stack runs perfectly without one, and a
    /// program that does will panic where it does, which is where the diagnosis is.
    pub fn install() -> Option<Self> {
        // SAFETY: a euser static with no arguments (`_ZN12CTrapCleanup3NewEv`). It
        // allocates one object on this thread's heap, installs it as the thread's trap
        // handler, and returns null rather than leaving if it cannot.
        let cleanup = unsafe { CTrapCleanup_New() };
        (!cleanup.is_null()).then_some(TrapCleanup(cleanup))
    }
}

impl Drop for TrapCleanup {
    fn drop(&mut self) {
        // SAFETY: the shim runs `delete` through `~CTrapCleanup()`'s vtable slot on the
        // object this value owns and hands out nowhere. It is null-safe, it cannot
        // leave, and it uninstalls the handler before freeing it — which is why this
        // happens after all user code on the thread, and not before.
        unsafe { symrs_cleanup_destroy(self.0) };
    }
}
