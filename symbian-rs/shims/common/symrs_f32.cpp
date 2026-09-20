// symrs_f32.cpp -- trapped file-system calls. The rule for what belongs here is in
// symrs_shim.h; read it before adding a function.
#include "symrs_shim.h"

#include <bautils.h>
#include <e32base.h>
#include <f32file.h>

// bautils.h line 59: `IMPORT_C static void EnsurePathExistsL(RFs& aFs, const TDesC&
// aFileName);` -- a leaving static member, exported by bafl.dso as
// `_ZN9BaflUtils17EnsurePathExistsLER3RFsRK7TDesC16`. It leaves with the file server's
// own error, so a bad drive or a read-only path arrives in Rust as an Err rather than
// killing the process.
SYMRS_EXPORT TInt symrs_bafl_ensure_path_exists(RFs* aFs, const TDesC16* aPath)
	{
	if (!aFs || !aPath)
		return KErrArgument;
	TRAPD(err, BaflUtils::EnsurePathExistsL(*aFs, *aPath));
	return err;
	}
