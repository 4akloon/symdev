//! `AtomicLock`: the one process-wide lock every `__atomic_*` entry point takes, and
//! the bootstrap that creates it with no atomic available to do it with.

use symbian_sys::euser::{User_After, User_LockedInc, User_Panic};
use symbian_sys::thread::{
    EOWNER_PROCESS, RFastLock, RFastLock_CreateLocal, RFastLock_Signal, RFastLock_Wait,
};

use crate::cell::Shared;

/// `symrs-atomic`, the panic category, as UTF-16 with euser's descriptor header in
/// front. The header word is `(length << 4) | type`, type 0 = `EBufC` (observed on
/// this ROM's euser in experiment 69).
static PANIC_CATEGORY: DescriptorLiteral<12> = DescriptorLiteral::new([
    b's' as u16,
    b'y' as u16,
    b'm' as u16,
    b'r' as u16,
    b's' as u16,
    b'-' as u16,
    b'a' as u16,
    b't' as u16,
    b'o' as u16,
    b'm' as u16,
    b'i' as u16,
    b'c' as u16,
]);

/// A constant `TBufC16<N>`: the descriptor header word followed by the text, which is
/// what `User::Panic` wants and what `_LIT` builds in C++.
#[repr(C)]
struct DescriptorLiteral<const N: usize> {
    header: u32,
    text: [u16; N],
}

impl<const N: usize> DescriptorLiteral<N> {
    const fn new(text: [u16; N]) -> Self {
        Self {
            header: (N as u32) << 4,
            text,
        }
    }
}

// SAFETY: the literal is immutable, holds no interior mutability and no pointer; it is
// shared read-only with euser for the duration of one call.
unsafe impl<const N: usize> Sync for DescriptorLiteral<N> {}

/// The claim counter the bootstrap races on. `User::LockedInc` returns the **old**
/// value, so exactly one caller ever sees 0 and that caller creates the lock.
static CLAIM: Shared<i32> = Shared::new(0);

/// The lock itself. All zero is exactly what `RFastLock`'s own inline constructor
/// (`e32cmn.inl` line 3158, `iCount(0)` over `iHandle(0)`) would build.
static LOCK: Shared<RFastLock> = Shared::new(RFastLock::null());

/// Written only by the thread that won the claim, and only after `LOCK` is usable.
/// 0 = not yet, 1 = ready, negative = `CreateLocal` failed with that code.
static READY: Shared<i32> = Shared::new(0);

/// Held for the whole of one atomic operation.
pub struct AtomicLock;

impl AtomicLock {
    /// Takes the lock, creating it on the first call.
    pub fn acquire() -> Self {
        if ready_flag() != 1 {
            bootstrap();
        }
        // SAFETY: `bootstrap` returned, so `LOCK` holds a created `RFastLock` and is
        // never written again for the life of the process; `RFastLock::Wait` is a
        // non-leaving euser member with `this` as argument 0 (experiment 78).
        unsafe { RFastLock_Wait(LOCK.get()) };
        Self
    }
}

impl Drop for AtomicLock {
    fn drop(&mut self) {
        // SAFETY: this guard exists only after a successful `Wait` on the same lock.
        unsafe { RFastLock_Signal(LOCK.get()) };
    }
}

/// `READY`, read so that the compiler cannot cache it across the spin below.
fn ready_flag() -> i32 {
    // SAFETY: `READY` is a plain `TInt`; a naturally aligned 32-bit load cannot tear on
    // ARMv5TE, and a volatile read is what stops the loop in `bootstrap` from being
    // hoisted. It must NOT be an `AtomicI32`: an atomic here would call back into the
    // entry points this lock exists to serve, for ever.
    unsafe { core::ptr::read_volatile(READY.get()) }
}

/// Creates the lock exactly once, using the one atomic the platform really has.
///
/// `User::LockedInc` is a euser export that needs no lock and no initialisation of its
/// own, returns the old value, and was observed to be genuinely atomic against a second
/// thread (experiment 72). So the first caller to see 0 owns the creation and everybody
/// else waits for `READY`. There is no double-checked-locking hazard to reason about,
/// because there is no lock involved in deciding who creates the lock.
///
/// In practice this runs on the process's only thread — nothing can have spawned a
/// thread without first performing an atomic operation — but it is correct even if it
/// did not, which is why it is written this way rather than as a plain first-use check.
#[cold]
fn bootstrap() {
    // SAFETY: `CLAIM` is a plain `TInt` that only this function touches, and
    // `User::LockedInc` is a euser static member function taking a `TInt&`.
    let claim = unsafe { User_LockedInc(CLAIM.get()) };
    if claim == 0 {
        // SAFETY: we are the one caller that won the claim, so nobody else writes
        // `LOCK`; `CreateLocal` is a non-leaving member with `this` as argument 0.
        let rc = unsafe { RFastLock_CreateLocal(LOCK.get(), EOWNER_PROCESS) };
        // SAFETY: a plain `TInt` store, published after the lock is usable.
        unsafe { core::ptr::write_volatile(READY.get(), if rc == 0 { 1 } else { rc }) };
        if rc != 0 {
            fail(rc);
        }
        return;
    }
    loop {
        match ready_flag() {
            1 => return,
            0 => symbian_after_zero(),
            rc => fail(rc),
        }
    }
}

/// Yields to whichever thread is creating the lock. `User::After(0)` is the
/// reschedule point experiment 72 used to make the emulator interleave two threads.
fn symbian_after_zero() {
    // SAFETY: a euser static member that blocks this thread only and cannot leave.
    unsafe { User_After(0) };
}

/// Ends the process rather than pretending an operation was atomic.
///
/// There is no error to return: the caller is compiler-generated code implementing
/// `AtomicU32::fetch_add`, which has no failure path. Returning a value that is quietly
/// not atomic would be a bug nobody could find, so this says which file failed and
/// stops. `reason` is the `RFastLock::CreateLocal` error.
#[cold]
fn fail(reason: i32) -> ! {
    // SAFETY: `User::Panic` is a euser static member that never returns; the category
    // is a constant descriptor with the observed header layout, borrowed for the call.
    unsafe { User_Panic((&raw const PANIC_CATEGORY).cast(), reason) }
}

/// What the bootstrap recorded: `1` once the lock exists, `0` if no atomic operation
/// has been performed yet, or the `RFastLock::CreateLocal` error.
///
/// `extern "C"` and unmangled because this crate is linked as its own archive rather
/// than named as a dependency, so an application reaches it the same way it reaches
/// the C++ shim: through a declaration in `symbian-sys`. It is how a program checks on
/// the machine in hand that its atomics really are atomic.
///
/// # Safety
/// Reads two `TInt`s of this crate's own statics and nothing else.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symrs_atomic_init_status() -> i32 {
    ready_flag()
}

/// The lock's kernel handle, `0` if it has not been created.
///
/// # Safety
/// As [`symrs_atomic_init_status`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn symrs_atomic_lock_handle() -> i32 {
    // SAFETY: reading one `TInt` of a static that is written once, before publication.
    unsafe { (*LOCK.get()).base.handle }
}
