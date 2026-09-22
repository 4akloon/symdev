# Task 3 of native localisation: the run-time strings reader

Plan: `docs/superpowers/plans/2026-09-22-native-localisation.md`, Task 3. Shim
`symrs_rsc_open`/`symrs_rsc_read`, `symbian_core::locale::{Str, Text}`, proved on the
emulator with a scratch console example against the C++ heap target (open +4/+208 B,
string +1/+36 B, all returned on free).

## Findings

- Exports confirmed with `nm -D` (the "string table corrupt" warning is harmless): bafl has
  `RResourceFileC1Ev`, `OpenLER3RFsRK7TDesC16`, `ConfirmSignatureLEi`, `AllocReadLEi`,
  `CloseEv`, `Offset2Ev`, `BaflUtils::NearestLanguageFile(const RFs&, TBuf<256>&)`; euser has
  `_ZNK6TDesC83PtrEv` (0x1ad8), `User::Free` and `User::AllocSize(TInt&)`.
- `delete` of an `HBufC8` is not an euser call: global `operator delete` (`_ZdlPv`) is exported
  by `scppnwdl.dso`, not euser. `e32des8.h` says an HBufC8 "is hosted by a heap cell" (the
  thread heap); the ROM's `scppnwdl.dll` in EKA2L1's drive dump is 296 bytes, truncated, its
  `_ZdlPv` a Thumb thunk (`push; blx <import>; pop`) whose target is not in the file. So
  "`delete` == `User::Free`" is not statically verified; `HBufC8` has no destructor to run
  (trivial, non-virtual class), so freeing its cell with `User::Free` is what remains, and the
  emulator's cell count/bytes returning to baseline is the check.

- Emulator (scratch `strdemo`, uid 0xe00006b3, hand-written .rss compiled with our rcomp,
  installed at `E:\resource\apps\strdemo_strings.rsc`): index 2 `Hello, strings`, index 3
  `Привіт é ok` (18 UTF-8 bytes) intact, index 4 `third`; index 9 → KErrNotFound (-1) from
  AllocReadL, index 1 refused KErrArgument; `strdemo: 7 passed`.
- Heap (User::AllocSize, session already connected): before 4/176; first get, string held
  9/380 (+5/+204); string dropped 8/344 → open file = +4 cells/+168 B (C++ +4/+208);
  string held +1/+36 B (C++ +1/+36, 14 chars either way; ours 14 B UTF-8 vs C++ 28 B
  UTF-16 — same cell size); drop −1/−36; repeated get/drop and error paths return to 8/344.
- With `strdemo_strings.r01` ("Hello, English") also installed, index 2 read `Hello, English`:
  NearestLanguageFile picks the language variant.
- Size (minimal program = examples/hello + one `Str::at(2).get()`): hello 2 567 B exe; hello +
  first TRAP (`leave_if_error`) 5 087 B — +2 520 B is libsupc++/libgcc's unwinder and
  `__gxx_personality_v0` (~3.4 KB text), paid once by any program with a shim TRAP; reader on
  top of that 6 556 B = +1 469 B exe (+2 340 B text, +40 B bss). Without a prior TRAP the
  reader costs +3 989 B. `_LIT16` path pieces instead of `push_str` saved 232 B text.
- Gates: root `cargo test --workspace` and clippy clean; symbian-rs clippy (all-targets, release)
  clean. `cargo test --workspace` inside `symbian-rs/` fails before this change too (no `test`
  crate for `arm-symbian-e32`), so it is not a gate that can pass there.
- `examples/hello` is byte-identical with and without the new shim (only the E32 header's CRC
  and time differ): gc-sections drops `symrs_rsc.o` from a program that reads no strings.

## Decisions

- Shim: `symrs_rsc.cpp` holds both wrappers (no program uses one without the other); it also
  refuses a path longer than `KMaxFileName` (TFileName's ctor would USER 11 otherwise).
- sys: `RResourceFile` storage in a new `symbian-sys/src/bafl.rs` (24 B, align 4), opaque
  `HBufC8` in `des8.rs`, `TDesC8_Ptr` in `des.rs`, externs in `shim.rs` (euser.rs untouched).

## Dead ends

## Next step

Gates (test, clippy, clippy --release), drop scratch examples from members, fold note.
