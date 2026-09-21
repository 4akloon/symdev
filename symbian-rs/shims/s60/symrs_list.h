// symrs_list.h -- the ABI between the Avkon list box in symrs_list.cpp and the `List`
// of crates/symbian-ui/src/list.rs.
//
// This is a SECOND, smaller boundary beside symrs_avkon.h's, and it is shaped the other
// way round on purpose.
//
//   * symrs_avkon.h is a table Rust exports and the shim calls, because the framework
//     drives the application: the shim needs `symrs_app_vtbl` at link time and the link
//     line names it with `-u`.
//   * A list box is the other direction. Rust builds one when it wants one, so what
//     crosses is a set of plain `extern "C"` functions the shim DEFINES and Rust calls,
//     and they resolve on the first pass because the shim archive follows the Rust one.
//     The single callback that runs the other way -- "an item was chosen" -- is a
//     function POINTER Rust hands over at create time, in `SymRsListCallbacks`, so it
//     costs the link line nothing at all (no second `-u`, experiment 86).
//
// The callback table carries the same `iSize` word symrs_avkon.h's two tables carry, and
// `symrs_list_create` refuses a table shorter than this declaration.
//
// THE LEAVE RULE (avkon-rust-spec.md section 4.3), which is the same here as there:
//
//   * Every `symrs_list_*` entry below is a complete TRAP unit or is non-leaving by
//     construction, so nothing throws while a Rust frame is on the stack. Each returns
//     a TInt: `KErrNone` or a negative error, never a leave.
//   * `selected`, the one Rust callback, is called from inside the list's own
//     `OfferKeyEventL` -- which the framework has already trapped. Its TInt becomes a
//     leave through `User::LeaveIfError` AFTER the Rust frame has returned, on a stack
//     that is pure C++ again.
#ifndef SYMRS_LIST_H
#define SYMRS_LIST_H

#include <e32def.h>

extern "C" {

// What the shim calls on the Rust side. `aOwner` is opaque here: Rust allocates it
// before `symrs_list_create` and frees it after `symrs_list_destroy`, and C++ never
// dereferences or deletes it.
typedef struct SymRsListCallbacks
	{
	TUint32 iSize;
	// An item was chosen -- the selection key, or a tap. `aIndex` is the list's current
	// item. Returns KErrNone, or a negative error that becomes a leave once this call
	// has returned.
	TInt (*selected)(void* aOwner, TInt aIndex);
	} SymRsListCallbacks;

// Builds a CAknSingleStyleListBox over `aAppUi`'s CLIENT RECT, puts it on that app UI's
// control stack above the application's view, and writes the opaque handle to `*aOut`.
// `aAppUi` is the handle symrs_avkon.cpp passed to the Rust `construct`.
//
// The rect comes from CEikAppUi::ClientRect() and NOT from the view's own Rect(): a
// window-owning control's Rect() is window-relative, so the view reports (0,0) for its
// origin and a list built from it covers the title pane. Observed, 2026-09-21 -- the
// first run put the list over the status pane, and it is the same ambiguity
// avkon-rust-spec.md section 5.1 left open for Draw.
TInt symrs_list_create(void* aAppUi, const SymRsListCallbacks* aCallbacks,
	void* aOwner, void** aOut);

// Takes the list off the control stack and destroys it, with the item array. Safe on
// NULL. Non-leaving: it is reached from a Rust `Drop`.
void symrs_list_destroy(void* aList);

// Empties the item array. Non-leaving (`CDesC16Array::Reset`). The list is not redrawn
// and is not told until `symrs_list_commit`.
void symrs_list_clear(void* aList);

// Appends one row. `aText` is the label alone: the shim writes the column separators,
// because the row format belongs to the list style and not to the application.
TInt symrs_list_add(void* aList, const TUint16* aText, TInt aLength);

// Tells the list its items changed, clamps the current item into the new range and asks
// for a redraw. Call it once after a run of `symrs_list_clear` / `symrs_list_add`.
TInt symrs_list_commit(void* aList);

// The number of rows. Non-leaving; 0 for a NULL list.
TInt symrs_list_count(void* aList);

// The current item's index, or a negative value when the list is empty. Non-leaving.
TInt symrs_list_selected(void* aList);

// Moves the highlight and redraws. Non-leaving; KErrArgument for an out-of-range index.
TInt symrs_list_set_selected(void* aList, TInt aIndex);

}

#endif // SYMRS_LIST_H
