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
