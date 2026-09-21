// symrs_note.cpp -- the Avkon notes: the popups an application tells the user
// something with, instead of User::InfoPrint (which is a server round trip that draws
// a system dialog of its own). Compiled only for a project whose manifest has a `[ui]`
// section, like the rest of shims/s60.
//
// The Rust side is crates/symbian-ui/src/note.rs, which redeclares the one entry point
// below by hand. There is no header here on purpose: nothing but this translation unit
// and that one `extern "C"` block ever names it.
//
// ---------------------------------------------------------------------------
// WHAT THE SDK DECLARES
// ---------------------------------------------------------------------------
//
// aknnotewrappers.h declares exactly these, and nothing else of this family:
//
//   TAknNoteResData         a value class read out of a resource
//   CAknNoteWrapper         : CAknNoteDialog -- ExecuteLD(TInt aResId)
//                                               ExecuteLD(TInt aResId, const TDesC&)
//   CAknResourceNoteDialog  : CAknNoteWrapper -- holds the resource id, so it has
//                                               ExecuteLD() and ExecuteLD(const TDesC&)
//   CAknConfirmationNote    : CAknResourceNoteDialog -- R_AKN_CONFIRMATION_NOTE
//   CAknInformationNote     : CAknResourceNoteDialog -- R_AKN_INFORMATION_NOTE
//   CAknWarningNote         : CAknResourceNoteDialog -- R_AKN_WARNING_NOTE
//   CAknErrorNote           : CAknResourceNoteDialog -- R_AKN_ERROR_NOTE
//
// Each concrete class has three constructors: the default one, `(TBool aWaitingDialog)`
// -- which picks the R_AKN_..._NOTE_WAIT resource instead -- and `(T** aSelfPtr)` for a
// non-modal note that NULLs a client's pointer when it goes away. Only the default one
// is used here; the other two are wrapped when something is observed to need them.
//
// Every symbol below was read from `nm -D --defined-only avkon.dso`:
//
//   _ZN19CAknInformationNoteC1Ev                        CAknInformationNote::CAknInformationNote()
//   _ZN20CAknConfirmationNoteC1Ev                       CAknConfirmationNote::CAknConfirmationNote()
//   _ZN15CAknWarningNoteC1Ev                            CAknWarningNote::CAknWarningNote()
//   _ZN13CAknErrorNoteC1Ev                              CAknErrorNote::CAknErrorNote()
//   _ZN22CAknResourceNoteDialog9ExecuteLDERK7TDesC16    CAknResourceNoteDialog::ExecuteLD(const TDesC16&)
//
// ExecuteLD is exported by the BASE class, not by the four concrete ones, which is why
// the pointer below is a CAknResourceNoteDialog*. No new import library is needed:
// avkon.dso is already on a [ui] project's link line (RustSdk::UI_LIBRARIES).
//
// CAknGlobalNote (aknglobalnote.h) is deliberately NOT here. It is a different thing:
// a note the notifier server draws, which outlives the application that asked for it,
// is built with NewL/NewLC and shown with ShowNoteL(TAknGlobalNoteType, const TDesC&).
// It belongs to whoever has a use for a note that survives its application; wrapping it
// on the way past would be inventing a requirement.
//
// ---------------------------------------------------------------------------
// OWNERSHIP UNDER THE TRAP -- READ THIS BEFORE CHANGING ShowL
// ---------------------------------------------------------------------------
//
// ExecuteLD both leaves and deletes the object it is called on. eikdialg.h says of
// CEikDialog::ExecuteLD: "This function loads the specified dialog from a resource and
// displays it. The method then destroys the dialog when it exits, therefore there is no
// need for the application program to destroy the dialog." It says NOTHING about the
// leave path, and neither does aknnotewrappers.h. **Not observed.**
//
// So the safe reading is taken and written down rather than guessed at silently: the
// object is assumed to have destroyed itself on EVERY exit path, including a leave, and
// this file never deletes a note and never pushes one on the cleanup stack. If that
// reading is wrong, one dialog leaks on a path that only runs when the note could not be
// shown at all. The other choice, deleting after the TRAP, would be a double delete if
// the reading is right -- a corrupted heap instead of a leaked one.
#include <e32base.h>
#include <e32def.h>
#include <e32std.h>

#include <aknnotewrappers.h>
#include <eikenv.h>

// As shims/common/symrs_shim.h: hidden visibility keeps the symbol out of the E32's
// dynamic table, so --gc-sections can drop this whole object -- and with it the four
// avkon imports -- from an application that never shows a note.
#define SYMRS_EXPORT extern "C" __attribute__((visibility("hidden")))

// One-for-one with `Note` in crates/symbian-ui/src/note.rs. The two declarations are
// kept in step by hand; an unknown value is KErrArgument rather than a default note.
enum TSymRsNoteKind
	{
	ESymRsNoteInformation = 0,
	ESymRsNoteConfirmation = 1,
	ESymRsNoteWarning = 2,
	ESymRsNoteError = 3
	};

// Everything that can leave, in one place, with no Rust frame anywhere on the stack.
// The note is never pushed on the cleanup stack: see the ownership note above.
static void ShowL(TInt aKind, const TDesC16& aText)
	{
	CAknResourceNoteDialog* note = NULL;
	switch (aKind)
		{
		case ESymRsNoteInformation:
			note = new (ELeave) CAknInformationNote();
			break;
		case ESymRsNoteConfirmation:
			note = new (ELeave) CAknConfirmationNote();
			break;
		case ESymRsNoteWarning:
			note = new (ELeave) CAknWarningNote();
			break;
		case ESymRsNoteError:
			note = new (ELeave) CAknErrorNote();
			break;
		default:
			User::Leave(KErrArgument);
			return;
		}
	// From here the note owns itself. The TInt ExecuteLD returns is the id of the
	// button that dismissed it, and zero for a note that does not wait -- which is
	// every one of the four above, because the R_AKN_..._NOTE resources do not carry
	// EEikDialogFlagWait. It is dropped rather than handed back, because a note has
	// one button and nothing to decide; a dialog that asks a question is a query.
	note->ExecuteLD(aText);
	}

// Shows one Avkon note and returns KErrNone, the leave code, KErrArgument for a bad
// argument, or KErrNotReady when there is no application environment to draw into.
//
// A complete TRAP unit, per rule 1 of shims/common/symrs_shim.h: ExecuteLD leaves, a
// leave is a real C++ exception here, and rustc marks the whole Rust text `cantunwind`,
// so an exception that reached the caller would end the process with no diagnostic at
// all (experiment 76).
SYMRS_EXPORT TInt symrs_note_show(TInt aKind, const TUint16* aText, TInt aLength)
	{
	if (!aText || aLength < 0)
		{
		return KErrArgument;
		}
	// A note is a CEikDialog, so it needs the application's environment: it reads the
	// Avkon resource file through it and puts itself on CONE's control stack. Checked
	// rather than trusted, because the failure without the check is not a leave the
	// TRAP would catch -- CEikonEnv::Static() returning NULL would be dereferenced
	// somewhere inside avkon. KErrNotReady is what the Rust side turns into an Err.
	if (!CEikonEnv::Static())
		{
		return KErrNotReady;
		}
	TPtrC16 text(aText, aLength);
	TInt err = KErrNone;
	TRAP(err, ShowL(aKind, text));
	return err;
	}
