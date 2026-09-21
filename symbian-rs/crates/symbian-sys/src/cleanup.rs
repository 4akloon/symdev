//! `CTrapCleanup`: the per-thread trap handler and cleanup stack, and the value that
//! owns one.
//!
//! # Why every entry point installs one
//!
//! Symbian's cleanup stack is **per thread** and it does not exist until somebody
//! calls `CTrapCleanup::New()`. A thread without one panics `E32USER-CBase 69`
//! (`EClnNoTrapHandlerInstalled`, `e32panic.h`) the moment any code below reaches
//! `CleanupStack::PushL` — including inside an SDK call's own `TRAP` harness, where
//! the application never sees the call at all. A panic is not a leave, so nothing
//! catches it and the thread simply dies.
//!
//! That is measured, not feared: a `no_std` console application calling `RFs::GetDir`
//! died with exactly that panic line in the emulator log
//! (`Thread Main panicked with category: E32USER-CBase and exit code: 69`) until
//! `symbian_runtime::entry!` installed one.
//!
//! So a Symbian `E32Main` conventionally opens with `CTrapCleanup::New()`, and both
//! Rust entry points do the same: `std`'s `start` and the `no_std` `entry!`. This is
//! the C++ convention followed exactly, not a Rust addition — the C++ SDK's own
//! application skeletons pay for the same object.
//!
//! # Why the type lives in this crate
//!
//! It is the one owning value `symbian-sys` holds, and it is here because it is the
//! only crate the `std` overlay depends on. Writing it twice — once for `std` and once
//! for `no_std` — is what this file exists to prevent, and the two paths having
//! different cleanup behaviour is exactly the asymmetry it was written to remove.

use crate::shim::symrs_cleanup_destroy;

/// The opaque `CTrapCleanup` a thread's trap handler and cleanup stack live in; only
/// ever seen behind a pointer, because it is a `CBase` with a `CCleanup` inside it.
#[repr(C)]
pub struct CTrapCleanup {
    _private: [u8; 0],
}

unsafe extern "C" {
    /// `0000030c T _ZN12CTrapCleanup3NewEv` — `CTrapCleanup::New()`, euser.dso: a
    /// static that allocates the calling thread's trap handler and cleanup stack and
    /// installs them. Non-leaving: it returns null on a full heap. Destroying it goes
    /// through the C++ shim, because `~CTrapCleanup` is virtual.
    #[link_name = "_ZN12CTrapCleanup3NewEv"]
    pub fn CTrapCleanup_New() -> *mut CTrapCleanup;
}

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
