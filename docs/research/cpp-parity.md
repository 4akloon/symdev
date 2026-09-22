# C++ parity baseline for the `no_std` Rust examples

Written 2026-09-22 on branch `cpp-baseline`. This is the ruler the Rust SDK is measured
against: four C++ programs, built with the same GCC 12.1 / binutils 2.29.1 toolchain through
`symdev build` (no Wine), that do what `symbian-rs/examples/{hello,files,ui,locale}` do.
It measures; it proposes no fixes.

**Rust side measured at `4c5fc5e`** (the `main` this branch forked from). `main` has since
installed `CTrapCleanup::New()` in the `no_std` console entry (`5cad04f`), which the
coordinator measured at +29…+44 B of E32 per console example — see the CTrapCleanup row.

Sources: `docs/research/cpp-parity/{hello,files,locale,ui}/` (each a `symdev.toml` +
`group/bld.inf` + `.mmp` project), `common/symdevreport.{h,cpp}` (the C++ counterpart of
`symbian_std::test_report::Report`: same path, same JSON, so `symdev test --emulator` reads
both), `common/symdevprobe.h` + `probe.rs` (heap/tick probe, both halves), `measure.py`
(ELF + E32 header reader), `measure-all.sh` (rebuild + measure all eight), `count.py` (DX).

## How every number was produced

| Axis | Command / method |
|---|---|
| Sections, imports, E32 header | `docs/research/cpp-parity/measure-all.sh` → `measure.py <x>.elf <x>.exe`. Sections from the linked ELF before elf2e32. Imports from the ELF (`DT_NEEDED`, and relocations against undefined symbols grouped by symbol version = defining DLL), i.e. the rule `E32ImportSection::from_elf` applies; the E32 body is Symbian LZ77+Huffman (UID `0x101F7AFC`), not RFC 1951, so it is not decompressed. Header fields are read from the uncompressed E32 header. |
| Where the bytes go | `arm-none-symbianelf-nm -S --size-sort` on the ELF, symbols summed by origin (mangled crate path for Rust; `CSymdevReport`, `__gxx_personality_v0`/`__gnu_unwind_*`/`*encoded_value*`/`*lsda*` for C++ EH). |
| Heap | `User::AllocSize(TInt&)` (e32std.h:4484): returns the allocated **cell count**, writes the **total bytes** in allocated cells. Not `RHeap::Size()`, which e32cmn.inl:78 defines as "the total number of bytes committed by the host chunk". There is no `RAllocator::AllocSize()` in this SDK; `RHeap::AllocSize` is the virtual `User::AllocSize` forwards to. Same call, same order, both sides. |
| Startup | `User::NTickCount()` (euser `_ZN4User10NTickCountEv`). **No SDK header states its period.** `UserHal::TickPeriod` states the *system* tick period: measured **15 625 µs** on both sides. The nanotick period is **1 000 µs inside EKA2L1** (recorded in `symbian-core/src/time.rs`, experiment 85), consistent with 100 000 `User::Language()` calls = 12 nanoticks on both sides below. Emulator timings are not device timings. |
| Probe runs | Probe on: C++ `MACRO SYMDEV_CPP_PARITY_PROBE` (commented out in each `.mmp` by default); Rust: commit `cafa01a` added `#[path] mod probe;` to the three examples, reverted in `4acc3eb`. `SYMDEV_SIGN_PASSWORD=… symdev package && symdev test --emulator` in each project dir. P0 = first statement of the app's own entry (`E32Main` / Rust `fn main`; for GUI: first line of `ConstructL` / `construct`), P1 = after all work, before the report is written. |
| DX | `count.py <dir>` (raw lines / lines that are neither blank nor comment), C++ counted with the `#ifdef SYMDEV_CPP_PARITY_*` blocks removed. Generated `build/` excluded. |

## hello — `User::InfoPrint` a formatted line, `User::After(5 s)`, exit 0

Equivalence: exact. Both format `"<greeting> (<n> chars)"` into a 64-unit stack buffer; C++
with `TDes16::Format`, Rust with `write!` into `Buf16<64>`. Neither installs a cleanup stack
(the Rust one did not at `4c5fc5e`). (The task brief described `hello` as writing a result
file; the source at `4c5fc5e` does not — it InfoPrints.)

| | C++ | Rust | Rust/C++ |
|---|---:|---:|---:|
| `.text` | 232 | 3 524 | 15.2× |
| `.rodata` | 76 | 268 | |
| `.plt` / `.ARM.extab` / `.ARM.exidx` | 112 / 44 / 56 | 104 / 44 / 56 | |
| `.data` / `.bss` | 0 / 0 | 0 / 0 | |
| E32 `iCodeSize` / `iDataSize` / `iBssSize` | 736 / 0 / 0 | 4 216 / 0 / 0 | 5.7× |
| E32 file (compressed) | **802** | **3 187** | **4.0×** |
| DLLs / ordinals | 2 / 14 | 2 / 13 | |
| Files / lines (raw / code) | 4 / 68 / 43 | 3 / 57 / 39 | |

**Verdict — size: Rust loses 4.0× (+2 385 B).** Of the Rust image's 3 710 B of code symbols,
**3 172 B (85 %) is `core::fmt`**: `<u32 as Display>::fmt` 908, `<str as Display>::fmt` 784,
`core::fmt::write` 536, `Formatter::padding`/`pad_integral`/`write_prefix` ~290,
`Buf16::write_str`/`write_char` 636. C++ formats with `TDes16::Format`, **a euser export: zero
bytes in the image, one import ordinal**. Mechanism: `write!` monomorphises and statically links
the `core::fmt` state machine plus integer/str `Display` into every image that formats anything,
while the platform already ships a formatter in ROM. Imports: parity (13 vs 14).

### CTrapCleanup row (asked by the coordinator)

`docs/research/cpp-parity/hello`, uncomment `MACRO SYMDEV_CPP_PARITY_CLEANUP` in
`group/cpphello.mmp`, `symdev build`, `measure.py hello/build/cpphello.elf hello/build/cpphello.exe`
(commit `6bf3bca`; it was `ae0905c` before the history rewrite, same content):

| | without | with | Δ C++ | Δ Rust (coordinator, `nostd-cleanup`) |
|---|---:|---:|---:|---:|
| E32 file | 802 | 833 | **+31** | +44 (hello) |
| `iCodeSize` | 736 | 768 | +32 | — |
| `.text` / `.plt` | 232 / 112 | 256 / 120 | +24 / +8 | 16 B of it is `symrs_cleanup_destroy` |
| ordinals | 14 | 15 | +1 (`CTrapCleanup::New()` only) | |

`delete cleanup` adds **no** import: `~CTrapCleanup` is reached through the vtable and
`CBase::operator delete` is inline. `E32Main` grows 52 → 76 B. Rust pays +13 B (1.42×) over
C++ for the same thing, which the 16-byte C++ shim hop (`~CTrapCleanup` is virtual and cannot be
called from Rust) accounts for; that is a floor on every `no_std` console image.

## files — create dir ×2, write+flush+close, metadata, read back, seek+read 3, missing file, write-and-keep, rename, delete, JSON report

Equivalence: same 16 behavioural cases, all pass on both sides in EKA2L1
(`cppfiles: 20 passed`, `filesdemo: 19 passed`, the extra cases being probe lines). Differences:
C++ uses `_L8` literals and hand-written `KErrNotFound` checks where Rust has `ErrorKind`;
the C++ report builds the JSON incrementally in one `RBuf8`, Rust keeps a `Vec<Case>` of owned
`String`s and formats at the end; C++ uses `CTrapCleanup` + `TRAPD` (idiomatic, and required by
`CSymdevReport::NewL`), Rust at `4c5fc5e` had no cleanup stack.

| | C++ | Rust | Rust/C++ |
|---|---:|---:|---:|
| `.text` | 6 560 | 13 436 | 2.05× |
| `.rodata` | 1 084 | 2 240 | |
| `.plt` / `.ARM.extab` / `.ARM.exidx` | 544 / 236 / 240 | 320 / 44 / 56 | |
| `.data` / `.bss` | 0 / 0 | 0 / 24 | |
| E32 `iCodeSize` / `iBssSize` | 8 880 / 0 | 16 316 / 24 | 1.84× |
| E32 file | **6 058** | **10 552** | **1.74×** |
| DLLs / ordinals | 4 / **71** | 3 / **39** | **0.55×** |
| heap at entry (cells / bytes) | 1 / 36 | 1 / 36 | = |
| heap at end (cells / bytes) | 7 / 1 324, 1 292, 1 292 | 19 / 1 132 ×3 | 2.7× cells, 0.87× bytes |
| nanoticks entry → end (3 runs) | 6, 8, 8 | 9, 8, 9 | noise |
| Files / lines (raw / code) | 4 / 216 / 166 + shared report 2 / 228 / 191 | 3 / 151 / 100 | |

Where the code goes. C++ (6 826 B of code symbols): **GCC C++ EH runtime 3 648**
(`__gxx_personality_v0` 1 764, `__gnu_unwind_execute` 1 056, …) pulled in by `TRAP`/`new (ELeave)`,
report helper 2 480, app ~330, eexe startup 366. Rust (13 686 B): `core::fmt` **4 012**,
the inlined `main`/`run` **3 836**, `symbian_std::fs`/`io` 2 148, `symbian_core` 1 132,
`test_report` 1 076, `alloc` (RawVec growth, String) 780, eexe startup 366; EH runtime 16.

**Verdicts.** Size: **Rust loses 1.74× (+4 494 B)** even though it *avoids* 3.6 KB of EH runtime
the C++ side pays — so the real Rust excess over a like-for-like program is ~8 KB, and the first
4 KB of it is `core::fmt` again (every `checked()` formats the error with `{e:?}`; `check_detail`
takes `fmt::Arguments`). Second mechanism: the `std`-shaped `fs` layer (`OpenOptions::open`
1 000 B, `create_dir_all` 392, `metadata` 388, `fs::write` 152) is statically linked Rust over the
same 15 efsrv ordinals C++ calls directly. Imports: **Rust wins, 39 vs 71 ordinals, 3 vs 4 DLLs**
— C++'s descriptor/cleanup-stack/`CBase` idiom imports 40 euser ordinals and `scppnwdl`
(`operator new(ELeave)`); Rust uses 14. Heap: **Rust wins on bytes (-13 %), loses on cells (2.7×)**:
one `String` per case name and detail, where C++ appends to one doubling buffer. Startup:
identical heap at entry (the eexe path is the same code); tick deltas indistinguishable at 1 ms.

## locale — the same strings in three languages, picked by the device language

Equivalence: **not exact, by construction.** The C++ side uses the SDK's mechanism: one `.rls`
per language, `LANG SC 01 02 93` in the `.mmp`, `CHARACTER_SET UTF8`, a compiled resource file per
language (`.r01` 65 B, `.r02` 73 B, `.r93` 66 B, `.rsc` 65 B installed), picked at run time by
`BaflUtils::NearestLanguageFile`. Rust keeps the table in `.rodata` and picks a column. Things the
C++ API cannot express, so the C++ program does not do them:
the six `Text::get_in(language)` fallback cases (`NearestLanguageFile` reads `User::Language()`
itself — there is no per-language lookup), and `Language::base()` (the C++ `TLanguage` is flat).
The measurement of `User::Language()` cost is identical code on both sides.

**Unresolved: the C++ program cannot read its own strings inside EKA2L1.** In order: a resource
path without a drive resolves against the session drive (fixed by taking the drive from
`RProcess().FileName()`, which the SDK idiom requires and Rust does not need); then
`RResourceFile::ConfirmSignatureL` panics **BAFL 4** — no argument, with 4
(`EEikResourceSignatureValue`), and with `NAME APLC` (small value) as with `NAME CPLC` alike;
without it `Offset()` stays 0, `OwnsResourceId(R_GREETING)` is false and `ReadL(R_GREETING)`
leaves `KErrNotFound`; `AllocReadL(index)` panics BAFL 4. The file's bytes match
`rcomp-spec.md` §4 (UID1 `0x101F4A6B`, flags `0x01`, largest-resource 28, packed bits `0e`,
index at 0x37) and have the same shape as `uidemo.rsc`, which `CCoeEnv` reads fine. Not isolated
between symdev's rcomp and EKA2L1's/ROM bafl. The C++ run therefore reports
`FAIL the table came through: TInt -1` (8 of 9 pass); the program as committed omits
`ConfirmSignatureL`, so a working C++ version would carry one more bafl ordinal.

| | C++ | Rust | Rust/C++ |
|---|---:|---:|---:|
| `.text` | 7 004 | 11 396 | 1.63× |
| `.rodata` | 1 084 | 2 388 | |
| `.plt` / `.ARM.extab` / `.ARM.exidx` | 584 / 352 / 248 | 224 / 44 / 56 | |
| `.bss` | 0 | 24 | |
| E32 `iCodeSize` | 9 492 | 14 328 | 1.51× |
| E32 file | **6 647** | **9 684** | **1.46×** |
| + installed resources | 269 B (4 files) | 0 | |
| DLLs / ordinals | 5 / **76** | 3 / **27** | **0.36×** |
| heap entry / end (cells / bytes) | 1/36 → 8/1 840 | 1/36 → 12/1 200 | 1.5× cells, 0.65× bytes |
| nanoticks entry → end | 28, 29, 30 | 21 | |
| 100 000 × `User::Language()` | 12 nanoticks | 12 nanoticks | = |
| Files / lines (raw / code) | 9 / 338 / 232 + report | 4 / 213 / 149 | |

**Verdicts.** Size: **Rust loses 1.46× (+3 037 B E32, +2 768 B counting C++'s resource files)**;
mechanism is again `core::fmt` (4 364 B — every `writeln!` into the notes `String`) plus
`test_report` 864. Imports: **Rust wins big, 27 vs 76 ordinals, 3 vs 5 DLLs** (C++ needs `bafl`
for `BaflUtils`/`RResourceFile`/`TResourceReader`). Heap: **Rust wins on bytes (1 200 vs 1 840)**;
the C++ figure *excludes* the three `HBufC`s a working read would allocate (they are freed before
P1, so only fragmentation would show). Time: the C++ run is 7–9 ms slower entry→end, consistent
with the file-system round trips of `NearestLanguageFile` + `RResourceFile::OpenL`; one Rust run,
emulator only. The euser call itself costs the same on both sides.
DX: see below — this is the pair where Rust's advantage is structural.

## ui — Avkon app: bars, Up/Down/Select, Options menu (More bars / Fewer bars / Reset / Exit), Exit softkey

Equivalence: same drawing (white clear, 20-px bars at 30-px pitch, blue fill, black pen,
baseline, `bars=N keys=K cmd=C` in the title font), same keys, same four menu items, same
three report cases. C++ is the SDK's `helloworldbasic` shape: Application / Document / AppUi /
View, `.hrh` command enum, `.rss` + `_reg.rss` + `.rls`, `R_AVKON_SOFTKEYS_OPTIONS_EXIT`.
Rust's C++ shim builds the CBA from `symdev.toml` and fills the menu at `DynInitMenuPaneL`.
Both report `the view was sized before construct: 240x245` and pass 7/7 in EKA2L1.
Pixels were not re-compared in this session.

| | C++ | Rust | Rust/C++ |
|---|---:|---:|---:|
| `.text` | 7 020 | 17 192 | 2.45× |
| `.rodata` | 1 388 | 1 892 | |
| `.plt` / `.ARM.extab` / `.ARM.exidx` | 640 / 236 / 392 | 608 / 216 / 312 | |
| `.bss` | 0 | 24 | |
| E32 `iCodeSize` | 9 896 | 20 436 | 2.07× |
| E32 file | **7 317** | **13 714** | **1.87×** |
| + `.rsc` / `_reg.rsc` / `_aif.mif` | 307 / 89 / 268 | 234 / 91 / 268 | |
| DLLs / ordinals / slots | 8 / 225 / 237 | 9 / 219 / 230 | ≈ |
| heap at `construct` entry → end | 775/61 916 → 778/62 248 (+3 / +332) | 776/62 080 → 782/62 416 (+6 / +336) | +1 cell / +164 B resident |
| nanoticks construct | 2 | 2 | = |
| Files / lines (raw / code) | 18 / 520 / 416 + report | 4 / 279 / 192 | |

Where the code goes (Rust, 17 588 B of symbols): `symbian_ui` **4 820** (the generic
`vtbl::construct<Bars>` alone 2 212, `vtbl::draw<Bars>` 1 360), **`core::fmt` 3 196** — present
even though the example avoids `format!` in `draw` on purpose, because `Report::check_detail`
and `checked` format; the C++ shim (`CShimAppUi`/`CShimView`/`Host*`) ~1 800, GCC EH runtime
3 664 (same as C++'s 3 648 — the shim uses `TRAP`), `test_report` 872, `alloc` 752.

**Verdicts.** Size: **Rust loses 1.87× (+6 397 B)**. Mechanisms, in order: (1) the generic
`symbian_ui::vtbl::*<App>` thunks are monomorphised per application and `construct<Bars>` is
2.2 KB — the C++ app's whole `ConstructL`+`AppUi` is a few hundred bytes of calls into cone/eikcore;
(2) `core::fmt` 3.2 KB pulled in by the test report, which a shipping app without the report
would not carry — so for UI, `test_report`'s formatting is part of the measured gap;
(3) the C++ shim is a second copy of the Avkon class skeleton (~1.8 KB) on top of the Rust side.
Imports: parity (219 vs 225 ordinals). Heap: parity (the framework's ~62 KB dominates; the Rust
app holds 164 B more at `construct`, i.e. its boxed state + vtable glue). Startup: identical at the
bracket measured.

## DX — counted

| Pair | C++ files / code lines | Rust files / code lines | Hand-kept-in-sync in C++ (count) | Same in Rust |
|---|---|---|---|---|
| hello | 4 / 43 | 3 / 39 | UID3 ×3 (`.mmp` UID, SECUREID, `symdev.toml`) | UID3 ×1 |
| files | 4 / 166 (+2 / 191 report) | 3 / 100 | UID3 ×4 (+ `KUid3` in source); error classification by hand (`== KErrNotFound`, `KErrAlreadyExists` as success) | UID3 ×1 (`Report::new` reads `SYMDEV_UID3`) |
| locale | 9 / 232 (+ report) | 4 / 149 | language list ×2 (`LANG` line, `.rls` `#elif` switch); each key ×(N languages + `.rss`); drive letter supplied by hand; `ConfirmSignatureL` must be called for ids to resolve | languages ×1 (`locale!` header) |
| ui | 18 / 416 (+ report) | 4 / 192 | UID3 ×6 (mmp ×2, toml, `AppDllUid`, `_reg.rss`, report); each menu command ×3 (`.hrh` enum, `.rss` `MENU_ITEM`, `HandleCommandL` switch) plus its label in `.rls`; `.rsg` name in `_reg.rss` | UID3 ×1; each menu item ×1 (label + closure in `App::menu`) |

What the compiler catches:

* **C++, not caught:** a menu command present in the `.rss` but missing from the `switch` (silently
  ignored); a `CleanupStack` push/pop imbalance (run-time `E32USER-CBase` panic); a `TBuf<32>`
  `Format` that overflows (run-time `USER 11` panic); a resource path without a drive, and a
  resource file read without `ConfirmSignatureL` (run-time `KErrNotFound`, found only by running —
  this session lost several emulator runs to exactly these two).
* **C++, caught at build:** a translation missing from one `.rls` (rcomp: undefined
  `STRING_r_…`), a resource id typo (`.rsg` constant), a command-enum typo (`.hrh`).
* **Rust, caught at build:** a missing translation column (`locale!`), a menu item without an
  action (it is the closure), a non-`Result` error path (`?`). Buffer overflow in `Note`/`Buf16`
  truncates or returns `Err` instead of panicking.

**Verdict — DX: Rust wins** by 9 % (hello), 40 % (files), 36 % (locale) and 54 % (ui) fewer code lines, 1 vs 3–6 places for the
UID3, 1 vs 3 places per menu command, and it has no class of run-time-only resource/cleanup-stack
failure. C++ additionally needs the 191-line report helper that Rust's SDK provides.

## Summary

| Axis | hello | files | locale | ui |
|---|---|---|---|---|
| E32 size | Rust **4.0×** worse | Rust 1.74× worse | Rust 1.46× worse | Rust 1.87× worse |
| Import ordinals | = (13/14) | Rust **wins** 39/71 | Rust **wins** 27/76 | = (219/225) |
| Heap bytes at end | — | Rust wins −13 % (2.7× cells) | Rust wins −35 % (1.5× cells) | = |
| Startup (entry heap, ticks) | — | = | Rust faster (no resource I/O) | = |
| DX (code lines, sync points) | ≈ | Rust wins | Rust wins, structurally | Rust wins 2.2× |

**Where Rust loses worst, and the mechanism:**

1. **`core::fmt` is linked into every image that formats anything: 3.2–4.4 KB per example**,
   85 % of `hello`. `write!`/`writeln!`/`format_args!` monomorphise `core::fmt::write`,
   `Formatter::pad`/`pad_integral`/`padding` and the `Display` impls for `u32`/`str`/`i32`; C++
   calls `TDes::Format`/`AppendFormat`/`Num` in euser (ROM) — zero image bytes. In `files`,
   `locale` and `ui` a large share is reached through `symbian_std::test_report`
   (`checked` → `{e:?}`, `check_detail(fmt::Arguments)`, `to_json` → `write!`).
2. **Generic Avkon glue monomorphised per app**: `symbian_ui::vtbl::construct<App>` 2 212 B,
   `draw<App>` 1 360 B, `offer_key<App>` 184 B, `menu::item<App>` 160 B in `uidemo`, on top of a
   ~1.8 KB C++ shim that duplicates the Avkon class skeleton a C++ app writes once.
3. **The `std`-shaped fs layer is static Rust over the same efsrv calls**: `OpenOptions::open`
   1 000 B, `create_dir_all` 392, `metadata` 388, `File::create_new`/`open` 232 each — 2.1 KB in
   `files`, whereas C++ `RFile::Replace/Open` are direct imports.
4. **Many small heap cells**: `Report` stores one `String` per case name and detail (19 cells vs 7
   in `files`); fewer bytes than C++ but 2.7× the cells — relevant on a heap whose cell minimum is
   36 B payload (experiment 68).
   Plus, for console apps on today's `main`: the `CTrapCleanup` shim hop (+13 B over C++).

**Where Rust already wins:** import ordinals (39 vs 71, 27 vs 76 — C++'s descriptor/cleanup-stack
idiom imports far more of euser and needs `scppnwdl`/`bafl`); **no GCC EH runtime in console
images** (C++ pays 3 648 B the moment it uses `TRAP`); heap bytes; locale without resource files
or run-time file I/O; DX on every axis counted.

**Not measured:**
* Process start → first app instruction. From inside the guest only entry → end is observable;
  both sides share `eexe.lib`'s `_E32Startup` and show the identical heap at entry
  (1 cell / 36 B), so nothing language-specific runs before `E32Main`/`main`.
* Any device timing: all ticks are EKA2L1 at 1 ms resolution; the entry→end deltas (2–30 ticks)
  are within noise except locale.
* C++ locale's string read (unresolved BAFL 4 above), hence its heap-with-strings and the six
  per-language fallback cases, which the C++ API cannot express at all.
* Pixel comparison of the two UI apps and a key/menu drive of the C++ one.
* `hello` heap/ticks: the pair writes no report, and adding one would change what is measured;
  `files`' P0 is the same console entry path.

## symdev findings from building the C++ side

* All four `.mmp` projects build natively through `symdev build`, including `LANG SC 01 02 93`,
  `CHARACTER_SET UTF8` Cyrillic `.rls`, `START RESOURCE … HEADER`, `MACRO`, multiple
  `SOURCEPATH`, `[symbian] icon` → `_aif.mif`. Only warning: `EPOCSTACKSIZE is parsed but nothing
  on the GCCE path reads it`.
* `symdev build` writes `build/sdk-include-casefold/` into the project directory: ~1 000
  case-folding **symlinks into the SDK's `epoc32/include`** (e.g. `AKNEDSTSOBS.H ->
  …/epoc32/include/aknedstsobs.h`; `find -type l` = 260 at the top level of one project, one
  regular file). Git stores a symlink's target path, not the header, so no SDK *content* was
  committed — but the SDK's file list and host paths were, which the repository rule forbids.
  `.gitignore` covers `/examples/*/build/` and `/symbian-rs/examples/*/build/` only; a project
  anywhere else gets these links into `git add -A`. That is how this branch committed 1 044 of
  them (since purged from history; `docs/research/cpp-parity/.gitignore` now has `build/`).
* The resource-read failure in the locale section, pending isolation between rcomp output and
  EKA2L1/ROM bafl.
