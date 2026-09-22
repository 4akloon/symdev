// symrs_avkon.cpp -- the four Avkon subclasses a GUI application needs, each virtual
// forwarded to the Rust vtable of symrs_avkon.h. This file is compiled only for a
// project whose manifest has an `[ui]` section; a console application links none of it.
//
// It owns E32Main: for a GUI application the process entry point is
// `EikStart::RunApplication`, and the Rust side never sees it (spec section 1.1).
//
// SYMRS_UID3 is defined on the compile line by symdev, from the manifest's UID3. It is
// the application's identity, so it is generated rather than written twice.
#include "symrs_avkon.h"

#include <aknapp.h>
#include <akndoc.h>
#include <aknappui.h>
#include <avkon.hrh>
#include <coecntrl.h>
#include <e32base.h>
#include <eikenv.h>
#include <eikon.hrh>
#include <eikmenup.h>
#include <eikstart.h>
#include <gdi.h>

#ifndef SYMRS_UID3
#error "SYMRS_UID3 must be defined on the compile line (symdev passes the manifest's uid3)"
#endif

const TUid KSymRsAppUid = { static_cast<TInt32>(SYMRS_UID3) };

// The Rust side reads a TKeyEvent as four words (abi.rs `RawKeyEvent`).
__ASSERT_COMPILE(sizeof(TKeyEvent) == 4 * sizeof(TInt));

// --------------------------------------------------------------------------
// The "down" functions: what Rust may ask of the framework.
// --------------------------------------------------------------------------
// Not one of these can leave, except symrs_menu_add, which traps. `Clear`,
// `SetPenColor`, `SetBrushStyle`, `SetBrushColor`, `DrawRect`, `DrawLine`, `UseFont`, `DrawText` and `DiscardFont` are pure virtuals of
// CGraphicsContext (gdi.h), none of them declared leaving; `DrawDeferred` and
// `CAknAppUi::Exit` are non-leaving members. So there is no TRAP here and none is
// needed -- which is what lets `draw` be reached from outside a trap harness at all.

static CWindowGc* Gc(void* aGc)
	{
	return static_cast<CWindowGc*>(aGc);
	}

static TRgb Rgb(TUint32 aRgb)
	{
	return TRgb(static_cast<TInt>((aRgb >> 16) & 0xff),
		static_cast<TInt>((aRgb >> 8) & 0xff),
		static_cast<TInt>(aRgb & 0xff));
	}

static TRect Rect(SymRsRect aRect)
	{
	return TRect(TPoint(aRect.x, aRect.y), TSize(aRect.w, aRect.h));
	}

// The rect form, and not the no-argument Clear(). Observed in EKA2L1 with a red
// probe stripe at the top of the control: `Clear()` leaves the top ~40 pixels of a
// window-owning control's area unpainted -- the black band experiment 76 saw and
// could not isolate -- while `Clear(aRect)` and `DrawRect(aRect)` over the same area
// both cover it. Whatever narrows the no-argument form's clipping region, the rect
// form is the one that means "my whole view".
void symrs_gc_clear(void* aGc, SymRsRect aRect)
	{
	Gc(aGc)->Clear(Rect(aRect));
	}

void symrs_gc_set_pen(void* aGc, TUint32 aRgb)
	{
	Gc(aGc)->SetPenColor(Rgb(aRgb));
	}

void symrs_gc_set_brush(void* aGc, TUint32 aRgb, TInt aSolid)
	{
	Gc(aGc)->SetBrushStyle(aSolid ? CGraphicsContext::ESolidBrush
		: CGraphicsContext::ENullBrush);
	Gc(aGc)->SetBrushColor(Rgb(aRgb));
	}

void symrs_gc_draw_rect(void* aGc, SymRsRect aRect)
	{
	Gc(aGc)->DrawRect(Rect(aRect));
	}

void symrs_gc_draw_line(void* aGc, TInt aX1, TInt aY1, TInt aX2, TInt aY2)
	{
	Gc(aGc)->DrawLine(TPoint(aX1, aY1), TPoint(aX2, aY2));
	}

// The shim owns the font, so UseFont/DiscardFont never reach Rust and a Rust `draw` can
// never leave one in use. `aX, aY` is the left end of the baseline, which is what
// CGraphicsContext::DrawText(const TDesC&, const TPoint&) takes.
void symrs_gc_draw_text(void* aGc, const TUint16* aText, TInt aLength, TInt aX, TInt aY)
	{
	CEikonEnv* env = CEikonEnv::Static();
	if (!env || !aText || aLength <= 0)
		{
		return;
		}
	const CFont* font = env->TitleFont();
	TPtrC16 text(aText, aLength);
	Gc(aGc)->UseFont(font);
	Gc(aGc)->DrawText(text, TPoint(aX, aY));
	Gc(aGc)->DiscardFont();
	}

void symrs_view_redraw(void* aView)
	{
	static_cast<CCoeControl*>(aView)->DrawDeferred();
	}

void symrs_app_ui_exit(void* aAppUi)
	{
	static_cast<CAknAppUi*>(aAppUi)->Exit();
	}

// The one "down" function that can fail. AddMenuItemL leaves on no memory, so it is
// trapped here and the code is returned: nothing throws while a Rust frame is on the
// stack, and DynInitMenuPaneL raises it once that frame has gone.
//
// `SData` is a plain struct, and eikmenup.h says so in as many words: "NOTICE that
// SData is a structure so all fields in it should be set to avoid any unexpected
// behaviour." A plain line is iCascadeId = 0 and iFlags = 0 -- the same defaults
// eikon.rh gives `STRUCT MENU_ITEM` (`LLINK cascade=0; LONG flags=0;`) -- and an
// empty iExtraText, which is where CEikMenuPane would otherwise show a hotkey name.
TInt symrs_menu_add(void* aPane, const TUint16* aText, TInt aLength, TInt aCommand)
	{
	CEikMenuPane* pane = static_cast<CEikMenuPane*>(aPane);
	if (!pane || !aText)
		{
		return KErrArgument;
		}
	if (aLength < 0)
		{
		aLength = 0;
		}
	// iText is a TBuf<40>; TDes16::Copy of anything longer PANICS (ETDes16Overflow),
	// which no TRAP catches. The Rust side already cuts the label on a character
	// boundary, and this is the descriptor's own invariant kept where it lives.
	if (aLength > CEikMenuPaneItem::SData::ENominalTextLength)
		{
		aLength = CEikMenuPaneItem::SData::ENominalTextLength;
		}
	CEikMenuPaneItem::SData data;
	data.iCommandId = aCommand;
	data.iCascadeId = 0;
	data.iFlags = 0;
	data.iText.Copy(TPtrC16(aText, aLength));
	data.iExtraText.Zero();
	TRAPD(err, pane->AddMenuItemL(data));
	return err;
	}

// --------------------------------------------------------------------------
// CCoeControl: the view.
// --------------------------------------------------------------------------
class CShimView : public CCoeControl
	{
public:
	CShimView(void* aApp) : iApp(aApp) {}
	void ConstructL(const TRect& aRect)
		{
		CreateWindowL();
		SetRect(aRect);
		ActivateL();
		}
	// Public because CShimAppUi offers the key down the stack itself only through
	// CCoeAppUi; this is the framework's own entry and stays where CCoeControl puts it.
	TKeyResponse OfferKeyEventL(const TKeyEvent& aKeyEvent, TEventCode aType)
		{
		// No error channel on purpose (spec section 4.3): a key handler that fails has
		// nowhere to report it, so the Rust side returns only consumed / not consumed.
		const TInt consumed = symrs_app_offer_key(iApp, &aKeyEvent, static_cast<TInt>(aType));
		return consumed ? EKeyWasConsumed : EKeyWasNotConsumed;
		}
private:
	// Private virtual of CCoeControl, called OUTSIDE any trap harness and const.
	// Nothing here may leave, on either side, and there is deliberately no TRAP.
	// Only the control's size crosses; Rust makes it the rect at (0,0): drawing through
	// a window gc is window-relative, and a view that always lays out from (0,0) has no
	// second coordinate system to get wrong. Two words rather than a four-word struct
	// on the stack is experiment 104.
	void Draw(const TRect& /*aRect*/) const
		{
		const TSize size = Size();
		symrs_app_draw(iApp, &const_cast<CShimView*>(this)->SystemGc(), size.iWidth,
			size.iHeight);
		}
	void SizeChanged()
		{
		const TSize size = Size();
		symrs_app_size_changed(iApp, size.iWidth, size.iHeight);
		}
	TInt CountComponentControls() const { return 0; }
	void* iApp; // borrowed; the app UI owns it and outlives this control
	};

// --------------------------------------------------------------------------
// CAknAppUi.
// --------------------------------------------------------------------------
class CShimAppUi : public CAknAppUi
	{
public:
	void ConstructL()
		{
		BaseConstructL(EAknEnableSkin);
		// Everything that can leave happens here, with no Rust frame on the stack.
		iApp = symrs_app_create();
		if (!iApp)
			{
			User::Leave(KErrNoMemory);
			}
		iView = new (ELeave) CShimView(iApp);
		iView->SetMopParent(this);
		iView->ConstructL(ClientRect());
		AddToStackL(iView);
		// The Rust frame runs with no harness open around it; its error becomes a leave
		// only after it has returned, on a stack that is pure C++ again.
		const TInt err = symrs_app_construct(iApp, iView, this);
		User::LeaveIfError(err);
		}
	~CShimAppUi()
		{
		if (iView)
			{
			RemoveFromStack(iView);
			delete iView;
			}
		if (iApp)
			{
			symrs_app_destroy(iApp);
			}
		}
private:
	void HandleCommandL(TInt aCommand)
		{
		if (aCommand == EEikCmdExit || aCommand == EAknSoftkeyExit)
			{
			Exit();
			return;
			}
		const TInt err = symrs_app_command(iApp, aCommand);
		User::LeaveIfError(err);
		}
	// MEikMenuObserver, through CAknAppUi. The application's Options menu is not in
	// any resource: the generated MENU_PANE is empty and every line is added here,
	// each time the menu opens, which is what AddMenuItemL calls adding an item
	// "dynamically". The resource id is not checked because this application has
	// exactly one pane -- the one symdev generated -- so there is no other pane this
	// call can be about.
	void DynInitMenuPaneL(TInt /*aResourceId*/, CEikMenuPane* aMenuPane)
		{
		if (!aMenuPane)
			{
			return;
			}
		// The Rust frame runs with no harness open around it; its error becomes a
		// leave only after it has returned, on a stack that is pure C++ again.
		const TInt err = symrs_app_menu(iApp, aMenuPane);
		User::LeaveIfError(err);
		}
	CShimView* iView;
	void* iApp;
	};

// --------------------------------------------------------------------------
// CAknDocument and CAknApplication: no Rust on the stack anywhere here.
// --------------------------------------------------------------------------
class CShimDocument : public CAknDocument
	{
public:
	CShimDocument(CEikApplication& aApp) : CAknDocument(aApp) {}
private:
	CEikAppUi* CreateAppUiL() { return new (ELeave) CShimAppUi; }
	};

class CShimApplication : public CAknApplication
	{
private:
	TUid AppDllUid() const { return KSymRsAppUid; }
	CApaDocument* CreateDocumentL() { return new (ELeave) CShimDocument(*this); }
	};

LOCAL_C CApaApplication* NewApplication()
	{
	return new CShimApplication;
	}

GLDEF_C TInt E32Main()
	{
	return EikStart::RunApplication(NewApplication);
	}
