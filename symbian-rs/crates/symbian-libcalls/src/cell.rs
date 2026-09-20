//! `Shared`: a `static` this crate may take a raw pointer to.
//!
//! Everything here is below `core::sync::atomic` — this crate is what makes
//! `core::sync::atomic` exist — so none of the usual tools is available: no `Mutex`,
//! no `OnceLock`, not even an `AtomicBool`, because every one of them would call back
//! into the entry points these statics serve. What is left is a `static` holding an
//! `UnsafeCell`, with the reasoning written out at each use.

use core::cell::UnsafeCell;

/// A `static` whose contents this crate mutates through a raw pointer.
pub struct Shared<T>(UnsafeCell<T>);

impl<T> Shared<T> {
    pub const fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    /// A pointer to the contents. Every caller states why its access is sound.
    pub const fn get(&self) -> *mut T {
        self.0.get()
    }
}

// SAFETY: `Shared` gives out a raw pointer and promises nothing about races; each use
// site in this crate carries the argument that its own access is ordered — either by
// `User::LockedInc`, which is atomic on this platform without needing a lock, or by
// happening before any second thread can exist.
unsafe impl<T> Sync for Shared<T> {}
