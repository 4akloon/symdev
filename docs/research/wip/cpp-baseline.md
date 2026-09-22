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
- 2026-09-22 History rewritten: filter-branch removed every cpp-parity/*/build path from
  main..cpp-baseline and from refs/autosave/cpp-baseline; `git log main..HEAD --diff-filter=A
  --name-only | grep -c /build/` = 0; .git 12648573 -> 8271431 bytes after gc; fsck clean.
  refs/stash still holds 7 pre-rewrite commits (stash drop was refused by the permission
  classifier) — the user must `git stash drop` + gc to purge the last copies.
- 2026-09-22 C++ files runs 16/16 behavioural cases in EKA2L1. Probe: heap entry 1 cell/36 B,
  end 7 cells/1324 B, 6 nanoticks. Rust files: entry 1/36, end 19 cells/1132 B, 9 nanoticks.
  UserHal::TickPeriod = 15625 us.
- 2026-09-22 Dead end: C++ locale cannot read its own resources inside EKA2L1.
  Path had no drive (fixed: drive from RProcess().FileName()); then RResourceFile::
  ConfirmSignatureL panics BAFL 4 (no-arg, arg 4, NAME CPLC and APLC alike); without it
  Offset()=0, OwnsResourceId=0, ReadL(full id) leaves -1; AllocReadL(index) panics BAFL 4.
  Left UNRESOLVED and recorded as a failed case. Heap C++ locale end 8 cells/1840 B, 29 ticks.
- 2026-09-22 Probe results: Rust locale entry 1/36, end 12 cells/1200 B, 21 ticks, 12/12 pass;
  User::Language x100000 = 12 nanoticks on BOTH sides (identical syscall cost).
  ui construct bracket: C++ entry 775 cells/61916 B -> end 778/62248 (+3/+332), 2 ticks;
  Rust entry 776/62080 -> end 782/62416 (+6/+336), 2 ticks. View 240x245 both.
  files repeated x3: C++ end 7 cells 1324/1292/1292 B, ticks 6/8/8; Rust 19 cells 1132 B x3,
  ticks 9/8/9 — startup indistinguishable at 1 ms tick.
- Next: revert Rust probe commit, MACRO off, rebuild, re-verify sizes, write cpp-parity.md.
