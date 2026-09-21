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

## Decisions

## Dead ends

## Next step

- Read `docs/research/avkon-rust-spec.md`, `docs/research/eka2l1-input.md`, experiments 83 and
  86, then `symbian-ui/`, `shims/s60/symrs_avkon.cpp`, `ui_resources/`, `symdev-manifest/src/ui.rs`.
