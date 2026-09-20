// symrs_cstring.cpp -- the C runtime routines LLVM emits calls to and this platform
// does not export. The rule for what belongs in a shim is in symrs_shim.h; this file
// is the one documented exception to it, and the reason is measured.
//
// euser.dso exports memcpy, memset, memmove and memclr, and drtaeabi.dso the whole
// __aeabi_mem* family, so those resolve from ROM (experiment 77 put both DSOs before
// the Rust archive precisely so they would). NOTHING on the link line defines memcmp
// or bcmp -- `nm -D` over euser, drtaeabi, dfpaeabi, scppnwdl and drtrvct2_2 finds
// neither -- and LLVM emits a call to one of them for `a == b` on two `[u8]`. So
// comparing two byte slices in Rust failed to link at all until this file existed.
//
// The alternative was -Zbuild-std-features=compiler-builtins-mem, which does define
// them. Measured, that costs 4.6 kB: compiler_builtins is one codegen unit, so the
// single memcmp reference also drags in __adddf3, __divdf3, __muldf3 and the single-
// precision trio, and --gc-sections cannot drop them because they are weak *global*
// symbols and every dynamic symbol is a collection root in a -shared link (experiment
// 68). `examples/files` came to 15 595 bytes that way and 10 983 this way, with no
// other difference. From this archive the member is pulled only by a program that
// really compares bytes, and it costs everyone else nothing.
#include <e32def.h>

// Both are plain C, defined here rather than taken from the platform because the
// platform does not have them. No Symbian behaviour is being guessed at: this is
// ISO C 7.24.4.1, a byte-wise unsigned comparison of exactly aCount bytes.
extern "C" __attribute__((visibility("hidden")))
TInt memcmp(const void* aLeft, const void* aRight, unsigned int aCount)
	{
	const unsigned char* l = (const unsigned char*)aLeft;
	const unsigned char* r = (const unsigned char*)aRight;
	for (unsigned int i = 0; i < aCount; ++i)
		{
		if (l[i] != r[i])
			return (TInt)l[i] - (TInt)r[i];
		}
	return 0;
	}

// LLVM emits `bcmp` instead of `memcmp` when only equality matters, so a build that
// has one and not the other still fails to link. Same comparison, and the contract is
// weaker: any non-zero value means "different".
extern "C" __attribute__((visibility("hidden")))
TInt bcmp(const void* aLeft, const void* aRight, unsigned int aCount)
	{
	return memcmp(aLeft, aRight, aCount);
	}
