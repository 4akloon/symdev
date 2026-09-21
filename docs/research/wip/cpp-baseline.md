# cpp-baseline

Task: build C++ SDK equivalents of the four `no_std` Rust examples (hello, files, ui, locale)
and measure size / imports / heap / startup / DX against them; write `docs/research/cpp-parity.md`.

## Findings

## Decisions

## Dead ends

## Next step

Read the four Rust examples and the SDK `helloworldbasic` skeleton.

- 2026-09-21 The C++ hello project builds through `symdev build` with no Wine: root `symdev.toml`
  (`[language] name="cpp"`), `group/bld.inf`, `group/<n>.mmp`, `src/*.cpp`. Only warning:
  "EPOCSTACKSIZE is parsed but nothing on the GCCE path reads it".
- 2026-09-21 hello pair: C++ .text 232 / code 736 / e32 802 B; Rust .text 3524 / code 4216 /
  e32 3187 B. Imports 2 DLLs both sides (euser + drtaeabi), 14 vs 13 ordinals.
- Decision: imports and sections are read from the linked ELF, not the E32 body — the E32 body
  is Symbian LZ77+Huffman, not RFC1951. Script: docs/research/cpp-parity/measure.py.
