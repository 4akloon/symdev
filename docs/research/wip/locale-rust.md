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
- Full `TLanguage` list extracted (108 enumerators, 0..101 contiguous + `ELangEnglish_Apac=129`,
  `_Taiwan=157`, `_HongKong=158`, `_Prc=159`, `_Japan=160`, `_Thailand=161`, `ELangMalay_Apac=326`,
  `ELangNone=0xFFFF`). Dialects are the only declared parent/child relation in the header
  (`ELangX_Suffix`). `ELangAmerican=10`, `ELangCanadianEnglish=46` etc. have NO declared parent.
- `TLocale::LanguageDowngrade(0..2)` (`e32std.h:2278`, inline over `iLanguageDowngrade[3]`) is the
  platform's own downgrade list, but reading it needs the `TLocale` layout + `TLocale::Refresh`,
  neither observed → TODO (not observed), not used.
- Target `arm-symbian-e32.json`: `atomic-cas: true`, `max-atomic-width: 32` → `AtomicU32`
  Relaxed load/store available for the read-once cache (no `__sync_*` libcall for load/store).
- `symbian-macros` is owned by another slice → proc-macro route is closed to me anyway; decide
  on merit.

## Decisions
- D1: pure-Rust declaration via a **`macro_rules!`**, not `locales/uk.toml`. Reasons: (a) rustc's
  own struct-literal rule gives "key missing from one language" for free (E0063), no cross-file
  checker to write; (b) no `build.rs`/`include!` boilerplate per application, no `toml`/`serde` in
  the host build; (c) plain `cargo build` works; (d) `no_std` trivially; (e) symdev never sees the
  strings, which is the point of this half.
- D2: language read lazily at first use, cached in an `AtomicU32` (sentinel `u32::MAX`), so an
  application that never asks never calls euser, and the single-language case folds away.
- D3: fallback chain = exact code → dialect base (suffix stripped, read off the enum's names) →
  the declared default language.
- Built: `symbian-sys::euser::User_Language`, `symbian_core::locale::{Language, lang}` (108 named
  consts in `lang.rs`, generated from the header, compile-time `const _` assertions instead of
  `#[test]` — `symbian-rs` builds for `arm-symbian-e32` and has no test harness),
  `symbian_std::locale!` (a `macro_rules!`, in `crates/symbian-std/src/locale.rs`).
- `cargo build --release -p localedemo --offline` clean first try; clippy clean.

## Next step
- `symdev build`/`package`/`run` the example in EKA2L1 and read `User::Language()`.
- **MEASURED (EKA2L1, default config, 2026-09-21):** `User::Language()` returns **1
  (`ELangEnglish`)**. `symdev test --emulator` on `examples/locale`: `localedemo: 8 passed`.
  `E:\symdev\locale\measured.txt` shows the whole chain resolving as designed
  (`english_apac` → English, `french` → default English, `ukrainian` → Ukrainian).
- `symdev build` of `examples/locale`: `localedemo.exe` = 10 100 bytes (3 keys x 3 languages
  + fs + Report harness; not comparable with `hello` — size deltas measured separately).
- **MEASURED: the emulator's language is changeable, but only to a language the ROM lists.**
  `~/.local/share/EKA2L1/config.yml` key `language:` (not `emulator-language:`, which is the Qt
  UI's own). EKA2L1 validates it against `Z:\resource\bootdata\languages.txt` of the device and
  silently rewrites the config to the ROM default otherwise (`src/emu/system/src/epoc.cpp`
  1379-1383, list read in `src/emu/system/src/devices.cpp` ~320). Setting `language: 93` came
  back as `language: 1` and `User::Language()` still returned 1.
  The RM-469 ROM's `languages.txt` (UTF-16LE) is: `01,d 02 03 14 18 05` — English (default),
  French, German, Turkish, Dutch, Italian. Six, and no Ukrainian or Russian.
