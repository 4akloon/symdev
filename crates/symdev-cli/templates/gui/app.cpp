// symdev GUI template: minimal S60 3rd Edition Avkon application.
#include <aknapp.h>
#include <akndoc.h>
#include <aknappui.h>
#include <coecntrl.h>
#include <eikenv.h>
#include <eikstart.h>
#include <avkon.hrh>
#include <gdi.h>

const TUid KUidThisApp = { static_cast<TInt32>({{UID3}}) };

class CAppView : public CCoeControl
    {
public:
    void ConstructL(const TRect& aRect)
        {
        CreateWindowL();
        SetRect(aRect);
        ActivateL();
        }
private:
    void Draw(const TRect& /*aRect*/) const
        {
        CWindowGc& gc = SystemGc();
        gc.Clear(Rect());
        const CFont* font = CEikonEnv::Static()->TitleFont();
        gc.UseFont(font);
        gc.SetPenColor(KRgbBlack);
        const TInt baseline = Rect().Height() / 2 + font->AscentInPixels() / 2;
        gc.DrawText(_L("Hello from symdev"), Rect(), baseline, CGraphicsContext::ECenter);
        gc.DiscardFont();
        }
    };

class CAppAppUi : public CAknAppUi
    {
public:
    void ConstructL()
        {
        BaseConstructL(EAknEnableSkin);
        iView = new (ELeave) CAppView;
        iView->SetMopParent(this);
        iView->ConstructL(ClientRect());
        AddToStackL(iView);
        }
    ~CAppAppUi()
        {
        if (iView)
            {
            RemoveFromStack(iView);
            delete iView;
            }
        }
private:
    void HandleCommandL(TInt aCommand)
        {
        if (aCommand == EEikCmdExit || aCommand == EAknSoftkeyExit)
            {
            Exit();
            }
        }
    CAppView* iView;
    };

class CAppDocument : public CAknDocument
    {
public:
    CAppDocument(CEikApplication& aApp) : CAknDocument(aApp) {}
private:
    CEikAppUi* CreateAppUiL() { return new (ELeave) CAppAppUi; }
    };

class CAppApplication : public CAknApplication
    {
private:
    TUid AppDllUid() const { return KUidThisApp; }
    CApaDocument* CreateDocumentL() { return new (ELeave) CAppDocument(*this); }
    };

LOCAL_C CApaApplication* NewApplication()
    {
    return new CAppApplication;
    }

GLDEF_C TInt E32Main()
    {
    return EikStart::RunApplication(NewApplication);
    }
