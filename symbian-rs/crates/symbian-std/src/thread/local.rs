//! `thread_local!` and [`LocalKey`], `std`'s shape over the one kernel call Symbian
//! gives a thread (design spec §11 step 76).
//!
//! ```ignore
//! use core::cell::Cell;
//! use symbian_std::thread_local;
//!
//! thread_local! {
//!     static DEPTH: Cell<u32> = const { Cell::new(0) };
//! }
//!
//! DEPTH.with(|d| d.set(d.get() + 1));
//! ```
//!
//! # What this costs, and why
//!
//! There is no ELF thread-local segment on this platform and the target says so
//! (`has-thread-local: false`): E32 has no `PT_TLS` and the post-linker has no notion
//! of one, so nothing here is a `#[thread_local]` static and no compiler support is
//! used. Every access is instead **one kernel call** — `UserSvr::DllTls`, which reads
//! the calling thread's own slot — followed by a walk of a short list. That is cheaper
//! than a single atomic operation on this CPU, which is a `Wait`/`Signal` pair on a
//! process-wide `RFastLock` (experiment 80), so a `thread_local!` is the *cheap* way
//! to keep per-thread state here rather than the expensive one. The measured figure is
//! in `examples/tls` and in experiment 88.
//!
//! # How this differs from `std`
//!
//! - **`with` ends the process instead of unwinding.** `panic = "abort"` makes `std`'s
//!   own `with` do exactly that; here it is `User::Panic` with category `symrs-tls` and
//!   the reason below, so the failure arrives as a Symbian panic a device can report
//!   rather than as a silent exit. [`LocalKey::try_with`] is `std`'s recoverable form
//!   and has `std`'s exact signature; prefer it (CLAUDE.md: a library returns errors).
//! - **Destructors run at the end of every thread [`crate::thread::spawn`] created**,
//!   newest value first. They do **not** run for the main thread: the only hook below
//!   this crate is `symbian_runtime::entry!`, which cannot call into `symbian-std`
//!   without a dependency cycle, and putting an unmangled hook symbol on the link line
//!   would pull this machinery into every program that links the crate (the
//!   measured cost of exactly that mistake is in experiment 80). An application that
//!   needs them on the main thread calls [`crate::thread::drop_thread_locals`] itself;
//!   step 77's `lang_start` is where that call belongs once `std` owns the entry point.
//! - **The value lives on the process heap**, in a `Box` this module leaks into the
//!   table. A worker thread shares the creator's allocator from its first instruction
//!   (see [`crate::thread`]), so a value created on one thread and dropped on another
//!   goes back to the heap it came from — there is no cross-heap free anywhere here.
//! - **No `LocalKey<Cell<T>>`/`LocalKey<RefCell<T>>` conveniences** (`get`, `set`,
//!   `take`, `replace`, `with_borrow*`). They are `std` inherent impls on those exact
//!   types and each would be a separate decision about what the failure means; nothing
//!   needed them yet.

use alloc::boxed::Box;
use core::fmt;

use symbian_core::{Buf16, DesC16};
use symbian_sys::euser::User_Panic;

use super::table::{self, Cause};

/// A thread-local variable, as `std::thread::LocalKey`. Written with
/// [`thread_local!`](crate::thread_local), never by hand.
pub struct LocalKey<T: 'static> {
    init: fn() -> T,
}

impl<T: 'static> LocalKey<T> {
    /// Used by [`thread_local!`](crate::thread_local); not part of the API `std` has.
    #[doc(hidden)]
    pub const fn new(init: fn() -> T) -> Self {
        Self { init }
    }

    /// Runs `f` with this thread's value, initialising it on first use, as
    /// `std::thread::LocalKey::try_with`.
    pub fn try_with<F, R>(&'static self, f: F) -> Result<R, AccessError>
    where
        F: FnOnce(&T) -> R,
    {
        let value = self.value()?;
        // SAFETY: `value` is the `Box<T>` this thread leaked for this key. Only this
        // thread can reach it, the table holds it until this thread's sweep, and the
        // sweep cannot run while this frame is on this thread's stack.
        Ok(f(unsafe { &*value }))
    }

    /// Runs `f` with this thread's value, as `std::thread::LocalKey::with`.
    ///
    /// **Ends the process** if the value cannot be reached, which is what `std`'s own
    /// `with` does under `panic = "abort"`; the category is `symrs-tls` and the reason
    /// is the `e32err.h` code [`AccessError`] carries. Use [`Self::try_with`] to handle
    /// it instead.
    pub fn with<F, R>(&'static self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        match self.try_with(f) {
            Ok(value) => value,
            Err(error) => panic_tls(error.reason()),
        }
    }

    /// This thread's value, initialised if this is the first ask.
    fn value(&'static self) -> Result<*mut T, AccessError> {
        let key = core::ptr::from_ref(self).addr();
        if let Some(value) = table::get(key).map_err(access)? {
            return Ok(value.cast::<T>());
        }
        table::reserve(key, drop_value::<T>).map_err(access)?;
        let value = Box::into_raw(Box::new((self.init)()));
        table::publish(key, value.cast::<u8>());
        Ok(value)
    }
}

/// Drops a value whose type the table has forgotten.
///
/// # Safety
/// `value` must be a `Box<T>` leaked by `LocalKey::<T>::value` and unreachable
/// from anywhere else.
unsafe fn drop_value<T>(value: *mut u8) {
    // SAFETY: the caller guarantees the provenance and that this is the last reference.
    drop(unsafe { Box::from_raw(value.cast::<T>()) });
}

/// What `LocalKey::try_with` reports, as `std::thread::AccessError`.
///
/// `std`'s has one meaning — the value has been destroyed. This one has three, because
/// on this platform storage itself can fail, so the message names which.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AccessError {
    reason: i32,
}

impl AccessError {
    /// The `e32err.h` code this failure reports, so it can become a panic reason or an
    /// [`crate::io::Error`] without losing what happened.
    pub fn reason(self) -> i32 {
        self.reason
    }

    /// Which of the three failures this was, in words: its `Display`.
    pub fn message(self) -> &'static str {
        match self.reason {
            NO_SLOT => "the kernel refused a thread-local slot for this thread",
            REENTRANT => "a thread-local's own initialiser asked for that thread-local",
            _ => "this thread's thread-locals have already been dropped",
        }
    }
}

/// `KErrNoMemory`: the kernel could not grow this thread's slot table.
const NO_SLOT: i32 = -4;
/// `KErrInUse`: the key's own initialiser asked for the key.
const REENTRANT: i32 = -14;
/// `KErrDied`: the thread is past the point where its thread-locals exist.
const DESTROYED: i32 = -13;

/// The one place a table failure becomes the crate's error.
fn access(cause: Cause) -> AccessError {
    AccessError {
        reason: match cause {
            Cause::Kernel => NO_SLOT,
            Cause::Reentrant => REENTRANT,
            Cause::Destroying => DESTROYED,
        },
    }
}

impl fmt::Display for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

impl fmt::Debug for AccessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

/// Ends the process the Symbian way, as [`LocalKey::with`] documents.
#[cold]
fn panic_tls(reason: i32) -> ! {
    let mut category = Buf16::<16>::new();
    // The category is a fixed nine characters into a sixteen-unit buffer, so this
    // cannot overflow; a library here may not `unwrap`, and an empty category is still
    // a panic the device reports.
    let _ = category.push_str("symrs-tls");
    // SAFETY: `User::Panic` is a euser static that never returns; the category is a
    // live descriptor for the duration of the call.
    unsafe { User_Panic(category.as_tdesc16(), reason) }
}

/// Drops every thread-local this thread initialised, newest first, as the end of a
/// thread does.
///
/// [`spawn`]'s threads call it themselves; the **main** thread has no hook below this
/// crate that could (see [`LocalKey`]), so a program that wants its main thread's
/// thread-locals dropped before the process ends calls this as the last thing `main`
/// does. Calling it twice, or on a thread that has no thread-local, costs one kernel
/// call and does nothing.
pub fn drop_thread_locals() {
    table::destroy();
}

/// How many thread-locals this thread is currently holding.
///
/// `std` has no such thing; it is here because "the destructors ran" is otherwise not
/// observable from inside the program, and a leak that nothing can see is exactly what
/// this module must not ship.
pub fn live_thread_locals() -> usize {
    table::live()
}

/// Declares one or more thread-local variables, as `std::thread_local!`.
///
/// Both of `std`'s initialiser forms are accepted, including `const { … }`. The
/// `const` form buys nothing here beyond saying that the initialiser is a constant —
/// the value is still a heap block reached through a kernel call, because this
/// platform has no thread-local *storage* for the compiler to place it in.
///
/// ```ignore
/// thread_local! {
///     static COUNT: Cell<u32> = const { Cell::new(0) };
///     pub static NAME: String = String::from("worker");
/// }
/// ```
#[macro_export]
macro_rules! thread_local {
    () => {};

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block; $($rest:tt)*) => {
        $crate::thread_local!($(#[$attr])* $vis static $name: $t = const $init);
        $crate::thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = const $init:block) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::LocalKey<$t> = {
            fn __symbian_init() -> $t { const $init }
            $crate::thread::LocalKey::new(__symbian_init)
        };
    };

    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => {
        $crate::thread_local!($(#[$attr])* $vis static $name: $t = $init);
        $crate::thread_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => {
        $(#[$attr])*
        $vis static $name: $crate::thread::LocalKey<$t> = {
            fn __symbian_init() -> $t { $init }
            $crate::thread::LocalKey::new(__symbian_init)
        };
    };
}
