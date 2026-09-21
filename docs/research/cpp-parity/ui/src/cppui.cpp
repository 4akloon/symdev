// The process entry point. `EikStart::RunApplication` owns everything after this.
#include <eikstart.h>
#include "cppuiapplication.h"

LOCAL_C CApaApplication* NewApplication()
    {
    return new CCppUiApplication;
    }

GLDEF_C TInt E32Main()
    {
    return EikStart::RunApplication(NewApplication);
    }
