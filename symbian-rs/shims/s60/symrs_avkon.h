// symrs_avkon.h -- the ABI between the Avkon subclasses in symrs_avkon.cpp and the Rust
// application in crates/symbian-ui (design: docs/research/avkon-rust-spec.md section 3).
//
// Two plain C structs, both in .rodata, both word-aligned, nothing in them wider than a
// pointer. `SymRsHost` is what Rust may ask of the framework; `SymRsAppVtbl` is what the
// framework calls on the Rust application object. Both start with a `size` word holding
// `sizeof` of the struct as the side that wrote it knows it, so a Rust SDK older than
// this shim fails loudly at startup instead of having its table read off the end. Every
// field added later goes on the end.
//
// The Rust side declares the same two structs with `#[repr(C)]` in
// crates/symbian-ui/src/abi.rs; the two declarations are kept in step by hand, because
// they are eight fields each and a bindgen would be a second toolchain.
//
// THE LEAVE RULE, which is the part most easily got wrong (spec section 4.3):
//
//   * Every `SymRsHost` entry is a complete TRAP unit or is non-leaving by
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

// One-for-one with TRect (as four TInts, not two TPoints) and with TKeyEvent, which
// w32std.h line 974 declares as exactly these four words.
typedef struct SymRsRect
	{
	TInt x;
	TInt y;
	TInt w;
	TInt h;
	} SymRsRect;

typedef struct SymRsKeyEvent
	{
	TUint iCode;
	TInt iScanCode;
	TUint iModifiers;
	TInt iRepeats;
	} SymRsKeyEvent;

// "Down": what Rust may ask of the framework. `aGc` is the CWindowGc the framework
// handed Draw and is valid only for that call; `aView` and `aAppUi` live as long as the
// app UI. Colours cross as 0x00RRGGBB and are unpacked into TRgb(r, g, b) by the shim,
// so no assumption is made about TRgb's internal word.
typedef struct SymRsHost
	{
	TUint32 iSize;
	void (*clear)(void* aGc, SymRsRect aRect);
	void (*set_pen)(void* aGc, TUint32 aRgb);
	void (*set_brush)(void* aGc, TUint32 aRgb, TInt aSolid);
	void (*draw_rect)(void* aGc, SymRsRect aRect);
	void (*draw_line)(void* aGc, TInt aX1, TInt aY1, TInt aX2, TInt aY2);
	void (*draw_text)(void* aGc, const TUint16* aText, TInt aLength, TInt aX, TInt aY);
	void (*redraw)(void* aView);
	void (*exit)(void* aAppUi);
	// The one entry that is a TRAP unit rather than non-leaving by construction:
	// CEikMenuPane::AddMenuItemL leaves on no memory. The error is RETURNED, and the
	// Rust side carries it back out of `menu` so that the shim can raise it after the
	// Rust frame has gone -- which is the leave rule above, not an exception to it.
	// `aText` is UTF-16 and at most CEikMenuPaneItem::SData::ENominalTextLength (40)
	// units long; the Rust side cuts it there, and this shim clamps again because the
	// descriptor whose invariant it is lives here.
	TInt (*menu_item)(void* aPane, const TUint16* aText, TInt aLength, TInt aCommand);
	} SymRsHost;

// "Up": what the framework calls on the Rust application object. The object is opaque
// here: Rust allocates it in `create`, frees it in `destroy`, and C++ never dereferences
// or deletes it.
typedef struct SymRsAppVtbl
	{
	TUint32 iSize;
	void* (*create)(void);
	void (*destroy)(void* aApp);
	TInt (*construct)(void* aApp, const SymRsHost* aHost, void* aView, void* aAppUi);
	void (*draw)(void* aApp, void* aGc, SymRsRect aArea);
	TInt (*offer_key)(void* aApp, const SymRsKeyEvent* aEvent, TInt aType);
	TInt (*command)(void* aApp, TInt aCommand);
	void (*size_changed)(void* aApp, SymRsRect aArea);
	// DynInitMenuPaneL: the Options menu is about to be shown, so fill it. `aPane` is
	// the CEikMenuPane* and is valid for this call only.
	TInt (*menu)(void* aApp, void* aPane);
	} SymRsAppVtbl;

// The single symbol this shim imports from the Rust side. Written by
// `#[symbian_std::main(gui)]`; the link line names it with `-u symrs_app_vtbl` so the
// Rust archive is searched for it before this archive is reached. It may be called more
// than once and must return the same pointer.
const SymRsAppVtbl* symrs_app_vtbl(void);

}

#endif // SYMRS_AVKON_H
