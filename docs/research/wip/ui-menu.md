# ui-menu

Step 91 of the Rust SDK: the S60 **Options menu** (`R_AVKON_MENUBAR`/`R_AVKON_MENUPANE` from the
`[ui]` manifest) and **working softkeys** — left softkey opens the menu, right softkey exits.

## Findings

- The CBA **exists** in our applications: `corpus/86-ui/uidemo-before.png` and a fresh run both
  show `Exit` drawn on the right softkey, so `EIK_APP_INFO { cba = R_AVKON_SOFTKEYS_EXIT; }` from
  our generated `.rss` was read and applied by `CEikAppUi::BaseConstructL`.
- `ECoeStackPriorityCba = 60` vs our view's `ECoeStackPriorityDefault = 0` (`coeaui.h:47-61`), so
  the CBA sees a key **before** our view: "F2 never reaches `OfferKeyEventL`" is expected, not a
  symptom.
- `RDebug::Print` (`_ZN6RDebug5PrintE11TRefByValueIK7TDesC16Ez`, euser.dso) reaches EKA2L1's
  `debug_print` executive, but the default log filter in `~/.local/share/EKA2L1/config.yml` has
  **`Emulated.Stdout:off`** — that is why a guest print looks like nothing happened. Set it to
  `Emulated.Stdout:trace` to see guest prints (backup at scratchpad/config.yml.bak; restore at the
  end).

- **SOFTKEYS WERE NEVER DEAD.** With `Emulated.Stdout` on, a probe in `CShimAppUi::HandleWsEventL`
  and `HandleCommandL` shows F2 arriving as `type=3 scan=a5`, `type=1 code=f843 scan=a5`, the CBA
  consuming the `EEventKey` (our view never sees it, because `ECoeStackPriorityCba`=60 > 0) and
  **`HandleCommandL 3001`** being called. The command simply never matched: the shim compares
  against `EEikCmdExit` (0x100) and `3002`.
- **`3002` is not `EAknSoftkeyExit`.** Read from `avkon.hrh:330-339`: `EAknSoftkeyOptions=3000`,
  `Back=3001`, `Mark=3002`, `Unmark`, `Insert`, `Yes`, `No`, `Done`, `Close`, **`Exit=3009`**. The
  spec's §5 table (`EAknSoftkeyExit = 3002`) was recalled, not read, and 3002 is `EAknSoftkeyMark`.
- **And 3009 would not have matched either**: the ROM's `R_AVKON_SOFTKEYS_EXIT` (`0x8cc002a` from
  the SDK's `avkon.rsg`; the ROM's `avkon.r01` does carry offset `0x8cc0`) delivers **3001**
  (`EAknSoftkeyBack`) while drawing `Exit`. Observed, not explained — the ROM's `avkon.rsc` is
  dictionary-compressed and was not decoded. The lesson is the same either way: **do not depend on
  the command ids inside a ROM CBA resource.**
- **`R_AVKON_SOFTKEYS_OPTIONS_EXIT` without a `menubar` crashes the application**: F1 gives
  `Access violation reading address 0x9C in thread Bars`, and no `HandleCommandL` is called first —
  so `EAknSoftkeyOptions` is intercepted below `HandleCommandL` and used to display the menu bar.
  The Options softkey and the menu resource are one feature and must ship together.

## Decisions

- Declare **our own `CBA`, `MENU_BAR` and `MENU_PANE`** in the generated `.rss` with command ids we
  choose, instead of naming a ROM resource. Left softkey must carry `EAknSoftkeyOptions` (3000, from
  `avkon.hrh`) because that is what makes the framework open the menu bar; the exit item carries
  `EEikCmdExit` (0x100, `eikon.hrh:376`), which the shim already handles.

## Dead ends

## Next step

- Read `docs/research/avkon-rust-spec.md`, `docs/research/eka2l1-input.md`, experiments 83 and
  86, then `symbian-ui/`, `shims/s60/symrs_avkon.cpp`, `ui_resources/`, `symdev-manifest/src/ui.rs`.
