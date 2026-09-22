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

## Decisions

- Shim: `symrs_rsc.cpp` holds both wrappers (no program uses one without the other); it also
  refuses a path longer than `KMaxFileName` (TFileName's ctor would USER 11 otherwise).
- sys: `RResourceFile` storage in a new `symbian-sys/src/bafl.rs` (24 B, align 4), opaque
  `HBufC8` in `des8.rs`, `TDesC8_Ptr` in `des.rs`, externs in `shim.rs` (euser.rs untouched).

## Dead ends

## Next step

`symbian-core/src/locale/strings.rs`: Str, Text, the static open file.
