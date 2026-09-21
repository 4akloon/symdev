# WIP: S60 list box for symbian-ui (branch `ui-list`)

Task: add an S60 Avkon list box to the symdev Rust SDK — `symbian-ui/src/list.rs` +
`shims/s60/symrs_list.cpp` — with a safe `List::new/set_items/selected/on_select`
surface, proven in the emulator with arrow keys and Return.

## Findings

- Spec §3: ABI is shape B — two `#[repr(C)]` tables, `SymRsHost` (down) and `SymRsAppVtbl`
  (up), each with a leading `u32 size`; single exported symbol `symrs_app_vtbl()`.
- Spec §4.3: every host entry that can leave is `TRAP`ped **inside the shim**; Rust never
  has a frame on the stack when a leave starts. `Draw` is called OUTSIDE a trap harness,
  is `const`, and nothing reachable from it may leave — by construction, not by TRAP.
- Spec §5.2: keys — `EKeyUpArrow` 0xf809, `EKeyDownArrow` 0xf80a, `EKeyDevice3` (select)
  0xf845; `EEventKey = 1`; `EKeyWasNotConsumed = 0` / `EKeyWasConsumed = 1`.
- exp 83 / eka2l1-input.md: XTEST is dropped on this host; `XSendEvent` to the emulator
  TOPLEVEL works. Driver is `docs/research/acceptance/emukey.py {keys,shot,focus} <pid>`.
  Arrows + Return verified; **F1/F2 softkeys are dead in our own apps — do not test them.**
- exp 86: existing view is stacked with `SetMopParent(this); ConstructL(ClientRect());
  AddToStackL(iView)`. Client area measured 240x245. `uidemo.exe` = 12 715 bytes.
- exp 86: link needs `-u symrs_app_vtbl`; `-l:euser.dso -l:drtaeabi.dso` must stay BEFORE
  the Rust archive (that ordering is the whole reason the E32 is 12 KB and not 107 KB).

### From the SDK, 2026-09-21

- **Class: `CAknSingleStyleListBox`** (`aknlists.h:224`) — `list_single_pane`, the
  simplest style that shows only text. Header comment at `aknlists.h:218-222`:
  `list item string format: "\tTextLabel\t0\t1"` / `where 0 and 1 are indexes to icon
  array`. So a row is TAB-separated columns: **[0] icon-ish column (empty for
  single_pane), [1] the text, [2] and [3] icon-array indices.** With no icon array set
  we emit `"\t" + text` only; columns 2/3 index an array that does not exist. To be
  CONFIRMED in pixels, not assumed.
- `aknlists.h:208-210`: "These are only for full screen lists -- the Rect() of the list
  must be ClientRect()". Matches the view's own rect.
- **Ownership:** `CTextListBoxModel::SetItemTextArray(MDesCArray*)` "Panics if NULL" and
  only **assigns** — it does not delete the previous array. `SetOwnershipType` takes
  `TListBoxModelItemArrayOwnership` (`eiklbm.h:26`): `ELbmOwnsItemArray = 0`,
  `ELbmDoesNotOwnItemArray = 1`. Decision below.
- **Observer:** `MEikListBoxObserver::HandleListBoxEventL(CEikListBox*, TListBoxEvent)`
  is pure virtual (`eiklbo.h`). `TListBoxEvent` has no explicit values, so
  `EEventEnterKeyPressed = 0`, `EEventItemClicked = 1`, `EEventItemDoubleClicked = 2`,
  `EEventItemActioned = 3`, `EEventEditingStarted = 4`, `EEventEditingStopped = 5`,
  `EEventPenDownOnItem = 6`, `EEventItemDraggingActioned = 7`.
- **Scrollbars:** `CEikListBox::CreateScrollBarFrameL(TBool aPreAlloc=EFalse)` then
  `CEikScrollBarFrame::SetScrollBarVisibilityL(TScrollBarVisibility, TScrollBarVisibility)`
  with `EOff = 0`, `EOn = 1`, `EAuto = 2` (`eiksbfrm.h:107`).
- **Key priority is DOCUMENTED, not insertion order.** `coeaui.h:36-38`: "Controls with
  higher priorities get offered key events before controls with lower priorities."
  `ECoeStackPriorityDefault = 0` (`coeaui.h:47`), which is what `symrs_avkon.cpp` uses
  for `CShimView`. So the list goes on at `ECoeStackPriorityDefault + 1` and wins by a
  documented rule.
- **`CEikTextListBox::ConstructL(const CCoeControl* aParent, TInt aFlags = 0)` is public**
  (`eiktxlbx.h:68`, inside the `public:` at line 33) even though
  `CEikListBox::ConstructL` is protected. `CEikTextListBox::Model()` is public and
  returns `CTextListBoxModel*`.

### Mangled names, `nm -D` on `epoc32/release/armv5/lib/`

- `avkon.dso`: `_ZN22CAknSingleStyleListBoxC1Ev`,
  `_ZN23AknListBoxLinesTemplateI17CAknColumnListBoxE11SizeChangedEv`,
  `_ZTV22CAknSingleStyleListBox`.
- `eikcoctl.dso`: `_ZN15CEikTextListBox10ConstructLEPK11CCoeControli`,
  `_ZNK15CEikTextListBox5ModelEv`,
  `_ZN17CTextListBoxModel16SetItemTextArrayEP12MDesC16Array`,
  `_ZN17CTextListBoxModel16SetOwnershipTypeE31TListBoxModelItemArrayOwnership`,
  `_ZN11CEikListBox21CreateScrollBarFrameLEi`,
  `_ZN18CEikScrollBarFrame23SetScrollBarVisibilityLENS_20TScrollBarVisibilityES0_`,
  `_ZN11CEikListBox18SetListBoxObserverEP19MEikListBoxObserver`,
  `_ZNK11CEikListBox16CurrentItemIndexEv`,
  `_ZNK11CEikListBox26SetCurrentItemIndexAndDrawEi`,
  `_ZN11CEikListBox19HandleItemAdditionLEv`.
- `bafl.dso`: `_ZN16CDesC16ArrayFlatC1Ei`, `_ZN12CDesC16Array7AppendLERK7TDesC16`,
  `_ZN12CDesC16Array5ResetEv`. (`badesca.h:237,246`: `CDesCArray` is `CDesC16Array`,
  `CDesCArrayFlat` is `CDesC16ArrayFlat`.)
- **So `UI_LIBRARIES` needs `eikcoctl.dso` and `bafl.dso` added** — today it is
  `apparc cone eikcore avkon gdi` (`crates/symdev-build/src/rust_sdk.rs:52`). That is an
  edit OUTSIDE my lane; flag it.

## Decisions

- Use `CAknSingleStyleListBox`: only style in the single-line family that needs no icon
  array (`list_single_pane`); `CAknSingleGraphicStyleListBox` etc. all put an icon index
  in column 0 and would need a `CAknIconArray` we have nothing to put in.
- **The shim owns the item array.** `CDesCArrayFlat` allocated by the shim,
  `SetItemTextArray(iItems)` + `SetOwnershipType(ELbmDoesNotOwnItemArray)` once; the shim
  deletes it in its own destructor. `set_items` does `Reset()` + `AppendL` into the SAME
  array, so no ownership ever changes hands and `SetItemTextArray`'s non-deleting
  assignment can never leak or double-free.
- **The list is its own stacked control, not a child of `CShimView`.** `CShimView`
  returns 0 from `CountComponentControls` and does not forward it to Rust, so a child
  would need a change to `symrs_avkon.cpp`. A window-owning list added to the app UI's
  own control stack needs NOTHING from that file.
- The list goes on the stack at `ECoeStackPriorityDefault + 1` so it is offered keys
  before `CShimView` by the documented priority rule.

- A full-screen list COVERS the application's view, so a selection cannot be shown by
  drawing in `App::draw` — the only visible surface is the list itself. That is why
  `on_select` is `FnMut(usize, &mut Rows)` and not `FnMut(usize)`: the callback has to
  be able to rewrite the thing that owns it, and `Rows` borrows only the control handle,
  never the closure beside it, so no `Rc`/`Weak` cycle is needed in the application.
- `selected_thunk` MOVES the closure out of the `Owner` for the length of the call and
  puts it back. Whether anything a callback does can make the list report a second event
  before the first returns was never observed; this makes a nested call a no-op instead
  of a second `&mut` to the same box.
- `bafl.dso` was already on every Rust link line through `RustSdk::LIBRARIES`; only
  `eikcoctl.dso` had to be added, and it went into `UI_LIBRARIES`.

## Dead ends

## Next step

- DONE: shim, `list/{mod,rows}.rs`, `eikcoctl.dso`. Host + symbian-rs gates clean.
- NEXT: `symbian-rs/examples/ui-list` (own example, not `examples/ui`, which three other
  agents also touch), uid3 0xe0000696, then build/package/run and the emukey acceptance.
