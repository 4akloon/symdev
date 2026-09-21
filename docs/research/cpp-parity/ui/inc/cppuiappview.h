#ifndef __CPPUIAPPVIEW_H__
#define __CPPUIAPPVIEW_H__

#include <coecntrl.h>

class CCppUiAppView : public CCoeControl
    {
public:
    static CCppUiAppView* NewL(const TRect& aRect);
    virtual ~CCppUiAppView();

    void Draw(const TRect& aRect) const;
    TKeyResponse OfferKeyEventL(const TKeyEvent& aKeyEvent, TEventCode aType);
    /// The menu commands the AppUi forwards. Returns ETrue when it acted.
    TBool HandleCommand(TInt aCommand);

private:
    CCppUiAppView();
    void ConstructL(const TRect& aRect);
    void SizeChanged();
    void ReportStartupL() const;

    TInt iBars;
    TUint iKeys;
    TUint iCommands;
    };

#endif
