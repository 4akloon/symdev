// symrs_process.cpp -- the process calls whose signatures are not C signatures. The
// rule for what belongs here is in symrs_shim.h; read it before adding a function.
#include "symrs_shim.h"

#include <e32std.h>

// e32std.h line 3771: `IMPORT_C TFileName FileName() const;` -- a TBuf16<256> returned
// BY VALUE. That is rule 2 of symrs_shim.h: a 520-byte class with a non-trivial copy
// constructor comes back through a hidden sret pointer that displaces `this`, which is
// not an extern "C" signature. So the copy is made here and handed out as an ordinary
// descriptor the caller owns.
//
// `RProcess()` is the CURRENT process: e32std.inl line 3768 constructs it with
// KCurrentProcessHandle, and e32const.h line 572 gives that as 0xffff0000|KHandleNoClose,
// so there is nothing to open and nothing to close.
//
// FileName() is non-leaving and allocates nothing. KErrOverflow rather than a panic if
// the caller's descriptor is too short, because a descriptor overflow is USER 11 and no
// TRAP catches it.
SYMRS_EXPORT TInt symrs_process_file_name(TDes16* aOut)
	{
	if (!aOut)
		return KErrArgument;
	RProcess me;
	TFileName name = me.FileName();
	if (name.Length() > aOut->MaxLength())
		return KErrOverflow;
	aOut->Copy(name);
	return KErrNone;
	}
