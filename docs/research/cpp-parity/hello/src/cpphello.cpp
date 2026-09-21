// The C++ counterpart of `symbian-rs/examples/hello`: build one line of text on the
// stack, show it with User::InfoPrint, wait five seconds, exit 0.
//
// No CTrapCleanup: neither call below can leave, and the Rust side creates no cleanup
// stack either, so adding one here would be measuring something the Rust side does not
// do.
#include <e32std.h>

_LIT(KGreeting, "Hello from C++ SDK");
_LIT(KFormat, "%S (%d chars)");

GLDEF_C TInt E32Main()
    {
    TBuf<64> note;
    note.Format(KFormat, &KGreeting(), KGreeting().Length());
    User::InfoPrint(note);
    User::After(5000000);
    return KErrNone;
    }
