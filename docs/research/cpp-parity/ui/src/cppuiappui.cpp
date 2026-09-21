#include <avkon.hrh>
#include <aknnotewrappers.h>

#include "cppuiappui.h"
#include "cppuiappview.h"
#include "cppui.hrh"

CCppUiAppUi::CCppUiAppUi()
    : iAppView(NULL)
    {
    }

void CCppUiAppUi::ConstructL()
    {
    BaseConstructL(CAknAppUi::EAknEnableSkin);
    iAppView = CCppUiAppView::NewL(ClientRect());
    AddToStackL(iAppView);
    }

CCppUiAppUi::~CCppUiAppUi()
    {
    if (iAppView)
        {
        RemoveFromStack(iAppView);
        delete iAppView;
        }
    }

void CCppUiAppUi::HandleStatusPaneSizeChange()
    {
    CAknAppUi::HandleStatusPaneSizeChange();
    if (iAppView)
        {
        iAppView->SetRect(ClientRect());
        }
    }

void CCppUiAppUi::HandleCommandL(TInt aCommand)
    {
    switch (aCommand)
        {
        case EEikCmdExit:
        case EAknSoftkeyExit:
            Exit();
            return;
        default:
            if (iAppView && iAppView->HandleCommand(aCommand))
                {
                iAppView->DrawDeferred();
                }
        }
    }
