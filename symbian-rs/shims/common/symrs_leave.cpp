// symrs_leave.cpp -- the shim's own proof that a leave raised inside C++ comes back to
// Rust as a value. The rule for what belongs in a shim is in symrs_shim.h.
#include "symrs_shim.h"

#include <e32base.h>
#include <e32std.h>

// e32std.h line 4462: `IMPORT_C static TInt LeaveIfError(TInt aReason);` -- euser's own
// error-to-leave converter, exported as `_ZN4User12LeaveIfErrorEi`. A negative aReason
// raises the leave, zero and positive values are returned unchanged.
//
// This is the one call in the SDK that leaves on demand, so it is how a program checks,
// on the machine it is actually running on, that the trap harness is in place: pass a
// negative code, get the same code back, still be alive. Untrapped, the same call with a
// Rust frame on the stack ends the process with nothing in the log (experiment 76).
SYMRS_EXPORT TInt symrs_leave_if_error(TInt aReason)
	{
	TRAPD(err, User::LeaveIfError(aReason));
	return err;
	}
