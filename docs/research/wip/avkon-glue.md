# WIP: experiment 104 — smallest Avkon glue

Task: shrink the Avkon glue in `symbian-rs/crates/symbian-ui` (generic over the app type,
monomorphised per app: `vtbl::construct<Bars>` 2 212 B, `draw<Bars>` 1 360 B) plus the C++ shim
`symbian-rs/shims/s60/symrs_avkon.cpp` (~1.8 kB), keeping the public API unchanged
(`symbian_std::ui::App`, `#[symbian_std::main(gui)]`, `App::menu` closures, `List`, notes, queries).
Baseline: `examples/ui` 11 645 B vs C++ 7 317 B. Measure every non-std example before/after by symbol.
Verify: `symdev test --emulator` for every GUI example; ui by hand (F1, Down Return → `bars=2 keys=0 cmd=1`);
ui-list (Down Down Down Return → `picked: 3`). Out of bounds: symbian-std/src/fs, symbian-core/src/fs,
symbian-core/src/des, symbian-macros/src/fast_write.

## Findings

## Decisions

## Dead ends

## Next step

Measure baseline sizes of every example.
