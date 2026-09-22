// symrs_query.cpp -- Avkon's modal query dialogs, behind three C entry points.
//
// A query is the one piece of Avkon that needs no place on the control stack and no
// observer: `CAknQueryDialog::NewL(...)->ExecuteLD(id, prompt)` shows itself, takes over
// input, runs its own loop and returns when the user confirms or cancels. So this file
// forwards nothing and subclasses nothing -- it is shape A of avkon-rust-spec.md 3.1,
// three ordinary `extern "C"` wrappers, and the Rust side in crates/symbian-ui/src/query.rs
// declares them. symrs_avkon.h does not grow a function.
//
// WHY EACH WRAPPER IS HERE, against the three rules of shims/common/symrs_shim.h:
//
//   Rule 1, IT CAN LEAVE. `CAknQueryDialog::NewL` and `CAknQueryDialog::ExecuteLD` both
//   leave, so the TRAP goes around them here. A query is asked for from `key` or
//   `command`, which means a Rust frame is on the stack when we are called, and rustc
//   marks the whole Rust text `cantunwind`: an escaping leave would end the process
//   with no diagnostic at all (experiment 76). Every entry below is a complete TRAP
//   unit and returns a TInt.
//
//   Rule 2, the signature is not a C signature: `ExecuteLD` takes a `const TDesC16&`
//   and the text query writes into a `TDes16&`. Both are built here over the caller's
//   own u16 buffer, so no descriptor crosses the boundary.
//
// OWNERSHIP UNDER THE TRAP -- the part most easily got wrong. `eikdialg.h` documents
// `ExecuteLD` as "loads, displays, and destroys the dialog ... there is no need for the
// application program to destroy the dialog", and says *nothing* about what happens when
// it leaves. The SDK is silent, so this file takes the safe reading and **never deletes
// the dialog after a leave**: `PrepareLC` pushes it on the cleanup stack, so a leave from
// anywhere after that point already destroys it, and a second `delete` would be a
// double free. The cost of the safe reading is that a leave raised between `NewL`
// returning and `PrepareLC` pushing would leak one dialog. That window was not observed
// and is not guessed at; a leaked dialog in an error path is the lesser fault.
//
// The mangled names this file calls, from `nm -D` on
// epoc32/release/armv5/lib/avkon.dso, the way symbian-sys cites its imports:
//
//   _ZN15CAknQueryDialog4NewLER6TDes16RKNS_5TToneE   CAknQueryDialog::NewL(TDes16&, TTone const&)
//   _ZN15CAknQueryDialog4NewLERiRKNS_5TToneE         CAknQueryDialog::NewL(int&, TTone const&)
//   _ZN15CAknQueryDialog9ExecuteLDEiRK7TDesC16       CAknQueryDialog::ExecuteLD(int, TDesC16 const&)
//
// `CAknQueryDialog::NewL` is the one factory for every value type (aknquerydialog.h
// lines 91-133): the overload chosen by the argument returns the right concrete
// subclass -- `CAknTextQueryDialog` for a `TDes16&`, `CAknNumberQueryDialog` for a
// `TInt&` -- so neither concrete class is named here and neither is linked by name.
#include <aknquerydialog.h>
#include <avkon.rsg>
#include <e32base.h>
#include <e32def.h>
#include <e32err.h>
#include <eikenv.h>

// Same rule as shims/common/symrs_shim.h: the shim is an implementation detail of the
// SDK, so hidden visibility keeps these out of the E32's dynamic table and lets
// --gc-sections drop a wrapper -- and avkon.dso's import of it -- from a program that
// asks no questions.
#define SYMRS_EXPORT extern "C" __attribute__((visibility("hidden")))

// What every entry point returns on success. `ExecuteLD` on a waiting dialog returns the
// id of the button that dismissed it, or zero for cancel (eikdialg.h lines 105-122), and
// every Avkon query is a waiting dialog. Rust only needs the distinction, so the button
// id is collapsed here rather than exporting `EEikBidOk` to a Rust constant nobody can
// check.
static const TInt KQueryCancelled = 0;
static const TInt KQueryConfirmed = 1;

// The tone. `ENoTone` and not a confirmation tone: a tone is a decision for the
// application, there is nowhere on this surface to express it yet, and silence is the
// only choice that cannot be wrong.
static const CAknQueryDialog::TTone KTone = CAknQueryDialog::ENoTone;

// ---------------------------------------------------------------------------
// The leaving halves. Each one is called only from inside a TRAP.
// ---------------------------------------------------------------------------

static TInt RunTextQueryL(const TDesC16& aPrompt, TDes16& aText)
	{
	// aText is a TPtr16 over the caller's buffer, so its MaxLength is what bounds the
	// input: CAknTextQueryDialog writes into the descriptor the caller owns, and
	// CAknQueryDialog::MaxTextLength(..., const TDes16&, ...) is how Avkon asks it how
	// much room there is. Nothing here chooses a length.
	CAknQueryDialog* dialog = CAknQueryDialog::NewL(aText, KTone);
	return dialog->ExecuteLD(R_AVKON_DIALOG_QUERY_VALUE_TEXT, aPrompt);
	}

static TInt RunNumberQueryL(const TDesC16& aPrompt, TInt& aValue)
	{
	CAknQueryDialog* dialog = CAknQueryDialog::NewL(aValue, KTone);
	return dialog->ExecuteLD(R_AVKON_DIALOG_QUERY_VALUE_NUMBER, aPrompt);
	}

// ---------------------------------------------------------------------------
// The C entry points.
// ---------------------------------------------------------------------------

// A query is a CONE dialog: it needs the environment CONE installs before any
// application code runs. Asking for one before there is a CEikonEnv is a programming
// error with a name, not a crash.
static TInt CheckEnv()
	{
	return CEikonEnv::Static() ? KErrNone : KErrNotReady;
	}

// A text query. `aPrompt`/`aPromptLength` is the question, `aText`/`aTextMax` is the
// caller's UTF-16 buffer, which the dialog both bounds itself by and writes into, and
// `*aTextLength` is set to how many code units came back.
//
// Returns KQueryConfirmed, KQueryCancelled, or a negative error: KErrArgument for a bad
// argument, KErrNotReady when there is no CEikonEnv yet, or the leave code.
SYMRS_EXPORT TInt symrs_query_text(const TUint16* aPrompt, TInt aPromptLength,
	TUint16* aText, TInt aTextMax, TInt* aTextLength)
	{
	if (!aPrompt || aPromptLength < 0 || !aText || aTextMax <= 0 || !aTextLength)
		{
		return KErrArgument;
		}
	const TInt ready = CheckEnv();
	if (ready != KErrNone)
		{
		return ready;
		}
	TPtrC16 prompt(aPrompt, aPromptLength);
	// Length 0, max aTextMax: the field starts empty and cannot outgrow the caller.
	TPtr16 text(aText, 0, aTextMax);
	TInt button = 0;
	TInt err = KErrNone;
	TRAP(err, button = RunTextQueryL(prompt, text));
	if (err != KErrNone)
		{
		return err;
		}
	// `text` is the descriptor the dialog wrote into, so its length is the answer's.
	*aTextLength = text.Length();
	return button ? KQueryConfirmed : KQueryCancelled;
	}

// A number query. `*aValue` is the value shown when the dialog opens and holds the
// entered number when this returns KQueryConfirmed; it is left alone on a cancel or an
// error. Same return codes as symrs_query_text.
SYMRS_EXPORT TInt symrs_query_number(const TUint16* aPrompt, TInt aPromptLength,
	TInt* aValue)
	{
	if (!aPrompt || aPromptLength < 0 || !aValue)
		{
		return KErrArgument;
		}
	const TInt ready = CheckEnv();
	if (ready != KErrNone)
		{
		return ready;
		}
	TPtrC16 prompt(aPrompt, aPromptLength);
	TInt value = *aValue;
	TInt button = 0;
	TInt err = KErrNone;
	TRAP(err, button = RunNumberQueryL(prompt, value));
	if (err != KErrNone)
		{
		return err;
		}
	if (button)
		{
		*aValue = value;
		}
	return button ? KQueryConfirmed : KQueryCancelled;
	}

// A confirmation query is NOT here, and the reason is a resource and not an oversight.
//
// `ExecuteLD` needs a resource id. The ROM's own `avkon.rsg` carries
// R_AVKON_DIALOG_QUERY_VALUE_TEXT/_NUMBER/_PHONE/_TIME/_DATE/_DURATION (ids 0x8cc0052 to
// 0x8cc0057) and **no confirmation query at all**: `AVKON_CONFIRMATION_QUERY` is only a
// resource STRUCT in `avkon.rh` line 165, a shape an application instantiates in its own
// `.rss`. No `.rsg` anywhere in epoc32/include defines an `R_..._CONFIRMATION_QUERY`.
//
// So an in-application confirmation query needs symdev to generate, into the `<app>.rss`
// that crates/symdev-build/src/ui_resources.rs already writes, something of this shape:
//
//   RESOURCE DIALOG r_symrs_confirmation_query
//       {
//       flags = EGeneralQueryFlags;
//       buttons = R_AVKON_SOFTKEYS_YES_NO;          // 0x8cc0024, in the ROM
//       items =
//           {
//           DLG_LINE { type = EAknCtQuery; id = EGeneralQuery;
//               control = AVKON_CONFIRMATION_QUERY { layout = EConfirmationQueryLayout;
//                   label = ""; }; }
//           };
//       }
//
// and a way for this file to learn the id that `<app>.rsg` gets for it -- which the
// application's resource-file offset may or may not already be folded into. That was not
// observed, and `ui_resources.rs` belongs to another slice this week, so nothing is
// guessed here: no confirmation entry point is exported and `symbian_ui::query` has no
// `confirm`. See docs/research/experiment-backlog.md entry 93.
