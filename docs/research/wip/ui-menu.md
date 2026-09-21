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
- **The shim's code was never wrong, only the spec's prose was.** `HandleCommandL` compares
  against the *symbols* `EEikCmdExit` and `EAknSoftkeyExit`, so it was comparing against 0x100 and
  3009 all along. The wrong number lived only in `avkon-rust-spec.md` §5's table.
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

- **It works.** With our own `CBA` + `MENU_BAR` + `MENU_PANE` in the generated `.rss`: F1 opens the
  Options menu (screenshot shows Avkon's "Show open apps." plus our four items), `Down` moves the
  highlight, `Return` (`EStdKeyDevice3`, `scan=a7`) selects and the probe logs
  `HandleCommandL 30226` = `0x7612` = `Command::named("fewer")`, and the drawing becomes
  `bars=2 keys=0 cmd=1`. F2 ends the process.
- `rcomp` resolves **forward** `LLINK` references, so `EIK_APP_INFO` can stay the third resource
  and still name `r_symrs_menubar`/`r_symrs_cba` declared below it.
- There is no `AVKON_MENUBAR`/`AVKON_MENUPANE` struct in `avkon.rh`; the structs are `MENU_BAR`,
  `MENU_TITLE`, `MENU_PANE` and `MENU_ITEM` from `eikon.rh:97-128`, and `CBA`/`CBA_BUTTON` from
  `eikon.rh:335-350` (`CBA_BUTTON.id` is a **WORD**, which is why command ids stay under 0x8000).

## Dead ends

- One run (`scratchpad/out/menu1`) ended at `bars=6 keys=6 cmd=2` for the sequence
  `F1 Down Return`, where a rerun of the identical sequence gave `bars=2 keys=0 cmd=1`. Extra key
  events, not reproduced; watch for it in the acceptance run.

## Next step

- Done: manifest `[[ui.menu]]` + `CommandId`, generated CBA/menu resources, `Command` in
  `symbian-ui`, `examples/ui` with four items. Host gate and `symbian-rs` gate clean, committed
  as `c3fa007`.
- Left: check the other examples' sizes are unchanged, `symdev test --emulator`, the clean
  acceptance run with screenshots into `symbian-rs/corpus/91-ui-menu/`, backlog entry 91, spec
  §5/§11 corrections, restore `~/.local/share/EKA2L1/config.yml` from
  `scratchpad/config.yml.bak`.
