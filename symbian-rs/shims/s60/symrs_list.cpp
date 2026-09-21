// symrs_list.cpp -- the S60 list box, behind the plain C entry points of symrs_list.h.
//
// WHICH CLASS, AND WHY. CAknSingleStyleListBox (aknlists.h line 224), the `list_single_pane`
// style. It is the simplest member of the single-line family that shows text and nothing
// else: every other single-line style (`CAknSingleGraphicStyleListBox`,
// `CAknSingleNumberStyleListBox`, `CAknSingleHeadingStyleListBox`) puts an icon index or a
// second string in column 0 and would need a CAknIconArray this shim has nothing to put in.
//
// THE ROW FORMAT, from aknlists.h lines 216-222 and not from recollection:
//
//     list_single_pane
//     list item string format: "\tTextLabel\t0\t1"
//     where 0 and 1 are indexes to icon array
//
// So a row is TAB-separated columns: an empty leading column, the label, and two icon-array
// indices. This shim sets NO icon array, so it writes the leading tab and the label and
// stops. Writing "0" and "1" would index an array that does not exist, and what a column
// list box does with an index into an absent icon array was never observed here.
//
// OWNERSHIP OF THE ITEM ARRAY, which is a double free when it is got wrong.
// CTextListBoxModel::SetItemTextArray (eiktxlbm.h) only ASSIGNS -- it does not delete what
// was there -- and TListBoxModelItemArrayOwnership (eiklbm.h line 26) decides who frees it:
// ELbmOwnsItemArray = 0, ELbmDoesNotOwnItemArray = 1. THIS SHIM OWNS THE ARRAY. It hands the
// model the same CDesCArrayFlat once, with ELbmDoesNotOwnItemArray, and every later change
// resets and refills that one array. So no ownership ever changes hands, the non-deleting
// assignment can neither leak nor double-free, and ~CSymRsList deletes the array itself --
// after the list box, so nothing can reach it in between.
//
// KEYS: WHO WINS. coeaui.h lines 36-38 state the rule -- "Controls with higher priorities
// get offered key events before controls with lower priorities." symrs_avkon.cpp puts
// CShimView on the stack at the default priority (ECoeStackPriorityDefault = 0), so this
// list goes on at ECoeStackPriorityDefault + 1 and is offered every key first. It consumes
// the arrows and the selection key and returns EKeyWasNotConsumed for the rest, which then
// reaches CShimView and so the Rust App::key. The application sees selections, not arrows,
// and the ordering is a documented priority rather than an insertion order nobody wrote down.
//
// Every mangled name cited below is the unversioned part of what
// `arm-none-symbianelf-nm -D --defined-only` printed on
// ~/sdk/S60_3rd_FP2/epoc32/release/armv5/lib/<dso>, 2026-09-21.
#include "symrs_list.h"

#include <aknappui.h>
#include <aknlists.h>
#include <avkon.hrh>
#include <badesca.h>
#include <coeaui.h>
#include <coecntrl.h>
#include <e32base.h>
#include <eiklbo.h>
#include <eiksbfrm.h>

// The column separator of every tab-separated list style.
const TUint16 KSymRsListColumn = 0x0009; // TAB

// The greatest number of UTF-16 code units one row's label may hold. The Rust side
// refuses a longer one before it gets here, so this is a second wall, not the first.
const TInt KSymRsListMaxLabel = 128;

// --------------------------------------------------------------------------
// CSymRsList: the list box, its item array and the observer, as one owner.
// --------------------------------------------------------------------------
// It is the MEikListBoxObserver itself (eiklbo.h), which is an ordinary C++ interface with
// one pure virtual -- so the forwarding is the same shape as symrs_avkon.cpp's: implement
// the virtual here, call a Rust function pointer, turn its TInt into a leave afterwards.
class CSymRsList : public CBase, public MEikListBoxObserver
	{
public:
	static CSymRsList* NewL(CAknAppUi* aAppUi, const TRect& aRect,
		const SymRsListCallbacks* aCallbacks, void* aOwner);
	~CSymRsList();

	void Clear();
	void AddL(const TUint16* aText, TInt aLength);
	void CommitL();
	TInt Count() const;
	TInt Selected() const;
	TInt SetSelected(TInt aIndex);

private:
	CSymRsList(CAknAppUi* aAppUi, const SymRsListCallbacks* aCallbacks, void* aOwner);
	void ConstructL(const TRect& aRect);
	// From MEikListBoxObserver.
	void HandleListBoxEventL(CEikListBox* aListBox, TListBoxEvent aEventType);

	CAknAppUi* iAppUi;                       // borrowed; outlives this object
	const SymRsListCallbacks* iCallbacks;    // borrowed; Rust .rodata or stack-free
	void* iOwner;                            // opaque Rust pointer, never dereferenced
	CAknSingleStyleListBox* iBox;            // owned
	CDesCArrayFlat* iItems;                  // owned -- see the ownership note above
	TBool iOnStack;
	};

CSymRsList::CSymRsList(CAknAppUi* aAppUi, const SymRsListCallbacks* aCallbacks, void* aOwner)
	: iAppUi(aAppUi), iCallbacks(aCallbacks), iOwner(aOwner), iBox(NULL), iItems(NULL),
	  iOnStack(EFalse)
	{
	}

CSymRsList* CSymRsList::NewL(CAknAppUi* aAppUi, const TRect& aRect,
	const SymRsListCallbacks* aCallbacks, void* aOwner)
	{
	CSymRsList* self = new (ELeave) CSymRsList(aAppUi, aCallbacks, aOwner);
	CleanupStack::PushL(self);
	self->ConstructL(aRect);
	CleanupStack::Pop(self);
	return self;
	}

void CSymRsList::ConstructL(const TRect& aRect)
	{
	// bafl.dso 000002a0 _ZN16CDesC16ArrayFlatC1Ei -- CDesC16ArrayFlat(TInt aGranularity).
	// badesca.h line 246 typedefs CDesCArrayFlat to it.
	iItems = new (ELeave) CDesCArrayFlat(8);
	// avkon.dso 00001f40 _ZN22CAknSingleStyleListBoxC1Ev.
	iBox = new (ELeave) CAknSingleStyleListBox;
	// eikcoctl.dso 00000610 _ZN15CEikTextListBox10ConstructLEPK11CCoeControli --
	// CEikTextListBox::ConstructL(const CCoeControl*, TInt), public at eiktxlbx.h line 68
	// although CEikListBox's own is protected. A NULL parent is allowed (eiklbx.h line
	// 1042: "The parent control. May be NULL"), and ECreateOwnWindow = 0x0200
	// (lafpublc.h line 154) makes the list a window-owning control -- which is what lets
	// it live beside CShimView on the control stack instead of inside it. A windowless
	// child would have to be returned from CShimView::CountComponentControls, and that
	// virtual is not forwarded to Rust.
	// EAknListBoxSelectionList is EAknGenericListBoxFlags is EAknListBoxScrollBarSizeExcluded
	// = 0x0080 (avkon.hrh lines 46, 64, 75).
	iBox->ConstructL(NULL, EAknListBoxSelectionList | CEikListBox::ECreateOwnWindow);
	// eikcoctl.dso 000013d4 _ZNK15CEikTextListBox5ModelEv,
	//              00000970 _ZN17CTextListBoxModel16SetItemTextArrayEP12MDesC16Array,
	//              00000974 _ZN17CTextListBoxModel16SetOwnershipTypeE31TListBoxModelItemArrayOwnership.
	CTextListBoxModel* model = iBox->Model();
	model->SetItemTextArray(iItems);
	model->SetOwnershipType(ELbmDoesNotOwnItemArray);
	// eikcoctl.dso 000000b4 _ZN11CEikListBox21CreateScrollBarFrameLEi and
	//              00000a4c _ZN18CEikScrollBarFrame23SetScrollBarVisibilityLENS_20TScrollBarVisibilityES0_.
	// EOff = 0, EOn = 1, EAuto = 2 (eiksbfrm.h line 107): no horizontal bar ever, a
	// vertical one as soon as the items overflow.
	iBox->CreateScrollBarFrameL(ETrue);
	iBox->ScrollBarFrame()->SetScrollBarVisibilityL(CEikScrollBarFrame::EOff,
		CEikScrollBarFrame::EAuto);
	// eikcoctl.dso 0000007c _ZN11CEikListBox18SetListBoxObserverEP19MEikListBoxObserver.
	iBox->SetListBoxObserver(this);
	iBox->SetRect(aRect);
	iBox->ActivateL();
	// Without focus the list draws no highlight, and a list with no highlight cannot show
	// a selection moving. SetFocus is a non-leaving CCoeControl member.
	iBox->SetFocus(ETrue);
	// coeaui.h lines 36-38: higher priority is offered keys first. CShimView is at
	// ECoeStackPriorityDefault, so +1 puts this list above it, definitely and by a rule
	// rather than by the order two AddToStackL calls happened to run in.
	iAppUi->AddToStackL(iBox, ECoeStackPriorityDefault + 1);
	iOnStack = ETrue;
	}

CSymRsList::~CSymRsList()
	{
	if (iBox)
		{
		if (iOnStack && iAppUi)
			{
			iAppUi->RemoveFromStack(iBox);
			}
		delete iBox;
		}
	// After the box, never before: the model holds a borrowed pointer to this array and
	// only this destructor frees it (ELbmDoesNotOwnItemArray).
	delete iItems;
	}

void CSymRsList::Clear()
	{
	if (iItems)
		{
		// bafl.dso 0000009c _ZN12CDesC16Array5ResetEv -- non-leaving.
		iItems->Reset();
		}
	}

void CSymRsList::AddL(const TUint16* aText, TInt aLength)
	{
	if (!iItems || !aText || aLength < 0 || aLength > KSymRsListMaxLabel)
		{
		User::Leave(KErrArgument);
		}
	// The row, in the format aknlists.h documents for list_single_pane, minus the two
	// icon columns this shim has no icon array for.
	HBufC* row = HBufC::NewLC(aLength + 1);
	TPtr ptr = row->Des();
	ptr.Append(TChar(KSymRsListColumn));
	ptr.Append(TPtrC16(aText, aLength));
	// bafl.dso 000000a8 _ZN12CDesC16Array7AppendLERK7TDesC16 -- copies the descriptor.
	iItems->AppendL(*row);
	CleanupStack::PopAndDestroy(row);
	}

void CSymRsList::CommitL()
	{
	if (!iBox || !iItems)
		{
		User::Leave(KErrNotReady);
		}
	// eikcoctl.dso 00000088 _ZN11CEikListBox19HandleItemAdditionLEv -- rebuilds the view
	// from the model and updates the scroll bars.
	iBox->HandleItemAdditionL();
	// The old current item may now be past the end. HandleItemAdditionL is not documented
	// to clamp it, so clamp it here rather than leave the question open.
	const TInt count = iItems->Count();
	if (count > 0)
		{
		const TInt current = iBox->CurrentItemIndex();
		if (current < 0)
			{
			iBox->SetCurrentItemIndex(0);
			}
		else if (current >= count)
			{
			iBox->SetCurrentItemIndex(count - 1);
			}
		}
	iBox->DrawDeferred();
	}

TInt CSymRsList::Count() const
	{
	return iItems ? iItems->Count() : 0; // CArrayFixBase::Count is inline (e32base.inl:130)
	}

TInt CSymRsList::Selected() const
	{
	// eikcoctl.dso 00001204 _ZNK11CEikListBox16CurrentItemIndexEv -- non-leaving.
	return iBox ? iBox->CurrentItemIndex() : KErrNotFound;
	}

TInt CSymRsList::SetSelected(TInt aIndex)
	{
	if (!iBox || !iItems || aIndex < 0 || aIndex >= iItems->Count())
		{
		return KErrArgument;
		}
	// eikcoctl.dso 00001248 _ZNK11CEikListBox26SetCurrentItemIndexAndDrawEi -- non-leaving
	// and const, which is why moving the highlight needs no TRAP.
	iBox->SetCurrentItemIndexAndDraw(aIndex);
	return KErrNone;
	}

void CSymRsList::HandleListBoxEventL(CEikListBox* aListBox, TListBoxEvent aEventType)
	{
	// eiklbo.h: TListBoxEvent has no explicit values, so EEventEnterKeyPressed = 0 and
	// EEventItemClicked = 1. Those two are "the user chose this item" -- the selection key
	// and a tap. The editing and dragging events mean nothing to a read-only list.
	if (aEventType != EEventEnterKeyPressed && aEventType != EEventItemClicked)
		{
		return;
		}
	if (!aListBox || !iCallbacks || !iCallbacks->selected)
		{
		return;
		}
	const TInt index = aListBox->CurrentItemIndex();
	if (index < 0)
		{
		return;
		}
	// The Rust frame runs with no harness open around it. Its error becomes a leave only
	// after it has returned, on a stack that is pure C++ again (spec section 4.3).
	const TInt err = iCallbacks->selected(iOwner, index);
	User::LeaveIfError(err);
	}

// --------------------------------------------------------------------------
// The C entry points. Each one is a complete TRAP unit or is non-leaving.
// --------------------------------------------------------------------------

static CSymRsList* List(void* aList)
	{
	return static_cast<CSymRsList*>(aList);
	}

extern "C" TInt symrs_list_create(void* aAppUi, const SymRsListCallbacks* aCallbacks,
	void* aOwner, void** aOut)
	{
	if (!aAppUi || !aOut)
		{
		return KErrArgument;
		}
	if (!aCallbacks || aCallbacks->iSize < sizeof(SymRsListCallbacks))
		{
		// A Rust SDK older than this shim. Loud, and at startup, like the vtable check.
		return KErrNotSupported;
		}
	*aOut = NULL;
	// The cast is from the `void*` symrs_avkon.cpp passed to the Rust `construct`:
	// `this` of CShimAppUi, whose primary base is CAknAppUi.
	//
	// ClientRect(), and deliberately not the view's Rect(). aknlists.h lines 208-210
	// require it of every style in this family ("the Rect() of the list must be
	// ClientRect()"), and the view cannot supply it: a window-owning control's Rect()
	// is window-relative, so CShimView -- itself constructed with ClientRect() --
	// reports an origin of (0,0), and the first run of this shim duly drew the list
	// over the title pane. CEikAppUi::ClientRect() is the area below the status pane
	// and above the softkeys, in the coordinates SetRect wants.
	CAknAppUi* appUi = static_cast<CAknAppUi*>(aAppUi);
	const TRect rect = appUi->ClientRect();
	CSymRsList* list = NULL;
	TRAPD(err, list = CSymRsList::NewL(appUi, rect, aCallbacks, aOwner));
	if (err != KErrNone)
		{
		return err;
		}
	*aOut = list;
	return KErrNone;
	}

extern "C" void symrs_list_destroy(void* aList)
	{
	delete List(aList);
	}

extern "C" void symrs_list_clear(void* aList)
	{
	if (aList)
		{
		List(aList)->Clear();
		}
	}

extern "C" TInt symrs_list_add(void* aList, const TUint16* aText, TInt aLength)
	{
	if (!aList)
		{
		return KErrArgument;
		}
	TRAPD(err, List(aList)->AddL(aText, aLength));
	return err;
	}

extern "C" TInt symrs_list_commit(void* aList)
	{
	if (!aList)
		{
		return KErrArgument;
		}
	TRAPD(err, List(aList)->CommitL());
	return err;
	}

extern "C" TInt symrs_list_count(void* aList)
	{
	return aList ? List(aList)->Count() : 0;
	}

extern "C" TInt symrs_list_selected(void* aList)
	{
	return aList ? List(aList)->Selected() : KErrNotFound;
	}

extern "C" TInt symrs_list_set_selected(void* aList, TInt aIndex)
	{
	return aList ? List(aList)->SetSelected(aIndex) : KErrArgument;
	}
