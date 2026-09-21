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
- 2026-09-21 locale pair builds (LANG SC 01 02 93, CHARACTER_SET UTF8, own RSS_SIGNATURE
  STRUCT because uikon.rh's is GUI-only). C++ code 9180 B + 204 B of .r01/.r02/.r93,
  e32 6405 B, 5 DLLs / 75 ordinals. Rust code 14328 B, e32 9684 B, 3 DLLs / 27 ordinals.
- Non-equivalence recorded: the C++ API has no per-language lookup — BaflUtils::
  NearestLanguageFile reads User::Language() itself — so the six `get_in(language)` cases
  the Rust example checks cannot be written in C++ at all.
- 2026-09-21 (coordinator's extra row) CTrapCleanup in C++ `cpphello`, same harness, toggled by
  `MACRO SYMDEV_CPP_PARITY_CLEANUP` in the mmp:
    without: e32 802, code_size 736, .text 232, .plt 112, 14 ordinals, E32Main 0x34 = 52 B
    with:    e32 833, code_size 768, .text 256, .plt 120, 15 ordinals, E32Main 0x4c = 76 B
  Delta: e32 +31, code +32 (.text +24 in E32Main, .plt +8 for one stub), imports +1 and only
  one: `CTrapCleanup::New()`. `delete cleanup` adds no import — the destructor is a vtable
  call and `CBase::operator delete` is inline. Rust's reported +44 whole-image is +13 (1.42x)
  over the C++ +31, which the 16-byte `symrs_cleanup_destroy` shim accounts for.
- 2026-09-21 ui pair builds (Avkon, 4 classes + hrh + rss + reg.rss + rls + svg icon).
  C++ code 9888 B / e32 7313 B / 8 DLLs / 225 ordinals; Rust code 20436 B / e32 13714 B /
  9 DLLs / 219 ordinals. Rust 2.07x code, 1.88x e32, but marginally FEWER ordinals.
- Next step: heap (User::AllocSize) and startup (User::NTickCount) probes on both sides,
  then write docs/research/cpp-parity.md.
- 2026-09-21 Probe commit made (Rust side instrumented in-place; to be reverted after the run,
  the C++ side stays behind `MACRO SYMDEV_CPP_PARITY_PROBE`, off by default).
