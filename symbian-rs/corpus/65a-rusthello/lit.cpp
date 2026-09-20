#include <e32std.h>
_LIT(KHello, "Hi");
extern "C" const TDesC* lit_probe(void) { return &KHello; }
