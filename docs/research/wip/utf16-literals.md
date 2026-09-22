# WIP: compile-time UTF-16 literals (experiment 106)

Task: measure which `symbian-rs/examples/*` (not `std-*`) could drop `Buf16::push_str`
(run-time UTF-8 -> UTF-16); if worth it, put compile-time-known text into the image as
UTF-16 (const fn behind `Lit16`, fast `write!` literal pieces, or `u16!`). `hello` is
1 245 B vs C++ 802 B. Output byte-identical; extend `crates/symdev-build/tests/fast_write*.rs`.
Measure every example before/after; emulator tests must pass. Record as experiment 106.
Mine: `symbian-core/src/des/**`, `symbian-macros/src/fast_write/**`. Not mine: symbian-ui,
shims/s60, macros/entry.rs, std/src/fs, core/src/fs.

## Findings

## Decisions

## Dead ends

## Next step

Measure baseline sizes of every example.
