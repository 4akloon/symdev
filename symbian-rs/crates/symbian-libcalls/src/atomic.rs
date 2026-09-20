//! The `__atomic_*` entry points LLVM emits on ARMv5TE, over one `RFastLock`.
//!
//! # Why they have to exist
//!
//! ARMv5TE has no `LDREX`/`STREX`, so no compiler can generate a lock-free atomic
//! read-modify-write inline, and Symbian 9.3 does not fill the gap: euser exports
//! exactly four atomics (`User::LockedInc`/`LockedDec`/`SafeInc`/`SafeDec`, `TInt` ±1,
//! old value returned) and there is no `e32atomics.h` — that family is 9.4 and later.
//! LLVM therefore lowers **every** atomic operation on this target to a libcall,
//! including a relaxed load, and nothing on the recorded link line defines one of them
//! (`nm` over `libgcc.a`, `libsupc++.a`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`,
//! `drtaeabi.dso`, `scppnwdl.dso`, `drtrvct2_2.dso`: zero hits). Experiment 72 proved
//! it by linking; the target's `max-atomic-width: 32` and `atomic-cas: true` are only
//! correct because this module is on the line.
//!
//! # The exact set, and why it is exactly this set
//!
//! Not a guess at libatomic's surface: it is what `nm -u` printed for the static
//! library of a probe touching every operation `core::sync::atomic` offers at every
//! width the target allows — 10 operations × widths 1, 2 and 4, plus
//! `__sync_synchronize`, 31 symbols. Width 8 never appears, because at
//! `max-atomic-width: 32` `core` has no `AtomicU64` to lower. `fetch_max`, `fetch_min`
//! and `compare_exchange_weak` emit no libcall of their own: they lower to a
//! `__atomic_compare_exchange_N` loop over the entry point below.
//!
//! # Why one lock is enough, and why nothing here may be atomic
//!
//! Every entry point takes the same lock for its whole operation, so all atomic
//! operations in the process are totally ordered by the lock's acquire order. That is
//! sequential consistency, which is at least as strong as any ordering a caller can
//! ask for, so the `order` arguments are accepted and ignored.
//!
//! The bodies use [`read_volatile`](core::ptr::read_volatile) and
//! [`write_volatile`](core::ptr::write_volatile) and nothing else. **An atomic
//! operation inside one of these functions would call the function again, for ever**,
//! and `RFastLock` is not recursive, so the second `Wait` would block instead
//! (observed, experiment 72). The build checks this: `symdev` is not what catches it,
//! `nm`/`objdump` over the archive is, and the check is written down in the backlog
//! entry.
//!
//! A load could in principle skip the lock, since a naturally aligned load cannot tear
//! and GCC itself lowers a relaxed C++ load to a bare `ldr` here. A store could not: a
//! plain store racing a locked read-modify-write can be overwritten by the RMW's
//! write-back and lost, which real atomics never allow. Keeping the lock on both is
//! uniform and obviously correct; dropping it from loads is a measured optimisation for
//! a later slice, not something to guess at now.
//!
//! # Cost
//!
//! Every operation is a kernel `Wait`/`Signal` pair — about 90× a plain increment in
//! the emulator (experiment 72, ratios only, not device timing). Every `Arc::clone` and
//! every `Arc` drop pays it. `Rc` and `RefCell` stay the right tools for single-threaded
//! code; this module is what makes the threaded case *correct*, not fast.

use crate::lock::AtomicLock;

/// One width's worth of entry points.
///
/// `$w` is the byte width in the symbol name and `$t` the unsigned integer of that
/// width. The ABI is the one experiment 72 read off the code GCC and LLVM emit, and the
/// two agree. `compare_exchange` takes **five** arguments: the builtin's `weak` flag is
/// not passed to the library routine.
macro_rules! atomic_width {
    ($w:literal, $t:ty, $load:ident, $store:ident, $xchg:ident, $cas:ident,
     $add:ident, $sub:ident, $and:ident, $or:ident, $xor:ident, $nand:ident) => {
        /// `__atomic_load_N(const void*, int) -> T`.
        ///
        /// # Safety
        /// `ptr` is a live, naturally aligned `$t` — the caller is compiler-generated
        /// code for a `core::sync::atomic` type, whose alignment equals its size.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $load(ptr: *const $t, _order: i32) -> $t {
            let _guard = AtomicLock::acquire();
            // SAFETY: as the contract above; volatile so that nothing here becomes an
            // atomic operation and calls this function again.
            unsafe { core::ptr::read_volatile(ptr) }
        }

        /// `__atomic_store_N(void*, T, int)`.
        ///
        /// # Safety
        /// As [`$load`].
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $store(ptr: *mut $t, value: $t, _order: i32) {
            let _guard = AtomicLock::acquire();
            // SAFETY: as the contract above.
            unsafe { core::ptr::write_volatile(ptr, value) };
        }

        atomic_rmw!($w, $t, $xchg, |_old, value| value);
        atomic_rmw!($w, $t, $add, |old: $t, value: $t| old.wrapping_add(value));
        atomic_rmw!($w, $t, $sub, |old: $t, value: $t| old.wrapping_sub(value));
        atomic_rmw!($w, $t, $and, |old: $t, value: $t| old & value);
        atomic_rmw!($w, $t, $or, |old: $t, value: $t| old | value);
        atomic_rmw!($w, $t, $xor, |old: $t, value: $t| old ^ value);
        atomic_rmw!($w, $t, $nand, |old: $t, value: $t| !(old & value));

        /// `__atomic_compare_exchange_N(void*, void* expected, T desired, int, int)
        /// -> bool`.
        ///
        /// On failure the caller's `expected` is overwritten with what was there, which
        /// is what a compare-exchange loop in `core` relies on.
        ///
        /// # Safety
        /// `ptr` and `expected` are live, naturally aligned `$t`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $cas(
            ptr: *mut $t,
            expected: *mut $t,
            desired: $t,
            _success: i32,
            _failure: i32,
        ) -> bool {
            let _guard = AtomicLock::acquire();
            // SAFETY: as the contract above.
            unsafe {
                let old = core::ptr::read_volatile(ptr);
                if old == core::ptr::read_volatile(expected) {
                    core::ptr::write_volatile(ptr, desired);
                    return true;
                }
                core::ptr::write_volatile(expected, old);
            }
            false
        }
    };
}

/// One read-modify-write entry point: `(ptr, value, order) -> old`.
macro_rules! atomic_rmw {
    ($w:literal, $t:ty, $name:ident, $combine:expr) => {
        /// `__atomic_<op>_N(void*, T, int) -> T`, returning the value from **before**
        /// the operation.
        ///
        /// # Safety
        /// `ptr` is a live, naturally aligned `$t`.
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(ptr: *mut $t, value: $t, _order: i32) -> $t {
            let _guard = AtomicLock::acquire();
            // SAFETY: as the contract above; volatile so that nothing here becomes an
            // atomic operation and calls this function again.
            unsafe {
                let old = core::ptr::read_volatile(ptr);
                let combine: fn($t, $t) -> $t = $combine;
                core::ptr::write_volatile(ptr, combine(old, value));
                old
            }
        }
    };
}

atomic_width!(
    1,
    u8,
    __atomic_load_1,
    __atomic_store_1,
    __atomic_exchange_1,
    __atomic_compare_exchange_1,
    __atomic_fetch_add_1,
    __atomic_fetch_sub_1,
    __atomic_fetch_and_1,
    __atomic_fetch_or_1,
    __atomic_fetch_xor_1,
    __atomic_fetch_nand_1
);

atomic_width!(
    2,
    u16,
    __atomic_load_2,
    __atomic_store_2,
    __atomic_exchange_2,
    __atomic_compare_exchange_2,
    __atomic_fetch_add_2,
    __atomic_fetch_sub_2,
    __atomic_fetch_and_2,
    __atomic_fetch_or_2,
    __atomic_fetch_xor_2,
    __atomic_fetch_nand_2
);

atomic_width!(
    4,
    u32,
    __atomic_load_4,
    __atomic_store_4,
    __atomic_exchange_4,
    __atomic_compare_exchange_4,
    __atomic_fetch_add_4,
    __atomic_fetch_sub_4,
    __atomic_fetch_and_4,
    __atomic_fetch_or_4,
    __atomic_fetch_xor_4,
    __atomic_fetch_nand_4
);

/// `__sync_synchronize()`: a full barrier, which on this device is a compiler barrier
/// and nothing else.
///
/// ARMv5TE has no `DMB` — the instruction arrives with ARMv6K — so there is no barrier
/// instruction to emit, and one core cannot reorder its own accesses in a way another
/// observer could see. Stopping the *compiler* from moving accesses across the call is
/// therefore the whole of what this can do here.
///
/// UNKNOWN, and recorded as such in `docs/research/eka2-concurrency.md`: whether the OS
/// can ever run two threads of one process on two cores on any device this SDK targets.
/// If one exists, this is not enough for it, and nothing on this host can say.
///
/// It is an empty `asm!` and **not** `core::sync::atomic::compiler_fence`, which is the
/// obvious spelling and is wrong here: on this target LLVM lowers even a
/// single-threaded `fence` to a libcall, so `compiler_fence` inside this function
/// compiled to `bl __sync_synchronize` — a call to itself, for ever. Observed in the
/// object (`objdump -dr`), which is why that check is part of the build's evidence.
///
/// An empty `asm!` with neither `nomem` nor `readonly` is assumed to read and write
/// memory, which is exactly the compiler barrier wanted, and emits no instruction.
///
/// # Safety
/// Takes nothing and touches nothing; `unsafe` only because the caller is the compiler.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn __sync_synchronize() {
    // SAFETY: an empty template emits no instruction; the options say only that it may
    // touch memory, which is the whole point, and that it disturbs neither the stack
    // nor the flags.
    unsafe { core::arch::asm!("", options(nostack, preserves_flags)) };
}
