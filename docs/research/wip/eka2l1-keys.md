# WIP: deliver a key press to the guest in EKA2L1

Task: find out why no host key press reaches the emulated Symbian guest on this host, and
make a reproducible way for an automated test to press a key and observe the effect.

## Findings

- Starting point: `docs/research/avkon-rust-spec.md` §5.2 and §9 — XTest with the window
  activated and focused delivered nothing, not even the stock Exit softkey (F2).
- `~/.local/share/EKA2L1/bindings/default.yml` **does** have key bindings (F1→164, F2→165,
  Enter→167, arrows→14..17, digits, `*`, `#`), and `config.yml` has
  `current-keybind-profile: default`. So "a device with no bindings" is not the cause.

## Decisions

## Dead ends

## Next step

Read the Qt input path (`src/emu/qt/src/displaywidget.cpp` `keyPressEvent`,
`mainwindow.cpp:1719`, `thread.cpp:144`) and find where a key is dropped.
