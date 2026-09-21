//! The per-thread table that turns Symbian's one kernel call into any number of
//! thread-locals, and the thread-exit sweep that drops them.
//!
//! # Why a table, when the platform has slots to spare
//!
//! `UserSvr::DllSetTls` keys a slot by a `TInt` the caller chooses, and experiment 88
//! held **64 of them at once on one thread** with no failure, so one slot per
//! `thread_local!` would work. This crate uses exactly **one** slot for the whole
//! program anyway, for two reasons that a slot each cannot give:
//!
//! 1. **Destructors.** `Drop` at thread exit needs the list of live values, and the
//!    kernel will not enumerate a thread's slots — `DllTls` only answers a handle you
//!    already know. One slot holding our own list is the only place that list can be.
//! 2. **The handle is unobserved on hardware.** No device has ever run this, and
//!    `aHandle` is the argument `Dll::Tls` would have filled in with the calling DLL's
//!    code segment handle. Taking exactly one handle for the whole SDK means one
//!    assumption to retest on a phone rather than one per key.
//!
//! The cost is a linear scan of the table on each access, beside the one kernel call
//! that is unavoidable. Measured in `examples/tls`.

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::ffi::c_void;

use symbian_sys::tls::{
    SYMBIAN_STD_TLS_HANDLE, UserSvr_DllFreeTls, UserSvr_DllSetTls, UserSvr_DllTls,
};

/// How a value is dropped once its type has been erased.
pub(super) type Dropper = unsafe fn(*mut u8);

/// Why an access could not reach its value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Cause {
    /// `UserSvr::DllSetTls` refused, which on this platform means the kernel could not
    /// grow the calling thread's slot table.
    Kernel,
    /// The key's own initialiser asked for the key.
    Reentrant,
    /// This thread is running its thread-local destructors, or has finished.
    Destroying,
}

/// One live thread-local on this thread. `value` is null only between the moment the
/// entry is reserved and the moment the initialiser returns.
struct Entry {
    key: usize,
    value: *mut u8,
    drop: Dropper,
}

/// Everything one thread holds. It lives in a `Box` on the one process heap — every
/// thread shares it, because [`crate::thread::spawn`] switches a worker onto the
/// creator's allocator before it runs a line — so whichever thread frees it is
/// returning the block to the heap it came from.
struct Table {
    entries: Vec<Entry>,
    destroying: bool,
}

/// This thread's table, or null.
fn current() -> *mut Table {
    // SAFETY: a euser static taking one scalar. It returns what this thread last
    // stored under the handle, which is either null or the `Box` leaked by `create`.
    unsafe { UserSvr_DllTls(SYMBIAN_STD_TLS_HANDLE).cast::<Table>() }
}

/// This thread's table, creating it on first use.
fn current_or_create() -> Result<*mut Table, Cause> {
    let existing = current();
    if !existing.is_null() {
        return Ok(existing);
    }
    let table = Box::into_raw(Box::new(Table {
        entries: Vec::new(),
        destroying: false,
    }));
    // SAFETY: a euser static taking two scalars; it stores the pointer for this thread
    // and never follows it.
    let code = unsafe { UserSvr_DllSetTls(SYMBIAN_STD_TLS_HANDLE, table.cast::<c_void>()) };
    if code != 0 {
        // SAFETY: the kernel did not take the pointer, so this is still the only
        // reference to a box this function made.
        unsafe { drop(Box::from_raw(table)) };
        return Err(Cause::Kernel);
    }
    Ok(table)
}

/// What this thread has stored for `key`, if anything.
///
/// `Ok(None)` means the key has no entry yet and the caller should initialise it.
pub(super) fn get(key: usize) -> Result<Option<*mut u8>, Cause> {
    let table = current();
    if table.is_null() {
        return Ok(None);
    }
    // SAFETY: the pointer came out of this thread's own slot, where only `create` puts
    // one, and only this thread reads or writes the table. The borrow ends before this
    // function returns, so nothing the caller does can alias it.
    let table = unsafe { &*table };
    if table.destroying {
        return Err(Cause::Destroying);
    }
    match table.entries.iter().find(|e| e.key == key) {
        None => Ok(None),
        Some(entry) if entry.value.is_null() => Err(Cause::Reentrant),
        Some(entry) => Ok(Some(entry.value)),
    }
}

/// Records that `key` is being initialised, so that an initialiser which asks for its
/// own key is an error rather than a second value or an endless recursion.
pub(super) fn reserve(key: usize, drop: Dropper) -> Result<(), Cause> {
    let table = current_or_create()?;
    // SAFETY: as `get`. The borrow ends at the end of this function, before any
    // initialiser runs.
    let table = unsafe { &mut *table };
    if table.destroying {
        return Err(Cause::Destroying);
    }
    table.entries.push(Entry {
        key,
        value: core::ptr::null_mut(),
        drop,
    });
    Ok(())
}

/// Publishes the value an initialiser produced, or withdraws the reservation when
/// `value` is null because it could not produce one.
pub(super) fn publish(key: usize, value: *mut u8) {
    let table = current();
    if table.is_null() {
        return;
    }
    // SAFETY: as `get`; the borrow does not outlive this function.
    let table = unsafe { &mut *table };
    let Some(index) = table.entries.iter().position(|e| e.key == key) else {
        return;
    };
    if value.is_null() {
        table.entries.remove(index);
    } else {
        table.entries[index].value = value;
    }
}

/// Drops every thread-local this thread initialised, newest first, and gives the slot
/// back to the kernel.
///
/// Called at the end of every thread [`crate::thread::spawn`] creates. It is
/// idempotent and costs one kernel call on a thread that has no thread-local.
pub(super) fn destroy() {
    let table = current();
    if table.is_null() {
        return;
    }
    // SAFETY: as `get`. Each step below takes the borrow, ends it, and only then runs
    // a `Drop` that may itself reach back into this table.
    unsafe { (*table).destroying = true };
    loop {
        // SAFETY: as above; the borrow ends with the statement.
        let Some(entry) = (unsafe { (*table).entries.pop() }) else {
            break;
        };
        if !entry.value.is_null() {
            // SAFETY: `value` is the `Box<T>` `LocalKey::try_with` leaked for this
            // entry, `drop` is the dropper written for that same `T`, and the entry has
            // been taken out of the table, so nothing can reach the value again.
            unsafe { (entry.drop)(entry.value) };
        }
    }
    // SAFETY: the table is empty and out of reach: `destroying` makes every further
    // access an error, so this is the last reference to the box `create` leaked.
    unsafe { drop(Box::from_raw(table)) };
    // SAFETY: a euser static taking one scalar; it forgets this thread's slot.
    unsafe { UserSvr_DllFreeTls(SYMBIAN_STD_TLS_HANDLE) };
}

/// How many thread-locals this thread is holding — for a test that wants to see the
/// sweep happen, and for nothing else.
pub(super) fn live() -> usize {
    let table = current();
    if table.is_null() {
        return 0;
    }
    // SAFETY: as `get`.
    unsafe { (*table).entries.len() }
}
