#ifndef __CPPUIAPPLICATION_H__
#define __CPPUIAPPLICATION_H__

#include <aknapp.h>

class CCppUiApplication : public CAknApplication
    {
public:
    TUid AppDllUid() const;
protected:
    CApaDocument* CreateDocumentL();
    };

#endif
