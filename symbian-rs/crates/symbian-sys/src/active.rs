//! The active object and its scheduler: the C++ shim's `symrs_active_*` entry points
//! (`shims/common/symrs_active.cpp`, design spec §11 step 73), and the one euser export
//! that needs no shim.
//!
//! `CActive` is the one case rule 3 of `symrs_shim.h` describes: `RunL` and `DoCancel`
//! are pure virtual and the constructor and `SetActive()` are protected, so a subclass
//! is the only way to have one and a Rust type cannot be a C++ subclass. The virtuals
//! forward to [`SymRsActiveVTable`], a Rust-owned table of function pointers with one
//! opaque context — the shape experiment 76 settled for the Avkon virtuals.
//!
//! `CActiveScheduler::Current()` is the exception: a plain non-leaving static returning
//! a pointer, so it is declared here and called directly. It is also the cheapest
//! identity a thread has in this SDK — the scheduler is per thread — which is what the
//! executor uses to notice that it is being driven from a thread it does not belong to.

use crate::thread::TRequestStatus;

/// The two virtuals `CSymRsActive` forwards, as function pointers. Laid out to match
/// `struct SymRsActiveVTable` in `shims/common/symrs_shim.h`.
///
/// Neither callback may leave or panic: they are called with C++ frames below them and
/// the whole Rust text is one `cantunwind` range. `run` returns a `TInt` and the shim
/// turns a negative one into a leave **after** the Rust frame has returned.
#[repr(C)]
pub struct SymRsActiveVTable {
    /// Called from `RunL` with the completion code the service wrote into `iStatus`.
    pub run: unsafe extern "C" fn(context: *mut core::ffi::c_void, status: i32) -> i32,
    /// Called from `DoCancel`: cancel the service that holds the request. It must
    /// complete the status at once, which is what `RTimer::Cancel` does.
    pub cancel: unsafe extern "C" fn(context: *mut core::ffi::c_void),
}

unsafe extern "C" {
    /// Creates one `CSymRsActive` and adds it to the scheduler installed on this
    /// thread. Null when there is no scheduler, when the table is incomplete, or on a
    /// full heap — never a panic, which is what the shim's own `Current()` check buys.
    pub fn symrs_active_new(
        vtable: *const SymRsActiveVTable,
        context: *mut core::ffi::c_void,
        priority: i32,
    ) -> *mut core::ffi::c_void;

    /// `Cancel()`, dequeue and `delete`. Null-safe. Cancelling first is not optional:
    /// `~CActive` panics (`E32USER-CBase 40`) if the request is still outstanding.
    pub fn symrs_active_destroy(active: *mut core::ffi::c_void);

    /// The object's own `iStatus`, to hand to an asynchronous service.
    pub fn symrs_active_status(active: *mut core::ffi::c_void) -> *mut TRequestStatus;

    /// `CActive::SetActive()`, after the request has been issued.
    pub fn symrs_active_issued(active: *mut core::ffi::c_void);

    /// `CActive::Cancel()`: `DoCancel`, then consume the completion. A no-op when the
    /// request is not outstanding.
    pub fn symrs_active_cancel(active: *mut core::ffi::c_void);

    /// `new CActiveScheduler` + `CActiveScheduler::Install`. `KErrInUse` when one is
    /// already installed — which is what a **GUI** application hits, because CONE
    /// installs `CCoeScheduler` before any application code runs.
    pub fn symrs_scheduler_install(scheduler: *mut *mut core::ffi::c_void) -> i32;

    /// `CActiveScheduler::Install(NULL)` + `delete`. Null-safe.
    pub fn symrs_scheduler_uninstall(scheduler: *mut core::ffi::c_void);

    /// `CActiveScheduler::Start()`, TRAPped: `e32base.h` says nothing either way about
    /// it leaving, which rule 1 counts as leaving, and every `RunL` in the program runs
    /// underneath this call. Returns the leave code, or `KErrNone`.
    pub fn symrs_scheduler_start() -> i32;

    /// `CActiveScheduler::Stop()`. Only legal while `Start()` is running.
    pub fn symrs_scheduler_stop();
}

unsafe extern "C" {
    /// `000006a8 T _ZN16CActiveScheduler7CurrentEv` —
    /// `CActiveScheduler::Current()`, the scheduler installed on the **calling thread**,
    /// or null. A plain static with a pointer return: no leave, no `this`, no sret, so
    /// rule 1 of `symrs_shim.h` does not apply and this needs no wrapper.
    #[link_name = "_ZN16CActiveScheduler7CurrentEv"]
    pub fn CActiveScheduler_Current() -> *mut core::ffi::c_void;
}
