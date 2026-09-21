//! Key-based thread-local storage over `UserSvr::DllTls` (experiment 88).
//!
//! # The platform
//!
//! There is no `Dll` class in this SDK. The whole surface is five `@internalAll`
//! `UserSvr` statics in `e32svr.h`, and `aHandle` — the argument `Dll::Tls` would have
//! filled in with the calling DLL's code segment handle — is **ours to choose** in an
//! EXE. Experiment 88 measured what that buys: an EXE gets *many* slots, keyed by a
//! `TInt` of its own choosing, they are **per thread**, 64 were held at once with no
//! failure, and one access costs 100 ns of which the kernel call is 53. A slot is
//! therefore cheap enough to spend one per `thread_local!`, which is exactly what
//! `sys::thread_local::os` expects of a `key`, so `os.rs` is used unchanged.
//!
//! # What the kernel will not do
//!
//! It will not enumerate a thread's slots, and it will not run a destructor. `DllTls`
//! only answers a handle you already know, and nothing is called when a thread ends.
//!
//! Both are solved here by owning the key space. Keys are handed out from one counter,
//! so the set of live keys *is* `[BASE, BASE + created)`, and [`run_dtors`] walks it.
//! `sys::thread::drop_thread_locals` calls that at the end of every thread, including
//! the main one, where `lang_start` calls it after `main` returns; the
//! `thread_local::guard` for this platform therefore does nothing.

use crate::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release};
use crate::sync::atomic::{Atomic, AtomicPtr, AtomicUsize};

/// The `TInt` handle a key is. It is not an index: it is what goes to the kernel.
pub type Key = i32;

/// Where this program's key space starts.
///
/// The value only has to be stable and unlikely to collide with a real DLL's code
/// segment handle, which is a kernel handle and therefore small. It is deliberately
/// **not** `symbian_sys::tls::SYMBIAN_STD_TLS_HANDLE`, the one slot the `no_std` SDK
/// takes for its own table: a program may link both and they must not share a slot.
const BASE: Key = 0x7374_6400; // "std\0"

/// How many keys this program may create.
///
/// The kernel grows a thread's slot table on demand and experiment 88 saw no ceiling at
/// 64, so the limit here is this module's own destructor table and not the platform's.
/// It is a fixed array because a key may be created while the allocator is busy.
const MAX_KEYS: usize = 128;

type Dtor = unsafe extern "C" fn(*mut u8);

/// How many keys have been created. The live key space is `BASE ..< BASE + CREATED`.
static CREATED: Atomic<usize> = AtomicUsize::new(0);

/// The destructor of each key, by index. Null means the key has none, or was destroyed.
static DTORS: [Atomic<*mut ()>; MAX_KEYS] = [const { AtomicPtr::new(crate::ptr::null_mut()) }; MAX_KEYS];

#[cold]
fn out_of_keys() -> ! {
    rtabort!("out of TLS keys");
}

#[cold]
fn set_failed(code: i32) -> ! {
    // `UserSvr::DllSetTls` fails when the kernel cannot grow this thread's slot table,
    // which is `KErrNoMemory`. There is nowhere to report it to and nothing to unwind.
    rtabort!("UserSvr::DllSetTls failed ({code})");
}

pub fn create(dtor: Option<Dtor>) -> Key {
    let index = CREATED.fetch_add(1, AcqRel);
    if index >= MAX_KEYS {
        out_of_keys();
    }
    if let Some(dtor) = dtor {
        DTORS[index].store(dtor as *mut (), Release);
    }
    BASE.wrapping_add(index as Key)
}

/// # Safety
/// `key` must come from [`create`] and must not have been [`destroy`]ed.
#[inline]
pub unsafe fn set(key: Key, value: *mut u8) {
    // SAFETY: a `UserSvr` static taking two scalars. It stores the pointer against this
    // thread's `key` slot and never follows it.
    let code = unsafe { symbian_sys::tls::UserSvr_DllSetTls(key, value.cast()) };
    if code != 0 {
        set_failed(code);
    }
}

/// # Safety
/// `key` must come from [`create`] and must not have been [`destroy`]ed.
#[inline]
pub unsafe fn get(key: Key) -> *mut u8 {
    // SAFETY: a `UserSvr` static taking one scalar; it returns what this thread last
    // stored under `key`, or null.
    unsafe { symbian_sys::tls::UserSvr_DllTls(key).cast() }
}

/// # Safety
/// `key` must come from [`create`].
#[inline]
pub unsafe fn destroy(key: Key) {
    if let Some(index) = usize::try_from(key.wrapping_sub(BASE)).ok().filter(|i| *i < MAX_KEYS) {
        DTORS[index].store(crate::ptr::null_mut(), Release);
    }
    // SAFETY: a `UserSvr` static taking one scalar; it forgets this thread's slot and
    // never touches what the pointer referred to.
    unsafe { symbian_sys::tls::UserSvr_DllFreeTls(key) };
}

/// How many rounds of destruction to run before giving up.
///
/// A destructor may set a thread-local of its own, which is why one pass is not enough;
/// POSIX makes the same allowance with `PTHREAD_DESTRUCTOR_ITERATIONS`, whose minimum
/// is 4. Anything still set after this many rounds is leaked, as it is there.
const ROUNDS: usize = 5;

/// Runs every thread-local destructor this thread owes, newest key space first.
///
/// Nothing in the kernel does this, so `sys::thread` calls it explicitly at the end of
/// every thread — including the main one, from `lang_start`. It costs one kernel call
/// per created key and is safe to call on a thread that has no thread-local at all.
///
/// # Safety
/// The calling thread must be finishing: after this returns, a value stored under any
/// of these keys has been dropped and the slot cleared.
pub(crate) unsafe fn run_dtors() {
    for _ in 0..ROUNDS {
        let mut ran = false;
        for index in (0..CREATED.load(Acquire).min(MAX_KEYS)).rev() {
            let dtor = DTORS[index].load(Relaxed);
            if dtor.is_null() {
                continue;
            }
            let key = BASE.wrapping_add(index as Key);
            // SAFETY: `key` is in the created range, so it is a valid key.
            let value = unsafe { get(key) };
            if value.is_null() {
                continue;
            }
            // The slot is cleared before the destructor runs, exactly as POSIX does it,
            // so a destructor that reads its own key sees nothing rather than a value
            // that is being dropped.
            // SAFETY: as above.
            unsafe { set(key, crate::ptr::null_mut()) };
            // SAFETY: `dtor` was registered by `create` for this key together with the
            // type whose `Box` `value` is, and the slot no longer refers to it.
            unsafe { crate::mem::transmute::<*mut (), Dtor>(dtor)(value) };
            ran = true;
        }
        if !ran {
            return;
        }
    }
}
