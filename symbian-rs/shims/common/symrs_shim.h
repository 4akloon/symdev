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
// WHAT IS *NOT* A REASON TO BE HERE: touching Symbian at all. A file belongs in this
// directory when it needs TRAP -- or, from step 75, when it has to define a C++
// subclass with virtual methods. Nothing else. Everything the SDK can express in Rust
// is written in Rust, because the second toolchain is a cost and the only thing it
// buys is the trap harness.
//
// Two files that used to live here have gone to Rust for exactly that reason:
// symrs_atomic.cpp, the __atomic_* family over an RFastLock, is now
// crates/symbian-libcalls/src/atomic.rs, and symrs_cstring.cpp (memcmp and bcmp) is
// crates/symbian-libcalls/src/cstring.rs. Neither can leave: RFastLock::CreateLocal,
// Wait and Signal are non-leaving euser members, and the member ABI is ordinary AAPCS
// with `this` as argument 0 (observed, experiment 78), so Rust calls them directly.
// What remains in this directory is the two TRAPs, in symrs_leave.cpp and symrs_f32.cpp,
// and the one C++ subclass, in symrs_active.cpp -- which is rule 3 and carries the third
// TRAP, around CActiveScheduler::Start().
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
class TRequestStatus;

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


// ---------------------------------------------------------------------------
// THE ACTIVE OBJECT (symrs_active.cpp, step 73)
// ---------------------------------------------------------------------------
//
// Here for rule 3: CActive::RunL and DoCancel are pure virtual, its constructor and
// SetActive() are protected, so a subclass is the only way to have one, and a Rust type
// cannot be a C++ subclass. The virtuals forward to a Rust-owned vtable of function
// pointers with one opaque context, the shape experiment 76 settled.
//
// The Rust callbacks may not leave and may not panic. iRun returns a TInt and the leave,
// if there ever is one, happens in RunL after the Rust frame has returned.
struct SymRsActiveVTable
    {
    // Called from RunL with the completion code the service wrote into iStatus.
    TInt (*iRun)(TAny* aContext, TInt aStatus);
    // Called from DoCancel: cancel the service that holds the request. It must complete
    // the status immediately, which is what RTimer::Cancel and RSocket::CancelAll do.
    void (*iCancel)(TAny* aContext);
    };

// Creates one active object and adds it to the scheduler installed on this thread.
// NULL when there is no scheduler, when the vtable is incomplete, or on a full heap.
SYMRS_EXPORT TAny* symrs_active_new(const SymRsActiveVTable* aVTable, TAny* aContext,
                                    TInt aPriority);
// Cancels the request if it is outstanding, dequeues and deletes. Null-safe.
SYMRS_EXPORT void symrs_active_destroy(TAny* aActive);
// The object's own iStatus, to hand to an asynchronous service.
SYMRS_EXPORT TRequestStatus* symrs_active_status(TAny* aActive);
// CActive::SetActive(), after the request has been issued.
SYMRS_EXPORT void symrs_active_issued(TAny* aActive);
// CActive::Cancel(): DoCancel, then consume the completion. Null-safe, and a no-op when
// the request is not outstanding.
SYMRS_EXPORT void symrs_active_cancel(TAny* aActive);

// The scheduler a CONSOLE application owns. A GUI application must never call these:
// CONE installs CCoeScheduler before any application code runs (avkon-rust-spec.md 1.3).
// KErrInUse when one is already installed, KErrNoMemory on a full heap.
SYMRS_EXPORT TInt symrs_scheduler_install(TAny** aScheduler);
SYMRS_EXPORT void symrs_scheduler_uninstall(TAny* aScheduler);
// CActiveScheduler::Start(), TRAPped: e32base.h says nothing either way about it
// leaving, and every RunL in the program runs underneath it.
SYMRS_EXPORT TInt symrs_scheduler_start(void);
SYMRS_EXPORT void symrs_scheduler_stop(void);


#endif // SYMRS_SHIM_H
