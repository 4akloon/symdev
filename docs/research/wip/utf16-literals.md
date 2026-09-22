# WIP: compile-time UTF-16 literals (experiment 106)

Task: measure which `symbian-rs/examples/*` (not `std-*`) could drop `Buf16::push_str`
(run-time UTF-8 -> UTF-16); if worth it, put compile-time-known text into the image as
UTF-16 (const fn behind `Lit16`, fast `write!` literal pieces, or `u16!`). `hello` is
1 245 B vs C++ 802 B. Output byte-identical; extend `crates/symdev-build/tests/fast_write*.rs`.
Measure every example before/after; emulator tests must pass. Record as experiment 106.
Mine: `symbian-core/src/des/**`, `symbian-macros/src/fast_write/**`. Not mine: symbian-ui,
shims/s60, macros/entry.rs, std/src/fs, core/src/fs.

## Findings

- Baseline (main de45e20, `~/.cache/utf16-literals-agent/res-base.txt`, exe bytes / UTF-16 symbol bytes): alloc 3767/656, async 18603/448, atomics 8903/640, cleanup 4472/448, files 9341/448, fmt 100031/3456, hello-raw 808/0, hello 1245/428, locale 8202/448, net 10640/448, notes 12805/520, panic 2010/432, query 14031/532, shim 4517/648, spawnee 3076/448, time 10256/448, tls 14114/736, ui-list 12624/520, ui 11645/520.
- Who links push_str and why: every report-writing example (result-file path via `fs::path_of` -> `Buf16<256>::push_str`), `panic` and `spawnee` (fs paths), `alloc` (`HBuf16::push_str` of a run-time `String`), `shim` (FileServer paths; its own literal `push_str`s into `Buf16<160>` are a separate 120-byte monomorph). Only `hello` links it purely for compile-time-known text; `hello-raw` already has none.
- Buf16 destinations of the fast `write!` in examples: only `hello`, `alloc` (its one Buf16 `write!` has `{over_byte:x}`, so it is `core::write!` whole) and `fmt` (test). The harness writes into `String`, so report examples are not touched by a Buf16-only literal path. `shim` calls `push_str("literal")` directly six times (Buf16<160> monomorph 120 B) but keeps the shared encoder for FileServer paths.
- `TDes16::Append(const TDesC16&)` is already bound (observed); `hello` imports `__aeabi_memclr4` from drtaeabi, so a units copy should also resolve to ROM (check).
- A proc macro cannot tell that `{GREETING}` names a `const`: constness is not in the type, so autoref specialisation cannot see it either, and a `const {}` block around a run-time variable is a compile error, not a fallback. So `{GREETING}` of a `&str` stays on `push_str`.

## Decisions

- Design to try: `symbian_fmt::Utf16Str` = `&'static str` + its `&'static [u16]`, built only by `utf16!(expr)` (const items: `[u16; utf16_len(S)]` from a const fn), `Deref<Target = str>` and `Display` = str's, `Arg` -> new `Sink::put_utf16(text, units)` whose default is `put_str(text)` (so every non-Buf16 destination sees the same `write_str`), `Buf16` overrides with a room check + unit copy. The fast `write!` emits each literal piece as a `utf16!` const. `hello` changes one line: `const GREETING: Utf16Str = utf16!("…")`.

## Dead ends

## Next step

Measure baseline sizes of every example.
