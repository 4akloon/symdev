#include "cppuidocument.h"
#include "cppuiappui.h"

CCppUiDocument* CCppUiDocument::NewL(CEikApplication& aApp)
    {
    return new (ELeave) CCppUiDocument(aApp);
    }

CCppUiDocument::CCppUiDocument(CEikApplication& aApp)
    : CAknDocument(aApp)
    {
    }

CCppUiDocument::~CCppUiDocument()
    {
    }

CEikAppUi* CCppUiDocument::CreateAppUiL()
    {
    return static_cast<CEikAppUi*>(new (ELeave) CCppUiAppUi);
    }
