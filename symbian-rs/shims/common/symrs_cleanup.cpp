// symrs_cleanup.cpp -- destroying a thread's CTrapCleanup. One wrapper, one translation
// unit, for the --gc-sections reason written in symrs_cdir.cpp.
#include "symrs_shim.h"

#include <e32base.h>

// CTrapCleanup: `delete aCleanup`. Here for rule 3 -- ~CTrapCleanup is virtual (it is a
// CBase) -- and paired with CTrapCleanup::New(), which Rust calls directly because it is
// a plain static returning a pointer. Every Symbian thread needs its own cleanup stack
// or the first CleanupStack::PushL below it panics E32USER-CBase 69; std installs one in
// std::os::symbian::start and in sys::thread's trampoline.
SYMRS_EXPORT void symrs_cleanup_destroy(CTrapCleanup* aCleanup)
	{
	delete aCleanup;
	}
