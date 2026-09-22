// symrs_avkon.h -- the ABI between the Avkon subclasses in symrs_avkon.cpp and the Rust
// application in crates/symbian-ui (design: docs/research/avkon-rust-spec.md section 3).
//
// Two plain C structs and two sets of plain `extern "C"` functions, resolved by the
// linker (experiment 104; until then these were two tables of function pointers with a
// `size` word each). `symrs_gc_*`, `symrs_view_redraw`, `symrs_app_ui_exit` and
// `symrs_menu_add` are what Rust may ask of the framework, DEFINED here; `symrs_app_*`
// are what the framework calls on the Rust application object, defined in the
// application crate by `#[symbian_std::main(gui)]`. A function one side expects and the
// other lacks is an undefined symbol at link time -- the loud failure the tables' size
// words gave at startup, a step earlier.
//
// The Rust side declares the same structs and functions in
// crates/symbian-ui/src/abi.rs and vtbl.rs; the two declarations are kept in step by
// hand, because a bindgen would be a second toolchain.
//
// THE LEAVE RULE, which is the part most easily got wrong (spec section 4.3):
//
//   * Every "down" function is a complete TRAP unit or is non-leaving by
//     construction. Nothing throws while a Rust frame is on the stack -- rustc marks
//     the whole Rust text `cantunwind`, so a C++ exception reaching it ends the process
//     with no diagnostic at all (experiment 76).
//   * A Rust callback that must fail returns a TInt. The shim calls
//     `User::LeaveIfError` on it AFTER the Rust frame has returned, on a stack that is
//     pure C++ again. Rust itself can never leave.
//   * `draw` and `size_changed` are called outside any trap harness. Nothing on either
//     side of them may leave, and no TRAP is written there either: a TRAP inside `Draw`
//     would swallow an error nobody can report.
#ifndef SYMRS_AVKON_H
#define SYMRS_AVKON_H

#include <e32def.h>

extern "C" {

// One-for-one with TRect, as four TInts rather than two TPoints.
typedef struct SymRsRect
	{
	TInt x;
	TInt y;
	TInt w;
	TInt h;
	} SymRsRect;

// A key crosses as the framework's own TKeyEvent, by pointer: w32std.h line 974
// declares it as exactly four words (iCode, iScanCode, iModifiers, iRepeats), which is
// the layout crates/symbian-ui/src/abi.rs mirrors. Passing it through rather than
// copying it into a struct of our own is experiment 104.
struct TKeyEvent;

// "Down": what Rust may ask of the framework, defined in symrs_avkon.cpp. `aGc` is the
// CWindowGc the framework handed Draw and is valid only for that call; `aView` and
// `aAppUi` live as long as the app UI. Colours cross as 0x00RRGGBB and are unpacked into
// TRgb(r, g, b) by the shim, so no assumption is made about TRgb's internal word.
void symrs_gc_clear(void* aGc, SymRsRect aRect);
void symrs_gc_set_pen(void* aGc, TUint32 aRgb);
void symrs_gc_set_brush(void* aGc, TUint32 aRgb, TInt aSolid);
void symrs_gc_draw_rect(void* aGc, SymRsRect aRect);
void symrs_gc_draw_line(void* aGc, TInt aX1, TInt aY1, TInt aX2, TInt aY2);
void symrs_gc_draw_text(void* aGc, const TUint16* aText, TInt aLength, TInt aX, TInt aY);
void symrs_view_redraw(void* aView);
void symrs_app_ui_exit(void* aAppUi);
// The one function that is a TRAP unit rather than non-leaving by construction:
// CEikMenuPane::AddMenuItemL leaves on no memory. The error is RETURNED, and the Rust
// side carries it back out of `symrs_app_menu` so that the shim can raise it after the
// Rust frame has gone -- which is the leave rule above, not an exception to it. `aText`
// is UTF-16 and at most CEikMenuPaneItem::SData::ENominalTextLength (40) units long;
// the Rust side cuts it there, and this shim clamps again because the descriptor whose
// invariant it is lives here.
TInt symrs_menu_add(void* aPane, const TUint16* aText, TInt aLength, TInt aCommand);

// "Up": what the framework calls on the Rust application object, defined by
// `#[symbian_std::main(gui)]`. The object is opaque here: Rust allocates it in `create`,
// frees it in `destroy`, and C++ never dereferences or deletes it. The link line names
// `symrs_app_create` with `-u` so the Rust archive's member -- which holds all eight --
// is pulled before this archive is reached.
void* symrs_app_create(void);
void symrs_app_destroy(void* aApp);
TInt symrs_app_construct(void* aApp, void* aView, void* aAppUi);
void symrs_app_draw(void* aApp, void* aGc, SymRsRect aArea);
TInt symrs_app_offer_key(void* aApp, const TKeyEvent* aEvent, TInt aType);
TInt symrs_app_command(void* aApp, TInt aCommand);
void symrs_app_size_changed(void* aApp, SymRsRect aArea);
// DynInitMenuPaneL: the Options menu is about to be shown, so fill it. `aPane` is the
// CEikMenuPane* and is valid for this call only.
TInt symrs_app_menu(void* aApp, void* aPane);

}

#endif // SYMRS_AVKON_H
