//! `std::sync`'s shape on Symbian OS 9.3 (design spec §6a, §11 step 72).
//!
//! ```ignore
//! use symbian_std::sync::{Arc, Mutex};
//!
//! let counter = Arc::new(Mutex::new(0u32));
//! *counter.lock()? += 1;
//! ```
//!
//! # What this costs, and when not to use it
//!
//! ARMv5TE has no `LDREX`/`STREX`, so there is no lock-free atomic on this CPU at all:
//! **every** `core::sync::atomic` operation is a call into the SDK's compiler-runtime
//! archive, which takes one process-wide `RFastLock` — about 90× a plain increment in
//! the emulator (experiment 72). `Arc::clone` and every `Arc` drop pay it, and so does
//! every `Once::call_once`, including one that has already completed.
//!
//! An application on this phone is single-threaded unless it creates a thread, and for
//! single-threaded sharing `Rc` and `RefCell` are the right tools and cost nothing.
//! What is here is for when a program really does have two threads.
//!
//! # What is not here
//!
//! - **No poisoning.** `panic = "abort"` means a panic ends the process, so no thread
//!   can observe data another thread left half-written. [`Mutex`] says the same thing
//!   in its own documentation, and `PoisonError` is absent rather than faked.
//! - **`RwLock`, `Condvar`, `mpsc`, `OnceLock`, `LazyLock`, `Barrier`** are not
//!   implemented. `RCondVar` in particular is doubtful: `CreateLocal()` returns
//!   `KErrNone` but leaves `Handle()` at 0 in EKA2L1, unlike every other handle
//!   (experiment 72), so nothing that needs it can be built honestly here yet.
//! - **`AtomicU64`** does not exist: the target declares `max-atomic-width: 32`.

mod mutex;
mod once;

pub use alloc::sync::{Arc, Weak};
pub use mutex::{Mutex, MutexGuard};
pub use once::Once;

/// `std::sync::atomic`, which on this target is the SDK's `__atomic_*` archive.
pub mod atomic {
    pub use core::sync::atomic::*;
}

/// What the lock behind every atomic operation recorded when it was created: `1` once
/// it exists, `0` if this program has not performed an atomic operation yet, or the
/// `RFastLock::CreateLocal` error.
///
/// `std` has no equivalent and needs none — on every platform `std` supports, an
/// atomic is an instruction. Here it is a kernel object created on first use, so a
/// program that depends on its atomics being atomic can check, on the machine it is
/// running on, that the object is really there.
pub fn atomic_lock_status() -> i32 {
    // SAFETY: an entry point of the SDK's own compiler-runtime archive that takes no
    // argument and only reads one `TInt` of that crate's statics.
    unsafe { symbian_sys::libcalls::symrs_atomic_init_status() }
}

/// The atomic lock's kernel handle, `0` if it has not been created.
pub fn atomic_lock_handle() -> i32 {
    // SAFETY: as [`atomic_lock_status`].
    unsafe { symbian_sys::libcalls::symrs_atomic_lock_handle() }
}
