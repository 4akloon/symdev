// symrs_rsc.cpp -- the localised-strings resource file (native localisation, Task 3). The
// rule for what belongs here is in symrs_shim.h; read it before adding a function.
//
// Both wrappers are here for rule 1: RResourceFile::OpenL, ConfirmSignatureL and
// AllocReadL leave (barsc.h lines 38-44, the L suffix and IMPORT_C with no exception spec).
// They share a translation unit because no program calls one without the other: the
// strings reader opens the file once and then reads from it.
#include "symrs_shim.h"

#include <barsc.h>
#include <bautils.h>
#include <e32base.h>
#include <f32file.h>

// Constructs an RResourceFile in the caller's KRscFileSize (24-byte) storage, lets
// BaflUtils::NearestLanguageFile (bautils.h line 61, non-leaving) turn aPath into the
// nearest language variant that exists, opens it and confirms its signature -- the
// resource at index 1, which teaches the file the NAME offset its ids carry. The argument
// to ConfirmSignatureL is ignored by this BAFL; 0 is the conventional spelling, as in the
// C++ baseline (docs/research/cpp-parity/locale/src/cpplocale.cpp).
//
// On failure the file is closed again, so the storage holds a closed RResourceFile and
// the caller may try again. KErrArgument for a null pointer or a path longer than a
// TFileName, which TFileName's own constructor would otherwise answer with USER 11.
SYMRS_EXPORT TInt symrs_rsc_open(RFs* aFs, const TDesC16* aPath, RResourceFile* aFile)
	{
	if (!aFs || !aPath || !aFile || aPath->Length() > KMaxFileName)
		return KErrArgument;
	new (aFile) RResourceFile;
	TFileName name(*aPath);
	BaflUtils::NearestLanguageFile(*aFs, name);
	TRAPD(err, aFile->OpenL(*aFs, name); aFile->ConfirmSignatureL(0));
	if (err != KErrNone)
		aFile->Close();
	return err;
	}

// AllocReadL(Offset() + aIndex): the whole resource as it stands, no length prefix read
// (a BUF8 has none; TResourceReader::ReadHBufC8L would run off the end -- BAFL 4). Offset()
// is barsc.h line 72's inline over the exported Offset2(). *aOut is NULL on failure;
// otherwise the caller owns one heap cell and frees it with User::Free.
SYMRS_EXPORT TInt symrs_rsc_read(const RResourceFile* aFile, TInt aIndex, HBufC8** aOut)
	{
	if (!aFile || !aOut)
		return KErrArgument;
	*aOut = NULL;
	TRAPD(err, *aOut = aFile->AllocReadL(aFile->Offset() + aIndex));
	return err;
	}
