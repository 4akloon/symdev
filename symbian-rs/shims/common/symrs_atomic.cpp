// symrs_atomic.cpp -- the atomic libcalls LLVM emits on ARMv5TE, over one process-wide
// RFastLock. The rule for what belongs in a shim is in symrs_shim.h; this file is the
// second documented exception to it, for the same reason as symrs_cstring.cpp: these
// are compiler runtime routines that this platform does not provide anywhere.
//
// ---------------------------------------------------------------------------
// WHY THIS FILE EXISTS
// ---------------------------------------------------------------------------
//
// ARMv5TE has no LDREX/STREX, so no compiler can generate a lock-free atomic
// read-modify-write inline, and Symbian 9.3 does not fill the gap: euser.dso exports
// exactly four atomics (User::LockedInc/LockedDec/SafeInc/SafeDec, TInt +/-1, old value
// returned) and there is no e32atomics.h -- that family is 9.4 and later. LLVM
// therefore lowers *every* atomic operation on this target to a libcall, including a
// relaxed load, and nothing on the recorded link line defines a single one of them
// (nm over libgcc.a, libsupc++.a, usrt2_2.lib, euser.dso, dfpaeabi.dso, drtaeabi.dso,
// scppnwdl.dso and drtrvct2_2.dso: zero hits). Experiment 72 proved that by linking.
//
// The exact set below is not a guess at libatomic's surface: it is what
// `nm -u` printed for the static library of a probe that touches every operation
// core::sync::atomic offers at every width the target allows -- 10 operations x
// widths 1, 2 and 4, plus __sync_synchronize. Width 8 never appears, because the
// target declares max-atomic-width 32 and so core has no AtomicU64 to lower.
// fetch_max, fetch_min and compare_exchange_weak emit no libcall of their own: they
// lower to a __atomic_compare_exchange_N loop over the entry point below.
//
// Each definition needs the __asm__ label: spelling `__atomic_load_4` directly as a
// function name makes GCC report `ambiguates built-in declaration`.
//
// ---------------------------------------------------------------------------
// THE ARGUMENT THAT ONE LOCK IS ENOUGH
// ---------------------------------------------------------------------------
//
// Every entry point below takes the same lock for its whole operation, so all atomic
// operations in the process are totally ordered by the lock's acquire order. That is
// sequential consistency, which is at least as strong as every ordering a caller can
// ask for, so the `memorder` arguments are accepted and ignored.
//
// The lock is NOT recursive (RFastLock: a second Wait from the owning thread blocks
// for ever, observed in experiment 72), so nothing between Wait and Signal may perform
// an atomic operation. Nothing here does: the bodies are plain loads and stores.
//
// A load could in principle skip the lock -- a naturally aligned load cannot tear, and
// GCC itself lowers a relaxed 8/16/32-bit C++ load to a bare `ldr` on this target. A
// store could NOT: a plain store racing a locked read-modify-write can be overwritten
// by the RMW's write-back and lost, which real atomics never allow. Keeping the lock
// on both is uniform and obviously correct; halving the cost of a load is a measured
// optimisation for a later slice, not something to guess at here.
//
// COST, measured in EKA2L1 (experiment 72, ratios only -- not device timing): a shim
// fetch_add is about 90x a plain increment, because it is a kernel Wait/Signal pair.
// Every Arc::clone and Arc::drop pays it. Rc and RefCell remain the right tools for
// single-threaded code; this file is what makes the threaded case *correct*, not fast.
#include "symrs_shim.h"

#include <e32std.h>

// ---------------------------------------------------------------------------
// THE LOCK, AND HOW IT IS INITIALISED WITH NO ATOMIC TO DO IT WITH
// ---------------------------------------------------------------------------
//
// The bootstrap problem is real: the lock that makes every atomic operation atomic
// cannot itself be created under an atomic guard, because creating it is what provides
// the guard. Double-checked locking needs an atomic; a first-use race would create two
// locks and lose updates; and euser's LockedInc, while genuinely atomic, would still
// leave a caller spinning on a half-published handle.
//
// The solution is to do it before concurrency can exist. The lock is created by the
// constructor of `gAtomicLockCreator` below, a static object of this translation unit:
// GCC puts the unit's initialiser in .init_array, the linker script this build uses
// collects that section between SHT$$INIT_ARRAY$$Base and SHT$$INIT_ARRAY$$Limit (both
// PROVIDEd in the map), and usrt2_2.lib's `__cpp_initialize__aeabi_` walks exactly that
// range and calls it before `E32Main()` is reached -- traced from inside the
// constructor with User::InfoPrint, not assumed. At that moment the process has one
// thread and no Rust code has run, because the only way to get a second thread is for
// user code to create one. So the construction needs no mutual exclusion at all: there
// is nobody to exclude.
//
// IT MUST BE A STATIC OBJECT DEFINED AFTER gAtomicLock, NOT __attribute__((constructor)),
// and that is the first thing this slice got wrong. A constructor attribute is emitted
// as its OWN .init_array slot, ahead of the slot holding the translation unit's C++
// static initialiser -- so the lock was created first and `RFastLock`'s own inline
// constructor then ran over it and zeroed `iHandle` again. Observed: `rc=0 h=196610`
// inside the creating function, `h=0` by the time `E32Main` read it, the same address
// (0x400008) both times, and a plain .bss sentinel written beside it surviving intact.
// The first atomic then hit the guard below and the process died. As a static object of
// this unit, C++ orders the two by their order of definition here and the question
// cannot arise.
//
// None of this is taken on trust at run time either. `symrs_atomic_init_status()`
// returns a value only the creator can write -- the sentinel below can never be a
// CreateLocal result -- so an application can assert on the device that the lock was
// created before its own first instruction. examples/atomics does exactly that, as its
// first two cases.
//
// The failure mode is loud, not silent. If a shim entry point is ever reached with no
// lock -- the creator did not run, or CreateLocal failed -- it panics with the category
// below rather than returning a value that is quietly not atomic. A wrong answer from
// an atomic is a bug nobody will find; a panic names the file.

/// The sentinel `gAtomicInitStatus` holds until the constructor overwrites it.
/// `RFastLock::CreateLocal` returns KErrNone or a negative error code and can never
/// return 1, so this value is unambiguous evidence that the constructor did not run.
static const TInt KAtomicNotConstructed = 1;

_LIT(KAtomicPanicCategory, "symrs-atomic");

/// Zero-initialised: `RFastLock`'s constructor is `inline RFastLock() : iCount(0)` over
/// `RHandleBase`'s `iHandle(0)` (e32cmn.h:2437, e32cmn.inl:3158), so the object is
/// constant-initialised into .bss and has no dynamic initialiser of its own to be
/// ordered against ours.
static RFastLock gAtomicLock;

/// In .data, not .bss: the sentinel is non-zero on purpose.
static TInt gAtomicInitStatus = KAtomicNotConstructed;

/// Creates the lock from .init_array before `E32Main()`, on the process's only thread.
///
/// Defined after `gAtomicLock`, and that is load-bearing: within one translation unit
/// C++ initialises static objects in order of definition, so `gAtomicLock`'s own inline
/// constructor has already run by the time this one calls `CreateLocal` on it.
class TAtomicLockCreator
	{
public:
	TAtomicLockCreator()
		{
		gAtomicInitStatus = gAtomicLock.CreateLocal();
		}
	};

static TAtomicLockCreator gAtomicLockCreator;

/// Held for the whole of one atomic operation.
class TAtomicLockGuard
	{
public:
	inline TAtomicLockGuard()
		{
		if (gAtomicLock.Handle() == 0)
			{
			// Never silently non-atomic: the reason code is the CreateLocal error, or
			// the sentinel if the constructor never ran at all.
			User::Panic(KAtomicPanicCategory(), gAtomicInitStatus);
			}
		gAtomicLock.Wait();
		}
	inline ~TAtomicLockGuard()
		{
		gAtomicLock.Signal();
		}
	};

// ---------------------------------------------------------------------------
// THE ENTRY POINTS
// ---------------------------------------------------------------------------
//
// The ABI is the one experiment 72 read off the code GCC and LLVM emit, and the two
// agree: load_N(ptr, mo), store_N(ptr, val, mo), exchange_N(ptr, val, mo) -> old,
// fetch_op_N(ptr, val, mo) -> old, compare_exchange_N(ptr, expected_ptr, desired,
// success_mo, failure_mo) -> bool. compare_exchange takes FIVE arguments: the builtin's
// `weak` flag is not passed to the library routine.
//
// `bool` and not TInt for the compare-exchange result: the caller is LLVM, which
// expects an i1 in the low bits of r0 as the C++ ABI defines it, so the C++ type is
// what gets that right rather than a four-byte TInt that happens to be 0 or 1.

#define SYMRS_ATOMIC_RMW(W, T, NAME, EXPR)                                            \
	SYMRS_EXPORT T symrs_atomic_##NAME##_##W(volatile void* aPtr, T aValue, int aOrder) \
		__asm__("__atomic_" #NAME "_" #W);                                            \
	SYMRS_EXPORT T symrs_atomic_##NAME##_##W(volatile void* aPtr, T aValue, int aOrder) \
		{                                                                             \
		(void)aOrder;                                                                 \
		volatile T* target = (volatile T*)aPtr;                                       \
		TAtomicLockGuard guard;                                                       \
		const T old = *target;                                                        \
		*target = (T)(EXPR);                                                          \
		return old;                                                                   \
		}

#define SYMRS_ATOMIC_FAMILY(W, T)                                                     \
	SYMRS_EXPORT T symrs_atomic_load_##W(const volatile void* aPtr, int aOrder)       \
		__asm__("__atomic_load_" #W);                                                 \
	SYMRS_EXPORT T symrs_atomic_load_##W(const volatile void* aPtr, int aOrder)       \
		{                                                                             \
		(void)aOrder;                                                                 \
		TAtomicLockGuard guard;                                                       \
		return *(const volatile T*)aPtr;                                              \
		}                                                                             \
	SYMRS_EXPORT void symrs_atomic_store_##W(volatile void* aPtr, T aValue, int aOrder) \
		__asm__("__atomic_store_" #W);                                                \
	SYMRS_EXPORT void symrs_atomic_store_##W(volatile void* aPtr, T aValue, int aOrder) \
		{                                                                             \
		(void)aOrder;                                                                 \
		TAtomicLockGuard guard;                                                       \
		*(volatile T*)aPtr = aValue;                                                  \
		}                                                                             \
	SYMRS_ATOMIC_RMW(W, T, exchange, aValue)                                          \
	SYMRS_ATOMIC_RMW(W, T, fetch_add, old + aValue)                                   \
	SYMRS_ATOMIC_RMW(W, T, fetch_sub, old - aValue)                                   \
	SYMRS_ATOMIC_RMW(W, T, fetch_and, old & aValue)                                   \
	SYMRS_ATOMIC_RMW(W, T, fetch_or, old | aValue)                                    \
	SYMRS_ATOMIC_RMW(W, T, fetch_xor, old ^ aValue)                                   \
	SYMRS_ATOMIC_RMW(W, T, fetch_nand, ~(old & aValue))                               \
	SYMRS_EXPORT bool symrs_atomic_compare_exchange_##W(                              \
		volatile void* aPtr, void* aExpected, T aDesired, int aSuccess, int aFailure) \
		__asm__("__atomic_compare_exchange_" #W);                                     \
	SYMRS_EXPORT bool symrs_atomic_compare_exchange_##W(                              \
		volatile void* aPtr, void* aExpected, T aDesired, int aSuccess, int aFailure) \
		{                                                                             \
		(void)aSuccess;                                                               \
		(void)aFailure;                                                               \
		volatile T* target = (volatile T*)aPtr;                                       \
		T* expected = (T*)aExpected;                                                  \
		TAtomicLockGuard guard;                                                       \
		const T old = *target;                                                        \
		if (old == *expected)                                                         \
			{                                                                         \
			*target = aDesired;                                                       \
			return true;                                                              \
			}                                                                         \
		/* On failure the caller's `expected` is overwritten with what was there, */  \
		/* which is what a compare_exchange loop in core relies on. */                \
		*expected = old;                                                              \
		return false;                                                                 \
		}

SYMRS_ATOMIC_FAMILY(1, TUint8)
SYMRS_ATOMIC_FAMILY(2, TUint16)
SYMRS_ATOMIC_FAMILY(4, TUint32)

/// A full barrier, which on this device is a compiler barrier and nothing else.
///
/// ARMv5TE has no DMB -- the instruction arrives with ARMv6K -- so there is no barrier
/// to emit, and the E52's single core cannot reorder its own accesses in a way another
/// observer could see. Preventing the *compiler* from moving accesses across the call
/// is therefore the whole of what this can and needs to do here.
///
/// UNKNOWN, and recorded as such in docs/research/eka2-concurrency.md: whether the OS
/// can ever run two threads of one process on two cores on any device this SDK targets.
/// If one exists, this routine is not enough for it, and nothing on this host can say.
SYMRS_EXPORT void symrs_sync_synchronize(void) __asm__("__sync_synchronize");
SYMRS_EXPORT void symrs_sync_synchronize(void)
	{
	__asm__ __volatile__("" : : : "memory");
	}

// ---------------------------------------------------------------------------
// WHAT AN APPLICATION CAN ASK ABOUT THE LOCK
// ---------------------------------------------------------------------------

SYMRS_EXPORT TInt symrs_atomic_init_status()
	{
	return gAtomicInitStatus;
	}

SYMRS_EXPORT TInt symrs_atomic_lock_handle()
	{
	return gAtomicLock.Handle();
	}
