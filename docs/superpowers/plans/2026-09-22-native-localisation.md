# Native localisation — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Localised strings and the launcher caption use the C++ SDK's model — one compiled
resource file per language, the device's language picked by the platform, strings read on
demand — instead of every language living in the executable.

**Architecture:** Translations are written per language in `locales/<language>.toml` beside
`Cargo.toml` (`default.toml` is the fallback). symdev compiles each into
`<app>_strings.rsc` / `<app>_strings.rNN` with the existing native `rcomp`, strings as
`BUF8` holding UTF-8, and installs them all. At run time the program opens the file
`BaflUtils::NearestLanguageFile` picks, once, through a C++ shim, and reads a string with
`RResourceFile::AllocReadL` when asked: one heap cell while it is held, as C++. A proc macro
reads the same files at compile time and emits one constant per key, so a missing key is a
compile error. A GUI project's caption is localised the same way, through `<app>.rNN`.

**Tech Stack:** Rust 1.98.1 (host crates and `no_std` phone crates), our `symdev-rcomp`,
the C++ shim archive `libsymrs.a`, EKA2L1 for verification.

**Spec:** `docs/research/wip/launcher-locale.md` (findings and the measured C++ target).

## Global Constraints

- The C++ target, measured: opening the resource file +4 cells / +208 B held while open;
  `ConfirmSignatureL` +0; one string held +1 cell / +36 B for a 14-character string;
  everything returned on free. Rust must not exceed this.
- `symdev.toml` holds only project configuration; translations are not configuration and
  never go there. `[ui] caption` stays where it is — it is the default caption.
- No `unwrap`/`expect`/`panic!` outside tests; every `.rs` file ≤ 300 lines; library paths
  return `Result` with an error naming what failed and the fix.
- Behaviour not observed from the real tools is an error, never a guess.
- `rcomp`'s `<0x0000>` literal is broken in this build (`rcomp-spec.md` §3); a translation
  containing U+0000 is refused.
- `no_std` is first-class: the runtime must work in a console `no_std` program.
- Commit messages: one full imperative sentence ending with a period, plus
  `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- Gates before merging: `cargo test --workspace --offline`,
  `cargo clippy --workspace --all-targets --offline`, and
  `cargo clippy --release --workspace --offline` in `symbian-rs/` — all clean.

## The contract every task shares

- **Files:** `locales/default.toml` (required when `locales/` exists) and
  `locales/<language>.toml`, where `<language>` is a `TLanguage` name with `ELang` dropped
  and CamelCase broken into lower-case words (`english`, `french`, `ukrainian`,
  `english_apac`). The `.rNN` suffix is the `TLanguage` value, two digits minimum
  (`french` → `.r02`, `ukrainian` → `.r93`, `english_apac` → `.r129`).
- **Syntax:** one `key = "value"` per line; blank lines and `#` comments; keys match
  `[a-z][a-z0-9_]*`; values are TOML basic strings with `\"`, `\\`, `\n`, `\t`, `\uXXXX`.
  Nothing else — no tables, no arrays, no other types. A line outside this is an error with
  the file and line number.
- **Reserved keys:** `caption` and `short_caption` translate `[ui] caption` /
  `short_caption`; they are allowed only in non-default files and are not string constants.
- **Completeness:** every non-reserved key of `default.toml` is in every other file and no
  other file has a key `default.toml` lacks.
- **Index:** the string keys sorted by byte order; key *i* (0-based) is resource index
  `2 + i` in the compiled file (index 1 is `RSS_SIGNATURE`). The run-time id is
  `RResourceFile::Offset() + index`.
- **Resource text:** `BUF8` whose bytes are the value's UTF-8.
- **Installed as:** `!:\resource\apps\<app>_strings.rsc` and `…_strings.rNN`, where
  `<app>` is the executable's stem. The run time derives the path from its own
  `RProcess::FileName()`: same drive, same stem.

---

### Task 1: `symdev-locale`, the one reader of locale files

**Files:**
- Create: `crates/symdev-locale/Cargo.toml`, `crates/symdev-locale/src/lib.rs`,
  `crates/symdev-locale/src/language.rs` (the name ↔ code table),
  `crates/symdev-locale/src/file.rs` (one file's syntax),
  `crates/symdev-locale/src/locales.rs` (the directory, completeness, indices),
  `crates/symdev-locale/src/tests.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces — produces:**
- `pub struct Language { pub name: &'static str, pub code: u16 }`,
  `Language::named(&str) -> Option<Language>`, `Language::suffix(&self) -> String`
  (`"r02"`, `"r93"`, `"r129"`).
- `pub struct Locales { pub default: Table, pub variants: Vec<(Language, Table)> }`,
  `Locales::load(dir: &Path) -> Result<Option<Locales>, Error>` (`None` when the directory
  does not exist), `Locales::keys(&self) -> Vec<&str>` (sorted string keys),
  `Locales::index(&self, key: &str) -> Option<u16>`.
- `pub struct Table { pub entries: BTreeMap<String, String> }`, `Table::caption()`,
  `Table::short_caption()`.
- `pub struct Error(String)` implementing `Display`, `std::error::Error`.

It must be dependency-free: `symbian-macros` (a proc-macro crate kept free of
dependencies) depends on it by path. The language table is ported from
`symbian-rs/crates/symbian-core/src/locale/lang.rs` (read off `e32const.h`), which Task 5
deletes.

- [ ] **Step 1: Write failing tests** in `src/tests.rs`: `Language::named("ukrainian")`
  is code 93 and suffix `r93`; `english_apac` is 129 / `r129`; an unknown name is `None`.
  Parsing `greeting = "Привіт"` gives one entry; `\"`, `\\`, `\n`, `A` decode; a
  table header, an array, an unquoted value, an upper-case key and a U+0000 each fail
  naming the line. `Locales::load` on a temp dir: missing `default.toml` fails; a key
  missing from `french.toml` fails naming both; an extra key fails; `caption` in
  `default.toml` fails; `unknown.toml` fails naming the file; `index` is `2 + position`
  in byte-sorted order and ignores reserved keys.
- [ ] **Step 2:** `cargo test -p symdev-locale --offline` — fails (crate does not exist).
- [ ] **Step 3:** Implement the three files.
- [ ] **Step 4:** Tests pass; clippy clean.
- [ ] **Step 5:** Commit: "Add symdev-locale, the one reader of per-language locale files."

### Task 2: symdev compiles and installs the strings files

**Files:**
- Create: `crates/symdev-build/src/strings_resources.rs` (the `.rss` text, the paths and
  install destinations), `crates/symdev-build/src/driver/strings_build.rs` (compile step)
- Modify: `crates/symdev-build/src/driver/rust_build.rs` (call it after the EXE, extend
  artifacts), `crates/symdev-build/Cargo.toml` (depend on `symdev-locale`),
  `crates/symdev-build/src/lib.rs`

**Interfaces:** consumes Task 1. Produces
`StringsResources { app: String, locales: Locales }` with `rss(&self, table: &Table) ->
String`, `rsc_path(build_dir, Option<Language>)`, `dest(Option<Language>)`, and
`RustBuild::build_strings(&self, project, build_dir) -> Result<Vec<Artifact>>`.

The generated source, for every language (`default` → `.rsc`):

```
// Generated by symdev from locales/<file>. Do not edit.
NAME STRS
STRUCT SYMDEV_SIG { LONG signature = 4; SRLINK self; }
STRUCT SYMDEV_STR { BUF8 text; }
RESOURCE SYMDEV_SIG { }
RESOURCE SYMDEV_STR { text = "Hello"; }            // key 0 → index 2
RESOURCE SYMDEV_STR { text = <0xd0><0x9f>"ok"; }   // non-ASCII bytes as <0xNN>
```

Printable ASCII other than `"` and `\` is written literally; every other byte as `<0xNN>`.
No `#include`: the file must compile for a console project with no Avkon headers.

- [ ] **Step 1: Failing tests** in `strings_resources/tests.rs`: the `.rss` for a two-key
  table matches the text above; a value with `"`, `\`, `é`, `П` and a newline is written
  as literal/escape runs; compiling that `.rss` with `symdev_rcomp` and reading resource
  `2 + i` back from the compiled bytes gives exactly the value's UTF-8 for every key
  (this is the test that proves `<0xNN>` in `BUF8` round-trips bytes 0x80–0xFF);
  `rsc_path(None)` is `build/<app>_strings.rsc`, `rsc_path(Some(french))` is
  `…_strings.r02`; `dest` is `!:\resource\apps\<app>_strings.<ext>`.
- [ ] **Step 2:** Run; they fail.
- [ ] **Step 3:** Implement; wire `build_strings` into `RustBuild::build` for any Rust
  project with a `locales/` directory.
- [ ] **Step 4:** Tests pass. Build `symbian-rs/examples/locale` after Task 5's
  `locales/` exist — or a scratch copy with a `locales/` directory now — and list
  `build/*_strings.*`.
- [ ] **Step 5:** Commit: "Compile each locales file into a per-language strings resource
  and install them all."

### Task 3: the run-time reader

**Files:**
- Create: `symbian-rs/shims/common/symrs_rsc.cpp`,
  `symbian-rs/crates/symbian-core/src/locale/strings.rs` (the process-wide open file,
  `Str`, `Text`)
- Modify: `symbian-rs/shims/common/symrs_shim.h` (declarations),
  `symbian-rs/crates/symbian-sys/src/shim.rs` (extern declarations),
  `symbian-rs/crates/symbian-sys/src/des.rs` (bind `TDesC8::Ptr`, `_ZNK6TDesC83PtrEv`),
  `symbian-rs/crates/symbian-core/src/locale/mod.rs`

**Interfaces:** produces `symbian_core::locale::Str` (`pub const fn at(index: u16) ->
Str`, `pub fn get(self) -> Result<Text>`) and `Text` (`Deref<Target = str>`, `Drop` frees
the `HBufC8` with `User::Free`, `Display`).

The shim, the rule from `symrs_shim.h` followed (leaving calls under `TRAP`, argument
checks, no Rust frame inside):

```cpp
// Opens the nearest language variant of aPath and confirms its signature.
SYMRS_EXPORT TInt symrs_rsc_open(RFs* aFs, const TDesC16* aPath, RResourceFile* aFile)
    {
    if (!aFs || !aPath || !aFile) return KErrArgument;
    new (aFile) RResourceFile;
    TFileName name(*aPath);
    BaflUtils::NearestLanguageFile(*aFs, name);
    TRAPD(err, aFile->OpenL(*aFs, name); aFile->ConfirmSignatureL(0));
    if (err != KErrNone) aFile->Close();
    return err;
    }

// AllocReadL(Offset() + aIndex): the caller frees *aOut with User::Free.
SYMRS_EXPORT TInt symrs_rsc_read(const RResourceFile* aFile, TInt aIndex, HBufC8** aOut)
    {
    if (!aFile || !aOut) return KErrArgument;
    *aOut = NULL;
    TRAPD(err, *aOut = aFile->AllocReadL(aFile->Offset() + aIndex));
    return err;
    }
```

`RResourceFile` is `KRscFileSize = 24` bytes (`barsc.h:59`); Rust holds it in a 24-byte,
4-aligned static. The file is opened lazily on the first `get`, from the path built as
`<drive of RProcess::FileName()>\resource\apps\<stem>_strings.rsc`
(`symbian_core` already has the process-file-name shim), and kept for the life of the
process — the same lifetime a C++ application gives its resource file.

- [ ] **Step 1:** There is no host test for device code; the test is the example in Task
  5. Write the shim and the Rust side.
- [ ] **Step 2:** `cargo build --release` in `symbian-rs/` for a crate using it; clippy.
- [ ] **Step 3:** Commit: "Read localised strings on demand from the nearest-language
  resource file, as C++ does."

### Task 4: `symbian_std::strings!()`

**Files:**
- Create: `symbian-rs/crates/symbian-macros/src/strings.rs`
- Modify: `symbian-rs/crates/symbian-macros/src/lib.rs`,
  `symbian-rs/crates/symbian-macros/Cargo.toml` (path dependency on `symdev-locale`),
  `symbian-rs/crates/symbian-std/src/lib.rs` (re-export)

**Interfaces:** `symbian_std::strings!();` at a module's top level expands to

```rust
pub mod strings {
    const _: &str = include_str!("<abs>/locales/default.toml");   // one per file: rebuild
    pub const GREETING: ::symbian_std::locale::Str = ::symbian_std::locale::Str::at(2);
}
```

reading `$CARGO_MANIFEST_DIR/locales` with `symdev_locale::Locales::load` — the same
reader and the same index as Task 2, so the two cannot disagree. A load error becomes
`compile_error!` with the message.

- [ ] **Step 1: Failing tests** (host, in `strings.rs`): expansion text for a two-key
  temp dir; the index matches `Locales::index`; a missing key in `french.toml` expands to
  a `compile_error!` naming it.
- [ ] **Step 2–4:** Implement, pass, clippy.
- [ ] **Step 5:** Commit: "Generate one string constant per locales key, checked at
  compile time."

### Task 5: the example, the deletion, and the C++ comparison

**Files:**
- Create: `symbian-rs/examples/locale/locales/{default,french,ukrainian}.toml`
- Modify: `symbian-rs/examples/locale/src/main.rs`; delete
  `symbian-rs/examples/locale/src/strings.rs`,
  `symbian-rs/crates/symbian-std/src/locale.rs` (the in-image `locale!`),
  `symbian-rs/crates/symbian-core/src/locale/lang.rs`, and `Language::base` if nothing
  else uses it.

The example measures what the C++ baseline measured, with `alloc_cells` (cells) — and
bytes if `User::AllocSize` is bound: before the first `get`, after it (file open + one
string held), after the string is dropped. Expected, against C++: open +4 cells, string
+1, drop −1. It also checks each string's text for the device language.

- [ ] **Step 1:** Write the locales and the new `main.rs`.
- [ ] **Step 2:** `symdev build && symdev package && symdev test --emulator` at
  `language: 1` (English) and `language: 2` (French) in `~/.local/share/EKA2L1/config.yml`
  (restore it after). Ukrainian cannot be selected on this ROM (experiment 96).
- [ ] **Step 3:** Sizes of every example against `main`; `locale` against C++'s 6 647.
- [ ] **Step 4:** Commit: "Move the locale example onto per-language resource files and
  delete the in-image table."

### Task 6: the caption

**Files:**
- Modify: `crates/symdev-build/src/ui_resources.rs` (`app_rss` takes the caption pair),
  `crates/symdev-build/src/driver/ui_build.rs` (compile `<app>.rsc` plus one `<app>.rNN`
  per variant that has `caption`), tests.

- [ ] **Step 1: Failing test:** with `locales/french.toml` holding `caption = "Barres"`,
  `build_ui` produces `<app>.rsc` (manifest caption) and `<app>.r02` whose
  `LOCALISABLE_APP_INFO` caption is `Barres`; artifacts install both.
- [ ] **Step 2–4:** Implement, pass.
- [ ] **Step 5:** Verify in the emulator at `language: 2`: the title pane of
  `examples/ui` shows the French caption (screenshot), and at `language: 1` the English.
- [ ] **Step 6:** Commit: "Localise the launcher caption through per-language application
  resources."

### Task 7: record it

- [ ] Experiment entry in `docs/research/experiment-backlog.md` with every number against
  C++; fold `docs/research/wip/launcher-locale.md` into it and delete the wip file;
  update `docs/research/cpp-parity.md`'s locale row.
