// The C++ counterpart of `symbian-rs/examples/hello`: build one line of text on the
// stack, show it with User::InfoPrint, wait five seconds, exit 0.
//
// SYMDEV_CPP_PARITY_CLEANUP toggles the CTrapCleanup pair a conventional C++ E32Main
// opens and closes with, so the parity note can price it against what the same thing
// costs in the Rust no_std entry path. Neither call below can leave, so the
// measurement is of the cleanup stack alone.
#include <e32base.h>

_LIT(KGreeting, "Hello from C++ SDK");
_LIT(KFormat, "%S (%d chars)");

LOCAL_C void Say()
    {
    TBuf<64> note;
    note.Format(KFormat, &KGreeting(), KGreeting().Length());
    User::InfoPrint(note);
    User::After(5000000);
    }

GLDEF_C TInt E32Main()
    {
#ifdef SYMDEV_CPP_PARITY_CLEANUP
    CTrapCleanup* cleanup = CTrapCleanup::New();
    if (cleanup == NULL)
        {
        return KErrNoMemory;
        }
    Say();
    delete cleanup;
#else
    Say();
#endif
    return KErrNone;
    }
