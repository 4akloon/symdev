# runtime-menu

Task: move the Options menu out of `symdev.toml` into a runtime `fn menu(&self, m: &mut Menu)` API with closures, deleting `Command`/command ids from the application-facing API.

## Findings

## Decisions

## Dead ends

## Next step

Read the current `[[ui.menu]]` path end to end: manifest -> macro -> shim -> resource.

- 2026-09-21 Platform facts re-read in the SDK headers (`LC_ALL=C grep -a`):
  `eikmenup.h:456` `AddMenuItemL(const CEikMenuPaneItem::SData&)` "adds a menu item
  dynamically"; `SData { iCommandId; iCascadeId; iFlags; TBuf<40> iText; TBuf<1>
  iExtraText; }` with `ENominalTextLength = 40`; `eikmobs.h:43`
  `DynInitMenuPaneL(TInt aResourceId, CEikMenuPane*)`.
- `eikon.hrh:221-239` names every `EEikMenuItem*` flag; `eikon.rh:103` `STRUCT MENU_ITEM`
  defaults `command=0`, `cascade=0`, `flags=0`. So a plain item is `iFlags = 0` and
  `iCascadeId = 0` — the resource compiler's own defaults, not a guess.
- Design shape chosen: the action is a **non-capturing closure coerced to `fn(&mut A)`**,
  so `menu(&self, …)` can be called twice — once to fill the pane, once to look the
  action up by index — and the `fn` pointer is copied out before the `&self` borrow ends.
  No `&mut` alias is ever live with the `&self` one.
- The native `rcomp` accepts `RESOURCE MENU_PANE r_symrs_menupane { items = { }; };`
  — an empty `items` array compiles, `symdev build` on `examples/ui` produced
  `uidemo.rsc` with it. Whether `DynInitMenuPaneL` then fires is the emulator question.
- **Observed, EKA2L1 pid 2770028:** `DynInitMenuPaneL` DOES fire for an **empty**
  `MENU_PANE`. F1 opened a pane holding Avkon's own "Show open apps." plus the four
  lines Rust added through `AddMenuItemL` ("More bars", "Fewer bars", "Reset", "Exit").
  `Down` `Return` on "Fewer bars" drew `bars=2 keys=0 cmd=1` — experiment 91's proof,
  through a closure this time. `F1 Down Down Down Return` on the `m.exit("Exit")` line
  ended the application cleanly (no panic, no KERN-EXEC in the log).
- So the resource cannot go away *entirely*: what stays is a `MENU_BAR` naming a
  `MENU_PANE` with no items. Nothing calls `DynInitMenuPaneL` without a pane to show.
- `uidemo.exe` 12 844 -> 13 714 bytes (+870).
- **40-character limit, observed.** Temporarily gave one item a 47-character label
  (`0123456789`×4 + `CUTHERE`), rebuilt and ran: the pane showed the item ellipsised by
  Avkon, the application did **not** panic, and selecting it still fired the right
  action (`bars=3 keys=0 cmd=1`). The exact `encode` function from `menu.rs`, compiled
  for the host: 47 units -> 40; a surrogate pair straddling 40 -> cut at 39, never
  split; 50 units of surrogate pairs -> 40. So the cut is in Rust, on a character
  boundary, and the shim's clamp never has to do anything.
- `symdev package` does **not** rebuild; `symdev build` first, or the sisx carries the
  previous `.exe` (cost me one emulator run).
- **Re-verified experiment 91's failure on this build.** With `has_menu()` forced to
  `false` (so `EAknSoftkeyOptions` on the left button and **no** `menubar` in
  `EIK_APP_INFO`), F1 kills the thread: `Thread Bars terminated ... KERN-EXEC and exit
  code: 3`. So the refusal the manifest used to carry guarded a real failure — which is
  why the two resources are now emitted from one condition instead.
