//! The SDK's compiler-runtime archive (`crates/symbian-libcalls`).
//!
//! That crate defines what rustc's own code generation calls and Symbian 9.3 on
//! ARMv5TE does not provide: the `__atomic_*` family over one process-wide
//! `RFastLock`, `__sync_synchronize`, `memcmp` and `bcmp`. Nothing there is named by
//! an application — those symbols are emitted by the compiler — but the two below are,
//! because a program that relies on its atomics being atomic should be able to say so
//! on the machine it is running on.
//!
//! They are declared here rather than reached through a `use` because the crate is not
//! a dependency: symdev builds it separately and puts it on the link line as its own
//! archive, so that a program which performs no atomic operation carries none of it
//! (as a dependency it cost every program 756 bytes, measured on `hello`).

unsafe extern "C" {
    /// What the atomic lock's bootstrap recorded: `1` once the lock exists, `0` if no
    /// atomic operation has been performed yet, or the `RFastLock::CreateLocal` error.
    pub fn symrs_atomic_init_status() -> i32;

    /// The atomic lock's kernel handle, `0` if there is none.
    pub fn symrs_atomic_lock_handle() -> i32;
}
