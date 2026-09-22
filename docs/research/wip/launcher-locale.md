# Native localisation: per-language resource files, read on demand

Replace the all-languages-in-the-image `locale!` with the C++ model — one compiled
resource file per language, opened once, strings read on demand — and localise the
launcher caption through the same files. The user's rule: resource use at the level of
the native solution.

## Findings

- **BAFL 4 in the C++ locale baseline was a bug in the probe, not in our rcomp.** Strings
  declared `TBUF` (no length byte) were read with `TResourceReader::ReadHBufCL`, which
  reads a length byte first and runs off the buffer. Read as raw UTF-16 and with
  `ConfirmSignatureL(0)` first: `GREETING=Hello from C++` from
  `E:\resource\apps\cpplocale.r01` (picked by `BaflUtils::NearestLanguageFile`),
  `cpplocale: 6 passed`. So our rcomp's `.rNN` + NearestLanguageFile + ConfirmSignatureL
  + AllocReadL all work in EKA2L1.
- The C++ baseline hardcodes `KUid3 = 0xe00006a3` in `cpplocale.cpp`; changing the UID
  in the .mmp alone makes the report land under the old name — two "failures" of mine
  were that, not the program.
- A `KERN-EXEC 3` "terminated peacefully" line appears at the C++ locale's thread end,
  after the report is written. Not investigated; not on this path.
- **The C++ heap, measured (User::AllocSize) reading one string:** before open 8/1544,
  after `RResourceFile::OpenL` 12/1752 (+4 cells, +208 B, held while open),
  `ConfirmSignatureL` +0, `AllocReadL` one TBUF 13/1788 (+1, +36), after delete 12/1752.
  That is the target.

## Decisions

## Dead ends

## Next step

Design: build side (locale files -> `.rNN` via our rcomp, installed), runtime side
(open once through a shim, read on demand), and the caption.
- Task 1 done (`c7fd7ee`): `crates/symdev-locale` — 108 languages (ELangTest/None left
  out), strict TOML-subset parser, completeness check, index = 2 + byte-sorted position.
- Task 2 done: `<0xNN>` in `BUF8` round-trips every byte through our rcomp (0x80, 0x9f,
  0xff, Cyrillic, quotes, backslash, newline, tab), both in the compiler's model and in
  the `.rsc` bytes. `symdev build` writes `<app>_strings.rsc/.r02/.r93`; **`symdev
  package` recomputes its own artifact list** (`crates/symdev-cli/src/artifacts.rs`), so
  it needed the strings files named there too — without that the SIS would have shipped
  without them. Verified inside the `.sisx`: the three destinations and the UTF-8 of
  "Bonjour", "Привіт", "гаразд", "d'accord".
- Task 6 done: a `locales/<language>.toml` with `caption`/`short_caption` gives
  `<app>.r<code>`, the application resource with only the caption pair changed; the
  registration's `localisable_resource_file` names the file without an extension, so the
  launcher picks it. **Verified in EKA2L1**, the same `.sisx`: `language: 1` → title
  "Bars" (`uidemo.rsc`), `language: 2` → "Barres" (`uidemo.r02`). A `locales/` that only
  translates the caption compiles no strings file.

## Task 3: the run-time reader (branch `locale-runtime`)

Built: `symbian-rs/shims/common/symrs_rsc.cpp` (`symrs_rsc_open`: construct in place,
`NearestLanguageFile`, `OpenL` + `ConfirmSignatureL(0)` under one TRAP, `Close` on failure;
`symrs_rsc_read`: `AllocReadL(Offset() + index)`), `symbian-sys` bindings (`bafl.rs` with the
24-byte 4-aligned `RResourceFile`, opaque `HBufC8`, `TDesC8_Ptr` = `_ZNK6TDesC83PtrEv` at
0x1ad8), and `symbian_core::locale::{Str, Text}` in `locale/strings.rs`: the file is a static
opened lazily on the first `get` and never closed (single-thread claim as `fs/session.rs`,
an `Opening` state as the reentrance flag), path `<exe drive>:\resource\apps\<stem>_strings.rsc`
from `symrs_process_file_name`, UTF-8 checked once (`KErrCorrupt` otherwise), `Drop` =
`User::Free`. Index 0, 1 or > 4095 is `KErrArgument`.

- **Heap against C++**, scratch console probe (not committed), `User::AllocSize`, file-server
  session connected before the first reading, as C++ had its `RFs`:
  | step | Rust | C++ |
  |---|---|---|
  | open file (`NearestLanguageFile`+`OpenL`+`ConfirmSignatureL`), held | +4 cells / +168 B | +4 / +208 |
  | one 14-character string held | +1 / +36 B | +1 / +36 |
  | string dropped | −1 / −36 B | −1 / −36 |
  Repeated get/drop and the error paths return to the same count. The open is 40 B *below*
  C++; the cause is not isolated (the files differ: C++'s is `TBUF` UTF-16 with rcomp's
  Unicode-compression header flags, ours `BUF8`, and C++ opened `.r01`, the probe `.rsc`).
  The string cell is the same 36 B although ours is 14 bytes of UTF-8 and C++'s 28 of UTF-16.
- **Emulator evidence:** index 2 `Hello, strings`, index 3 `Привіт é ok` (18 UTF-8 bytes,
  written as `<0xNN>` in the `.rss`) intact, index 4 `third`, index 9 `KErrNotFound` from
  `AllocReadL`; `strdemo: 7 passed`. With a `…_strings.r01` also installed, index 2 came from
  it: `NearestLanguageFile` picks the variant.
- **`delete` vs `User::Free`:** C++'s `delete` on an `HBufC8` is `_ZdlPv` from `scppnwdl.dso`,
  not euser, and the ROM's `scppnwdl.dll` in EKA2L1's dump is truncated (296 B; `_ZdlPv` is a
  Thumb thunk to an import not in the file), so "delete == User::Free" is not statically
  verified. `e32des8.h` says an HBufC8 is "hosted by a heap cell" and the class has no
  destructor; the cell count and bytes returning to baseline after `User::Free` is the check.
- **Size**, minimal program (`examples/hello` + one `Str::at(2).get()`): hello 2 567 B; + a
  first shim TRAP 5 087 B (+2 520 B: libsupc++/libgcc's unwinder and `__gxx_personality_v0`,
  paid once by any program with a TRAP); + the reader 6 556 B (+1 469 B, +2 340 B text, +40 B
  bss). Without a prior TRAP the reader costs +3 989 B. `examples/hello` stays byte-identical
  (only the E32 header CRC and time differ): gc-sections drops `symrs_rsc.o` when unused.
- `cargo test --workspace` inside `symbian-rs/` fails on `launcher-locale` before this change
  too (no `test` crate for `arm-symbian-e32`); the root workspace tests and all three clippy
  runs are clean.
