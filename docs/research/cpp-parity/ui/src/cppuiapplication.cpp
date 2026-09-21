#include "cppuiapplication.h"
#include "cppuidocument.h"

// The UID is written here as well as in the mmp and in the registration resource.
const TUid KUidCppUiApp = { 0xe00006a2 };

CApaDocument* CCppUiApplication::CreateDocumentL()
    {
    return static_cast<CApaDocument*>(CCppUiDocument::NewL(*this));
    }

TUid CCppUiApplication::AppDllUid() const
    {
    return KUidCppUiApp;
    }
