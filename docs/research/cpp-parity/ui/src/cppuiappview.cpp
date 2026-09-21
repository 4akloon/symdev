#include <coemain.h>
#include <eikenv.h>
#include <avkon.hrh>

#include "cppuiappview.h"
#include "cppui.hrh"
#include "symdevreport.h"

const TUint KUid3 = 0xe00006a2;

// How many bars the chart may show, as on the Rust side.
const TInt KMaxBars = 6;

CCppUiAppView* CCppUiAppView::NewL(const TRect& aRect)
    {
    CCppUiAppView* self = new (ELeave) CCppUiAppView;
    CleanupStack::PushL(self);
    self->ConstructL(aRect);
    CleanupStack::Pop(self);
    return self;
    }

CCppUiAppView::CCppUiAppView()
    : iBars(3), iKeys(0), iCommands(0)
    {
    }

CCppUiAppView::~CCppUiAppView()
    {
    }

void CCppUiAppView::ConstructL(const TRect& aRect)
    {
    CreateWindowL();
    SetRect(aRect);
    ActivateL();
    ReportStartupL();
    }

void CCppUiAppView::SizeChanged()
    {
    // Nothing to keep: `Rect()` is the framework's own copy, which is the state the
    // Rust side has to mirror into a field of its own.
    }

/// The same three cases `Bars::construct` records, so `symdev test` compares like
/// with like.
void CCppUiAppView::ReportStartupL() const
    {
    CSymdevReport* report = CSymdevReport::NewL(_L8("uidemo"), KUid3);
    CleanupStack::PushL(report);
    report->Check(_L8("the framework reached the C++ construct"), ETrue);
    TBuf8<32> size;
    size.Format(_L8("%dx%d"), Rect().Width(), Rect().Height());
    report->CheckDetail(_L8("the view was sized before construct"),
                        Rect().Width() > 0 && Rect().Height() > 0, size);
    DrawDeferred();
    report->Check(_L8("a redraw can be asked for from construct"), ETrue);
    report->Finish();
    CleanupStack::PopAndDestroy(report);
    }

void CCppUiAppView::Draw(const TRect& /*aRect*/) const
    {
    CWindowGc& gc = SystemGc();
    const TRect area = Rect();

    gc.SetBrushStyle(CGraphicsContext::ESolidBrush);
    gc.SetBrushColor(KRgbWhite);
    gc.Clear(area);

    const TInt baseline = area.Height() - 40;
    const TInt width = 20;
    const TInt gap = 10;
    const TInt left = 12;
    gc.SetPenColor(KRgbBlack);
    for (TInt i = 0; i < iBars; i++)
        {
        const TInt height = 20 + i * 18;
        gc.SetBrushColor(TRgb(32, 111, 235));
        gc.DrawRect(TRect(TPoint(left + i * (width + gap), baseline - height),
                          TSize(width, height)));
        }
    gc.DrawLine(TPoint(left, baseline), TPoint(area.Width() - left, baseline));

    TBuf<32> note;
    note.Format(_L("bars=%d keys=%u cmd=%u"), iBars, iKeys, iCommands);
    const CFont* font = CEikonEnv::Static()->TitleFont();
    gc.UseFont(font);
    gc.DrawText(note, TPoint(left, baseline + 24));
    gc.DiscardFont();
    }

TKeyResponse CCppUiAppView::OfferKeyEventL(const TKeyEvent& aKeyEvent, TEventCode aType)
    {
    if (aType != EEventKey)
        {
        return EKeyWasNotConsumed;
        }
    switch (aKeyEvent.iCode)
        {
        case EKeyUpArrow:
            if (iBars >= KMaxBars)
                {
                return EKeyWasNotConsumed;
                }
            iBars++;
            break;
        case EKeyDownArrow:
            if (iBars <= 1)
                {
                return EKeyWasNotConsumed;
                }
            iBars--;
            break;
        case EKeyDevice3:  // the selection key
            iBars = 3;
            break;
        default:
            return EKeyWasNotConsumed;
        }
    iKeys++;
    DrawDeferred();
    return EKeyWasConsumed;
    }

TBool CCppUiAppView::HandleCommand(TInt aCommand)
    {
    switch (aCommand)
        {
        case ECppUiMoreBars:
            if (iBars < KMaxBars)
                {
                iBars++;
                iCommands++;
                }
            return ETrue;
        case ECppUiFewerBars:
            if (iBars > 1)
                {
                iBars--;
                iCommands++;
                }
            return ETrue;
        case ECppUiReset:
            iBars = 3;
            iCommands++;
            return ETrue;
        default:
            return EFalse;
        }
    }
