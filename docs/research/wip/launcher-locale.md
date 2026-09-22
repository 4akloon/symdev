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
