#ifndef __CPPUIDOCUMENT_H__
#define __CPPUIDOCUMENT_H__

#include <akndoc.h>

class CEikApplication;

class CCppUiDocument : public CAknDocument
    {
public:
    static CCppUiDocument* NewL(CEikApplication& aApp);
    virtual ~CCppUiDocument();
public:
    CEikAppUi* CreateAppUiL();
private:
    CCppUiDocument(CEikApplication& aApp);
    };

#endif
