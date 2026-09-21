# Localisation, the Rust half: strings only the application reads

Research note, 2026-09-21. Branch `locale-rust`. Cites experiment 96 in
[experiment-backlog.md](experiment-backlog.md), the corpus at
`symbian-rs/corpus/96-locale/`, `symbian-rs/crates/symbian-core/src/locale/`,
`symbian-rs/crates/symbian-std/src/locale.rs`, `symbian-rs/examples/locale`, and
[command-id-design.md](command-id-design.md), whose question this one is the sequel to.
(That note was renamed `runtime-menu-design.md` on `main` after this branch left it; the
links here are to the file as it stands at this branch's base, `91d57c4`.)

## The split, and which half this is

Localisation on this platform is two unrelated problems that share a word.

**The application's name** is painted by the phone's launcher while the application is
**not running**, so nothing of ours can be asked for it. It can only be the platform's
own mechanism: a `localisable_resource_file` compiled once per language into
`<app>.r01`, `<app>.r93`… and resolved by `BaflUtils::NearestLanguageFile`. That half
belongs to `crates/symdev-build/src/ui_resources.rs` and the SIS, and **nothing on this
branch touches it.**

**Strings inside the application** are read by our own Rust and by nobody else. They
need no resource, no `.rss`, no `rcomp` run and no entry in `symdev.toml` — which is
also what the user asked for: the manifest holds what the phone needs *before* the
application runs, and a translation is not that. This note is that half.

## What the platform gives, re-verified

- **`User::Language()` is exported.**
  `nm -D $SYMDEV_EPOCROOT/epoc32/release/armv5/lib/euser.dso` →
  `00000a54 T _ZN4User8LanguageEv@@euser{000a0000}[100039e5].dll`. Declared
  `IMPORT_C static TLanguage Language();` at `e32std.h:4531`. A static member function,
  so it binds like the other euser statics in `symbian-sys/src/euser.rs`.
- **`TLanguage`** is the enum at `e32const.h:1439`: `ELangTest = 0`, `ELangEnglish = 1`,
  `ELangRussian = 16`, `ELangPolish = 27`, `ELangUkrainian = 93`, `ELangOther = 99`,
  `ELangNone = 0xFFFF`, 108 enumerators in all, including the seven dialects
  `ELangEnglish_Apac = 129`, `_Taiwan = 157`, `_HongKong = 158`, `_Prc = 159`,
  `_Japan = 160`, `_Thailand = 161` and `ELangMalay_Apac = 326`. The file is not ISO
  text: `LC_ALL=C grep -a`.
- **Sixteen bits is the platform's own width for it.** `TLocale` stores three of them as
  `TUint16 iLanguageDowngrade[3]` (`e32std.h:2314`), and `ELangNone = 0xFFFF` is the
  largest enumerator.

## Verdict

**A `macro_rules!` table in Rust, key-major, one file per module, no proc macro and no
translation file format.** The declaration is

```rust
// src/strings.rs
symbian_std::locale! {
    languages: english, ukrainian, french;

    GREETING    = { english: "Hello from Rust", ukrainian: "Привіт з Rust", french: "Bonjour depuis Rust" },
    LANGUAGE_IS = { english: "language",        ukrainian: "мова",          french: "langue" },
}
```

and the use is `strings::GREETING.get()` — a `&'static str`, no allocation, `no_std`,
and (measured) **nothing at all** when only one language is declared.

`locale!` expands to a `pub struct Text` with **one field per declared language** and
one `pub const` per key. That single choice is what buys every guarantee: rustc already
refuses an incomplete struct literal, so a key missing from one language is `error[E0063]:
missing field ukrainian in initializer of Text` with no completeness checker written by
us and none to get wrong.

## Why not translation files (`locales/uk.toml`)

The `rust-i18n` idiom — a directory of per-language files read by a proc macro or a
`build.rs` — was the other candidate, and it is the one most projects pick. It loses
here on four counts, in order of weight:

1. **It has to re-implement the guarantee that Rust gives away.** "This key exists in
   `en.toml` and not in `uk.toml`" is a check somebody writes, tests and maintains,
   and it is the *same* check as "this struct literal is missing a field" — which rustc
   performs, reports with the field's name, and can never be wrong about. A key-major
   Rust table gets it for free.
2. **It needs a reader, and a reader is either a proc macro or a `build.rs`.** The menu
   slice measured what a manifest-reading proc macro costs
   ([command-id-design.md](command-id-design.md)): `symbian-macros` gains a path
   dependency on `symdev-manifest`, which pulls `serde`, `toml`, `thiserror`, `syn` and
   their support crates into the host build of **every** application — 19 more
   `Cargo.lock` entries there. Doing it a second time for strings buys nothing new. A
   `build.rs` avoids the dependency on `symdev-manifest` but not on a TOML parser, and
   adds a `build.rs` *and* an `include!` line to every application that has any string.
   `macro_rules!` needs neither: it is 90 lines in `symbian-std`, compiles with the
   crate, and adds no dependency to anything.
3. **A `macro_rules!` works everywhere the crate works.** Plain `cargo build` with no
   symdev, `cargo build` in a bare checkout, `no_std`, the `std` overlay, rust-analyzer's
   own expansion — nothing special has to hold.
4. **Non-obviously, the Rust file is the better file for translators.** The thing a
   translator must not do is answer half the questions, and a key-major table puts every
   language for one key on adjacent lines, where a missing one is visible to a human
   before it is visible to the compiler. A per-language file is *worse* for that: you
   cannot see what you are translating against. The row is `english: "…", ukrainian: "…"`
   — it is not harder to edit than TOML, and it lives in `src/strings.rs`, which is the
   one place to look.

What the file layout costs: a translator needs the repository and a compile to check
their work, rather than a `.toml` they can be handed. If that ever matters, the escape
is one line — `include!("../locales/all.rs")` around the same `locale!` block — and
nothing in the design changes.

**A note on what was *not* available:** `symbian-macros` is being rewritten by the
runtime-menu slice and is out of bounds on this branch, so a proc macro could not have
been built here anyway. The argument above is the one that would apply if it had been
free, and it is the reason the answer would not change.

## The fallback chain

For a device reporting language `L`, `Text::get()` returns the first of:

1. **`L` itself**, if the declaration names it.
2. **`L.base()`** — the language `L` is a *dialect* of — if the declaration names that.
   This step is why a phone set to `ELangEnglish_Apac` reads English, and it is not
   invented: the `_Apac` suffix is in `e32const.h`'s own enumerator name, over an
   `ELangEnglish` the same enum declares. It covers exactly the seven suffixed
   enumerators and nothing else.
3. **The declared default**, which is the first language in the `languages:` list.

Two things are deliberately *not* in the chain:

- **The licensee downgrade list.** `TLocale::LanguageDowngrade(0..2)` (`e32std.h:2278`,
  an inline over `TUint16 iLanguageDowngrade[3]`) is the platform's own fourth step, the
  one `BaflUtils::NearestLanguageFile` walks. Reading it needs the `TLocale` layout and a
  `TLocale::Refresh`, and **`TODO: what TLocale::Refresh writes into iLanguageDowngrade
  on this ROM (not observed)`** — so it is absent rather than guessed.
- **The unsuffixed English variants.** `ELangAmerican = 10`,
  `ELangCanadianEnglish = 46`, `ELangInternationalEnglish = 47`,
  `ELangSouthAfricanEnglish = 48`, `ELangAustralian = 20`, `ELangNewZealand = 23` are
  English to a reader, and `e32const.h` says nothing that makes them English to a
  program. `Language::base` returns `None` for every one of them, and an application
  that wants them adds the column (the strings are identical, so it costs the eight
  bytes of a second pointer and length, not a second copy of the text). Inventing the
  relation would be exactly the kind of guess the repository's rules forbid.

## Reading the language: measured, not reasoned

The brief asked where the language should be read once — entry point, first use, or an
`OnceCell` equivalent. **The answer the machine gave is "none of them: do not cache".**

The obvious shape was built first: `Language::current()` reading `User::Language()` once
into a `static AtomicU32` with `u32::MAX` as the "unread" sentinel, relaxed load, relaxed
store, benign race. Then it was measured.

| `hello`-shaped program, `.exe` bytes | with the `AtomicU32` cache | asking euser every time |
|---|---|---|
| greeting as a plain `const` (no `locale!`) | 3 187 | 3 187 |
| `locale!`, 1 language | 3 183 | 3 183 |
| `locale!`, 2 languages | **4 024** | **3 230** |
| `locale!`, 3 languages | 4 067 | 3 309 |

**The cache costs 794 bytes** — a quarter of the whole image — because ARMv5TE has no
atomic instruction. `AtomicU32::load(Relaxed)` compiles to `bl __atomic_load_4`
(disassembled and confirmed: `82d0: bl 9678 <__atomic_load_4@plt>`), and linking that
one call drags in `symbian-libcalls`' whole atomics object: all thirty `__atomic_*`
entry points, `AtomicLock`, `RFastLock::{CreateLocal,Wait,Signal}`, `__sync_synchronize`,
16 bytes of `.bss`, four more PLT entries and about 43 more dynamic symbols. `.text`
grows 1 952 bytes.

And it is **slower**, which is the part that settles it. That libcall is a lock-based
emulation — an `RFastLock` Wait/Signal pair, a kernel round trip, per read. 100 000
iterations in the emulator: **16** nanokernel ticks through the cache, **12** straight to
euser. The cache is bigger and slower than the call it replaces.

So:

- **`Language::current()` asks euser, every time.** 0.12 µs inside EKA2L1
  (100 000 calls / 12 ticks of 1 000 µs; the emulator's dynarmic JIT, not an E52).
- **`Text::get()`** calls it — which is right for the occasional string.
- **`Text::get_in(language)`** takes the language instead, and *that* is the read-once
  answer: `Language` is two bytes and `Copy`, so an application that reads it once into
  a local or a field and passes it down has a cache that costs nothing and needs no
  atomic, no `OnceCell`, no lock and no `unsafe`.
- **One language costs nothing at all.** The generated `get()` opens with a constant
  `if TRANSLATED.is_empty()`, so a single-language module returns its one field, LLVM
  folds the branch, and `User::Language()` is never even imported — 3 183 bytes against
  the plain-`const` program's 3 187.

`core` has no `OnceCell` that avoids the atomic, there is no `std::sync` on this path,
and the target JSON says `has-thread-local: false`, so no cheaper cache exists to reach
for.

## What the compiler refuses

Every message below was produced by `cargo build --release` on the phone target, not
written out from memory; the full transcripts are in
`symbian-rs/corpus/96-locale/compile-errors.md`.

| Mistake | rustc |
|---|---|
| `strings::GREETNG.get()` | `error[E0425]: cannot find value GREETNG in module strings` + *note: similarly named constant `GREETING` defined here* + a `help` that rewrites the line |
| a row missing a language | `error[E0063]: missing field french in initializer of Text` |
| a row naming an undeclared language | `error[E0560]: struct Text has no field named german` |
| `languages: english, frnech;` | `error[E0425]: cannot find value frnech in module $crate::locale::lang` + *note: similarly named constant `french` defined here* at `lang.rs:23` + a `help` that fixes the spelling |
| two rows with the same key | `error[E0428]: the name GREETING is defined multiple times` |
| a translation that is not a string | `error[E0308]: mismatched types … expected &str, found integer` |

Two honest weaknesses:

- **E0063 points at the invocation, not the row.** A `macro_rules!` expansion reports at
  the call site, so the error underlines the whole `locale!` block and names the missing
  *language* rather than the key. Capturing each row as a `tt` so the braces would carry
  the caller's span was tried and changed nothing. With many keys, the language name is
  the thing to search for.
- **A key nothing uses is not an error.** The constant is emitted and unused, and
  `dead_code` does not fire inside a macro expansion — the same hole
  [command-id-design.md](command-id-design.md) records for menu constants, and the same
  as C++, where an unused enumerator is silent.

The language table is written out as 110 plain `pub const` lines rather than produced by
a generating macro **for the error message**: with the macro, rustc's "similarly named
constant" note pointed inside the macro body; without it, it points at
`pub const french: Language = Language::from_code(2); // ELangFrench`.

## What the emulator says

`symdev build` / `package` / `test --emulator` on `symbian-rs/examples/locale`, three
runs of the **same** `.sisx`, changing only `language:` in
`~/.local/share/EKA2L1/config.yml`:

| config | `User::Language()` | `GREETING` | `LANGUAGE_IS` | `OK` |
|---|---|---|---|---|
| 1 | 1 `ELangEnglish` | Hello from Rust | language | ok |
| 2 | 2 `ELangFrench` | Bonjour depuis Rust | langue | d'accord |
| 3 | 3 `ELangGerman` (the table has no German) | Hello from Rust | language | ok |

All three `localedemo: 8 passed`. The third row is the fallback to the declared default,
live.

**The emulator's language can be changed, and only within limits that must be recorded.**
The key is `language:` in `config.yml` (not `emulator-language:`, which is the Qt
front-end's own UI). EKA2L1 validates it against the device's
`Z:\resource\bootdata\languages.txt` and silently rewrites the config to the ROM default
otherwise (`src/emu/system/src/epoc.cpp` lines 1379-1383; the list is read in
`src/emu/system/src/devices.cpp` around line 320). The RM-469 ROM's file, UTF-16LE, is
`01,d 02 03 14 18 05` — **English (default), French, German, Turkish, Dutch, Italian, and
nothing else**. Setting `language: 93` came back as `language: 1` and `User::Language()`
still answered 1. So Ukrainian, Russian and every dialect enumerator **cannot be tested on
this ROM**; the fallback chain for those is exercised through `Text::get_in`, which takes
a language rather than asking for one, and the six cases pass on the device — but they are
not evidence about what a *device set to* `ELangEnglish_Apac` does, and nothing here
claims they are.

And, as for every step so far: **no E52.**

## The API, as it ended up

`symbian_core::locale` (re-exported as `symbian_std::locale`):

- `Language` — a `Copy` newtype over `u16`. `from_code`, `code`, `current`, `base`.
- `lang` — 110 `pub const Language`s, one per `TLanguage`, named by dropping `ELang`,
  breaking the CamelCase at each capital and lower-casing: `lang::english`,
  `lang::south_african_english`, `lang::english_apac`. Lower case on purpose, because
  those are the words written inside `locale!`; the lint is disabled for that module and
  only there.

`symbian_std::locale!` emits, into the module that invokes it:

- `pub struct Text` — one `&'static str` field per declared language.
- `Text::get(self) -> &'static str` — the chain, asking the device.
- `Text::get_in(self, Language) -> &'static str` — `const fn`, the chain, for a caller
  that already has the language.
- `pub const <KEY>: Text` — one per row.

`symbian_sys::euser::User_Language`.

## Left undone

- **`TLocale::LanguageDowngrade`.** The layout and `Refresh` were not observed.
- **A device.** Nothing here was run on an E52, and the emulator's ROM offers six
  languages.
- **Aliases in the declaration.** `languages: english [american, australian], …` would
  let one column answer several `TLanguage`s without a second copy of the strings; today
  an author adds the column instead. Not built, because the grammar cost is real and
  nothing needed it yet.
- **Whether a language change under a running process is visible to it.** Not observed;
  nothing here assumes either way, because nothing here remembers the value.
- **The other half.** The application's launcher name still needs
  `localisable_resource_file`, per-language `.r0N` compilation and
  `BaflUtils::NearestLanguageFile`, none of which is on this branch.
