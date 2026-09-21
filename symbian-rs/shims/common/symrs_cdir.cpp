// symrs_cdir.cpp -- destroying the CDir that RFs::GetDir allocates. One wrapper, one
// translation unit: the recorded GCCE argv has no -ffunction-sections, so --gc-sections
// drops an unused wrapper only when it is the whole of an object's text. Keeping this
// out of symrs_f32.cpp is what keeps every no_std example byte-identical.
#include "symrs_shim.h"

#include <e32base.h>
#include <f32file.h>

// f32file.h line 1597: `class CDir : public CBase` declares `IMPORT_C virtual ~CDir();`.
// A virtual destructor is rule 3 of symrs_shim.h: `delete aDir` dispatches through the
// vtable, and Rust cannot. Calling the exported `_ZN4CDirD0Ev` straight would work only
// by assuming the dynamic type really is CDir -- true today, because RFs::GetDir builds
// it with CDir::NewL(), but an assumption all the same. This is the C++ that is not.
//
// It cannot leave: ~CDir() frees a CArrayPakFlat and nothing in that path allocates.
SYMRS_EXPORT void symrs_f32_dir_delete(CDir* aDir)
	{
	delete aDir;
	}
