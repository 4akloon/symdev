#ifndef __CPPUIAPPUI_H__
#define __CPPUIAPPUI_H__

#include <aknappui.h>

class CCppUiAppView;

class CCppUiAppUi : public CAknAppUi
    {
public:
    void ConstructL();
    CCppUiAppUi();
    virtual ~CCppUiAppUi();
private:
    void HandleCommandL(TInt aCommand);
    void HandleStatusPaneSizeChange();
private:
    CCppUiAppView* iAppView;
    };

#endif
