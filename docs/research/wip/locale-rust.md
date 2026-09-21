# locale-rust — a Rust-side localisation system for in-application strings

Task: `no_std`, zero-allocation, compile-time-checked localisation of strings read by our own
Rust (not the launcher's application name — that half belongs to `ui_resources.rs`/SIS and is
not mine). Key must be a compile error when missing/mistyped; a key missing from one language
must be caught at build time. Language read once from `User::Language()`.

## Findings

## Decisions

## Dead ends

## Next step
- Read the repo: symbian-sys euser statics, symbian-macros shape, examples and their sizes.

### 2026-09-21
- `User::Language()` export verified: `nm -D $SYMDEV_EPOCROOT/epoc32/release/armv5/lib/euser.dso`
  → `00000a54 T _ZN4User8LanguageEv@@euser{000a0000}[100039e5].dll`. Decl `e32std.h:4531`.
- `TLanguage` in `e32const.h`: enum at 1439, `ELangTest = 0` (1444), `ELangEnglish = 1` (1447),
  `ELangRussian = 16`, `ELangPolish = 27`, `ELangUkrainian = 93` (1723), `ELangOther = 99`,
  `ELangNone = 0xFFFF` (1783). Needs `LC_ALL=C grep -a`.
