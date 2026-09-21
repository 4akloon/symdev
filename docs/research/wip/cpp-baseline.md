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
- 2026-09-21 files pair builds. C++ code 8880 B / e32 6058 B / 4 DLLs / 71 ordinals;
  Rust code 16316 B / e32 10552 B / 3 DLLs / 39 ordinals. Rust loses 1.84x on code but
  WINS on imports (39 vs 71 ordinals, 3 vs 4 DLLs).
- 2026-09-21 Mechanism for hello: of the Rust image's 3710 B of code symbols, 3172 B (85%)
  is `core::fmt` (integer Display 908, str Display 784, fmt::write 536, Formatter::padding/
  pad_integral, Buf16 write_str 444). C++ formats with `TDes::Format`, a euser export: zero
  bytes in the image, one import ordinal.
- 2026-09-21 Heap method verified in headers: `User::AllocSize(TInt&)` (e32std.h:4484) returns
  the cell count and outputs total allocated bytes; `RHeap::Size()` (e32cmn.inl:78) is
  "total number of bytes committed by the host chunk", not the allocation. Use AllocSize.
