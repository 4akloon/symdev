// symrs_active.cpp -- the CActive subclass a Rust executor runs on (step 73).
//
// This file is in the shim for rule 3 of symrs_shim.h and for nothing else: CActive's
// RunL() and DoCancel() are pure virtual, its constructor and SetActive() are protected,
// and CActiveScheduler::Start() is a function whose header says nothing about leaving.
// Subclassing is the only way to have a CActive at all, and a Rust type cannot be a C++
// subclass. Everything else the executor touches -- RTimer, CActiveScheduler::Current --
// is a plain non-leaving call that symbian-sys declares and Rust makes directly.
//
// The forwarding shape is the one experiment 76 settled for the Avkon virtuals: a
// Rust-owned vtable of function pointers plus one opaque context that is the Rust
// side's identity. No C++ state is exposed and no Symbian type crosses the boundary
// except TRequestStatus*, whose layout was measured (8 bytes, e32cmn.h line 2097).
#include <e32base.h>
#include <e32std.h>

#include "symrs_shim.h"

// The active object itself. One per outstanding request: the Rust side creates it,
// hands the TRequestStatus to whatever asynchronous service it is driving, marks it
// active, and destroys it when the future that owns it is dropped.
class CSymRsActive : public CActive
    {
public:
    CSymRsActive(const SymRsActiveVTable* aVTable, TAny* aContext, TInt aPriority)
        : CActive(aPriority), iVTable(aVTable), iContext(aContext) {}
    TRequestStatus& Status() { return iStatus; }
    void Issued() { SetActive(); }
private:
    void RunL();
    void DoCancel();
private:
    const SymRsActiveVTable* iVTable;
    TAny* iContext;
    };

// The scheduler has already marked the request complete and this runs under its trap
// harness. The Rust callback stores the completion code, wakes the waker and drives the
// executor; it cannot leave, because the whole Rust text is one cantunwind range.
//
// If it ever needs to fail it returns a TInt, and the leave happens HERE, after the
// Rust frame has returned and the stack is pure C++ again (symrs_shim.h, and section
// 4.3 of avkon-rust-spec.md). A leave from RunL is handled by CActive::RunError, whose
// default returns the code to CActiveScheduler::Error(), which panics -- a loud
// diagnostic rather than a swallowed error, which is what this SDK wants.
void CSymRsActive::RunL()
    {
    TInt err = iVTable->iRun(iContext, iStatus.Int());
    User::LeaveIfError(err);
    }

// Called by CActive::Cancel() only when the object is active. The Rust callback calls
// the cancel of whatever service holds the request -- RTimer::Cancel() for a sleep --
// which completes the status immediately with KErrCancel; Cancel() then consumes that
// completion itself.
void CSymRsActive::DoCancel()
    {
    iVTable->iCancel(iContext);
    }

SYMRS_EXPORT TAny* symrs_active_new(const SymRsActiveVTable* aVTable, TAny* aContext,
                                    TInt aPriority)
    {
    if (!aVTable || !aVTable->iRun || !aVTable->iCancel)
        return NULL;
    // CActiveScheduler::Add panics (E32USER-CBase 41) when no scheduler is installed on
    // this thread, and a panic is not catchable. The check turns that into a NULL the
    // Rust side reports as an error naming the fix.
    if (!CActiveScheduler::Current())
        return NULL;
    // CBase's operator new is User::AllocZ and does NOT leave; `new (ELeave)` is the one
    // that does. NULL on a full heap is an error value, not an exception.
    CSymRsActive* self = new CSymRsActive(aVTable, aContext, aPriority);
    if (!self)
        return NULL;
    CActiveScheduler::Add(self);
    return self;
    }

// Cancel first: ~CActive panics (E32USER-CBase 40) if the object is still active, and
// Cancel() on an inactive object does nothing. ~CActive dequeues.
SYMRS_EXPORT void symrs_active_destroy(TAny* aActive)
    {
    if (!aActive)
        return;
    CSymRsActive* self = static_cast<CSymRsActive*>(aActive);
    self->Cancel();
    delete self;
    }

SYMRS_EXPORT TRequestStatus* symrs_active_status(TAny* aActive)
    {
    if (!aActive)
        return NULL;
    return &static_cast<CSymRsActive*>(aActive)->Status();
    }

SYMRS_EXPORT void symrs_active_issued(TAny* aActive)
    {
    if (aActive)
        static_cast<CSymRsActive*>(aActive)->Issued();
    }

SYMRS_EXPORT void symrs_active_cancel(TAny* aActive)
    {
    if (aActive)
        static_cast<CSymRsActive*>(aActive)->Cancel();
    }

// The scheduler a console application owns. A GUI application must never call these:
// CONE installs CCoeScheduler before any application code runs and CCoeEnv is itself a
// CActive on it, so installing a second one panics and stopping CONE's ends the
// application (avkon-rust-spec.md section 1.3). The executor's joined form uses none of
// this; it only adds objects to whatever scheduler is already there.
SYMRS_EXPORT TInt symrs_scheduler_install(TAny** aScheduler)
    {
    if (!aScheduler)
        return KErrArgument;
    *aScheduler = NULL;
    // Install panics (E32USER-CBase 43) when one is already installed, so the caller
    // gets KErrInUse instead -- which is what a GUI application would hit.
    if (CActiveScheduler::Current())
        return KErrInUse;
    CActiveScheduler* scheduler = new CActiveScheduler;
    if (!scheduler)
        return KErrNoMemory;
    CActiveScheduler::Install(scheduler);
    *aScheduler = scheduler;
    return KErrNone;
    }

SYMRS_EXPORT void symrs_scheduler_uninstall(TAny* aScheduler)
    {
    if (!aScheduler)
        return;
    CActiveScheduler::Install(NULL);
    delete static_cast<CActiveScheduler*>(aScheduler);
    }

// The one TRAP in this file. CActiveScheduler::Start() is `static void Start()` and
// e32base.h says nothing either way about leaving, which rule 1 of symrs_shim.h counts
// as leaving; and every RunL in the program runs underneath this call, so an escaping
// leave would cross the Rust frame that called it.
SYMRS_EXPORT TInt symrs_scheduler_start()
    {
    TRAPD(err, CActiveScheduler::Start());
    return err;
    }

SYMRS_EXPORT void symrs_scheduler_stop()
    {
    CActiveScheduler::Stop();
    }
