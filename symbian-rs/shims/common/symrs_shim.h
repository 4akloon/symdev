// symrs_shim.h -- the C++ side of the Rust SDK (design spec section 7, step 70).
//
// Every function here is `extern "C"`, takes and returns only C types, and is a complete
// TRAP unit: it enters C++, opens a trap harness around the leaving SDK call, closes it,
// and returns a TInt. Rust turns a negative TInt into a SymbianError. Nothing ever
// throws while a Rust frame is on the stack.
//
// ---------------------------------------------------------------------------
// WHEN A CALL NEEDS A SHIM
// ---------------------------------------------------------------------------
//
// A Symbian call needs a wrapper here when ANY of the following is true. Otherwise
// `symbian-sys` declares it and Rust calls it directly, which is cheaper and is what
// most of euser already gets.
//
// 1. IT CAN LEAVE. The SDK's convention is a trailing L / LC / LD, but the name is a
//    hint, not the authority: confirm against the declaration in the SDK header, and
//    treat a function whose header says nothing either way as leaving.
//
//    Why this is absolute: `variant/symbian_os_v9.3.hrh` line 651 defines
//    __LEAVE_EQUALS_THROW__, so a leave is a real C++ exception unwound by drtaeabi's
//    _Unwind_* with __gxx_personality_v0. rustc emits one `cantunwind` .ARM.exidx entry
//    covering the whole Rust text, so an exception that reaches a Rust frame terminates
//    the process with NO diagnostic at all -- no panic, no KERN-EXEC, nothing in the log
//    (experiment 76, probe A-). The TRAP therefore goes around the leaving SDK call
//    *inside* this file, never around a call into Rust.
//
// 2. ITS SIGNATURE IS NOT A C SIGNATURE. A class type returned by value with a
//    non-trivial copy constructor is returned through a hidden pointer (sret), which
//    displaces `this`; a `TRefByValue` varargs function such as TDes16::Format has no
//    stable C spelling; a virtual call needs the vtable. None of those is expressible as
//    an `extern "C"` declaration, so they are wrapped here.
//
//    Observed for the sret case: `d->Left(n)` compiled from `probe_left(const TDesC16*,
//    TInt)` with the recorded GCCE argv gives `mov r0,sp; movs r1,r0; movs r2,n;
//    bl _ZNK7TDesC164LeftEi` -- the return slot is argument 0 and `this` moves to
//    argument 1. It is expressible, but only with that shape spelled out per signature.
//
// 3. IT IS A VIRTUAL MEMBER, or a member of a class with multiple or virtual
//    inheritance, where `this` may need adjusting. Not observed in this repository; a
//    guess is not acceptable, so such calls are wrapped here or not made at all.
//
// NOT a reason for a shim: BEING A NON-STATIC MEMBER FUNCTION. That was open until
// step 70 and is now observed, not guessed. `probe_append(TDes16* d, const TDesC16* s)
// { d->Append(*s); }` compiled with the recorded GCCE argv is a bare
// `bl _ZN6TDes166AppendERK7TDesC16` with no register shuffle whatsoever, and
// `d->AppendNum((TInt64)n)` emits `movs r2,r1; asrs r3,r1,#31` -- the 64-bit argument
// lands in r2:r3 and skips r1. So for a non-virtual member with scalar or pointer
// arguments and a scalar or void return, the calling convention is exactly the AAPCS one
// with `this` prepended as argument 0, and Rust declares it as
// `extern "C" fn(this: *mut T, ...)`. Evidence: experiment 78.
//
// TWO DOCUMENTED EXCEPTIONS, in symrs_cstring.cpp and symrs_atomic.cpp: the C runtime
// routines LLVM emits calls to and this platform does not export. euser exports
// memcpy/memset/memmove/memclr and drtaeabi the __aeabi_mem* family, but nothing on
// the link line defines memcmp or bcmp, and LLVM emits one of them for `a == b` on
// two byte slices. They are plain C with no Symbian call in them, they live in this
// archive because that is the one archive every Rust application already links, and
// they are pulled only by a program that really compares bytes. symrs_atomic.cpp is
// the same case one step larger: LLVM lowers every core::sync::atomic operation on
// ARMv5TE to an __atomic_* libcall, nothing on the link line defines one, and euser's
// four TInt counters are not a substitute. That file states its own argument in full.
//
// A panic is NOT a leave and a TRAP does not catch it: e32panic.h line 131 documents
// ETDes16Overflow = 11 (category USER) for "any of the copying, appending or formatting
// member functions". Nothing here can turn a panic into an error, so a Rust wrapper that
// could provoke one must make it unreachable by checking first.
#ifndef SYMRS_SHIM_H
#define SYMRS_SHIM_H

#include <e32def.h>

class RFs;
class TDesC16;

// The shim is an implementation detail of the SDK, not an export of the application.
// Hidden visibility keeps these symbols out of the E32's dynamic table, so --gc-sections
// on the Rust link line can drop a wrapper no program calls -- and with it the import of
// the DLL that wrapper needed.
#define SYMRS_EXPORT extern "C" __attribute__((visibility("hidden")))

// BaflUtils::EnsurePathExistsL(RFs&, const TDesC&) from bafl.dso, TRAPped.
// Creates every directory in `aPath`'s path component that does not exist yet.
// Returns KErrNone, the leave code, or KErrArgument for a null argument.
SYMRS_EXPORT TInt symrs_bafl_ensure_path_exists(RFs* aFs, const TDesC16* aPath);

// User::LeaveIfError(TInt) from euser.dso, TRAPped: the shim's own self-check.
// Returns aReason for a negative aReason, KErrNone otherwise.
SYMRS_EXPORT TInt symrs_leave_if_error(TInt aReason);

// symrs_atomic.cpp: what the atomic lock's static constructor recorded. KErrNone once
// the lock exists, 1 if the constructor never ran, otherwise the CreateLocal error.
// An application can read this to prove, on the machine in hand, that the lock was in
// place before its own first instruction.
SYMRS_EXPORT TInt symrs_atomic_init_status();

// symrs_atomic.cpp: the atomic lock's kernel handle, 0 if there is none.
SYMRS_EXPORT TInt symrs_atomic_lock_handle();

#endif // SYMRS_SHIM_H
