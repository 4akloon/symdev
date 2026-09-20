# Rust SDK for Symbian: validated design (2026-09-20)

Status: **approved to start** by the repository owner on 2026-09-20. This supersedes the
"Do not implement B" verdict in [rust-sdk-idea.md](../../research/rust-sdk-idea.md); that
note's three-product split (A tooling / B app SDK / C replace Nokia) still holds, and this
document is B.

This is the document to hand to any agent that will work on the Rust SDK. It records
(1) what symdev already has, so nobody rebuilds it, (2) where the owner's master prompt
is right, where it is out of date, and where it guesses, (3) the design that fits the
pipeline we actually have, and (4) the experiments that turn every "Hypothesis" below
into a fact. Nothing here is verified on a stock E52; see §0.

Sources: the master prompt (2026-09-20, in the conversation, not in-tree — not an
authority, per the north-star's §1 rule); [experiment-backlog.md](../../research/experiment-backlog.md)
experiments 5–63; the recorded link line in `crates/symdev-build/src/driver/link.rs`; the
captured SDK makefile (experiment 63); `CLAUDE.md`.

## 0. Two things to say before anything else

**Hardware M0 is still open.** `CLAUDE.md`: "Do not claim E52 support until a stock device
installs and launches the app; the emulator is not a device." No symdev output has been
installed on a stock E52 yet. `rust-sdk-idea.md` made that the precondition for B so that a
Rust failure is never confused with a SIS or signing failure. The owner chose to start
anyway; the consequence is that until a C++ hello runs on the phone, every Rust milestone
is "runs in EKA2L1", and the device column of every table below stays empty.

**The master prompt describes a backend symdev has already replaced.** Its §35–§38
("integrate with the legacy Symbian build ecosystem", "generate intermediate MMP", "bldmake
/ abld / GCCE / rcomp / makesis / signsis", "Windows VM / Rosetta / remote Windows
environments") are the situation of 2026-09-16. Since then every one of those tools has a
native, byte-verified Rust replacement in this repository and `symdev build` needs no Wine,
no Windows, no Perl. The Rust SDK does not integrate with the legacy toolchain; it plugs a
second *compiler* into a pipeline whose every later stage is already ours. That changes the
MVP list (§46 of the prompt) from ten items to about three.

## 1. What symdev already has

Everything in this table is native Rust in this repository, runs on Linux with no Wine,
and is verified the way the "Evidence" column says. Experiment numbers refer to
[experiment-backlog.md](../../research/experiment-backlog.md).

| Pipeline stage | Where | Evidence |
|---|---|---|
| `bld.inf` / `.mmp` front end: cpp pass, the full directive set, `START RESOURCE`, `START BITMAP`, `PRJ_EXPORTS` | `symdev-build` (`bld/`, `mmp/`, `project/`) | exp 61; spec written clean-room from the SDK Perl, then confirmed against the SDK's own generator run on Linux (exp 63) |
| C++ and C compile (`arm-none-symbianelf-g++`, `-x c` for `.c`), `gcce.h` varargs repair for GCC 12 | `driver/compile.rs`, `driver/language.rs`, `driver/gcce_compat.rs` | exp 5, 51, 59 |
| Link (`arm-none-symbianelf-ld` 2.29.1, recorded argv, `eexe.lib`/`edll.lib`, `usrt2_2.lib`, the six runtime DSOs) | `driver/link.rs` | exp 5; DSO import libs resolved case-insensitively (gap 9) |
| Post-link ELF → E32 (`elf2e32`): headers, imports by ordinal, relocations, deflate, exports incl. frozen `.def`, DLL data | `symdev-elf2e32` | exp 45–47, 52; byte-equal to `elf2e32_next` for EXE and DLL |
| Resource compiler (`cpp` + `rcomp`): `.rss` → `.rsc`/`.rsg`, SCSU text compression, registration resources | `symdev-rcomp` | exp 56: 143/143 SDK example resources byte-equal both ways |
| Icons: SVG → SVGB → `.mif` + `.mbg` (`svgtbinencode` + `mifconv`) | `symdev-mif` | exp 57, 60: 416/416 byte-equal incl. a real Illustrator icon |
| Bitmaps: BMP → `.mbm` + `.mbg` (`bmconv`), all depths, all four RLE encoders | `symdev-mbm` | exp 58: 288/288 byte-equal |
| SIS packaging (`makesis`) and signing (`signsis`, `makekeys`, self-signed DSA) | `symdev-sis`, `symdev-makekeys` | T2 series, exp 43; installs in EKA2L1 |
| UID CRC | `symdev-uidcrc` | T1 |
| Manifest (`symdev.toml`): package, uid3, capabilities (user-grantable set), icon, `[[install]]`, signing | `symdev-manifest` | tests; gap 12 |
| Emulator run: install SIS, launch UID3, pid + log files | `symdev-emulator` (EKA2L1 as a separate GPL process, our fixes upstream in PRs #724–#728) | `symdev run`; the screenshot loop in the `eka2l1-host` skill |
| Third-party reality check | Simon Tatham's Puzzles port: 55 `.c` + 11 `.cpp`, 26 libraries, 389-line `.rss`, 34 bitmaps | gaps 1–12, 14 closed; 13/15 in progress (icon containers in the manifest) |

Not there, and relevant to the Rust SDK:

- `symdev test`, `symdev debug`, `symdev logs`, `symdev devices`, `symdev doctor`, a toolchain manager, `symdev.lock`. The toolchain is six environment variables (`SYMDEV_EPOCROOT`, `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB`, `SYMDEV_EKA2L1`).
- Any physical-device transport. Nothing has been sent to an E52.
- A Symbian^3 SDK. Only S60 3rd FP2 is on this host. Everything about the N8 in the prompt is **UNKNOWN** here and stays so until a Symbian^3 SDK and an N8 (or its EKA2L1 ROM) exist on this host.
- `VENDORID` other than 0 (no `--vid` in our post-linker), `UID` inside `START RESOURCE`, `.cia`/assembly sources, `EPOCSTACKSIZE`/`EPOCHEAPSIZE` (parsed, warned, not passed to the post-linker).
- The manifest `language` enum is exactly `cpp`; `LanguageBackend` in `symdev-core` is an empty marker trait. That is the seam.

## 2. The master prompt, section by section

Verdicts: **keep** (correct and still needed), **done** (symdev has it), **correct** (right idea, wrong premise or wrong first step), **unknown** (cannot be verified on this host yet), **later** (fine, not for this phase).

| § | Topic | Verdict | Note |
|---|---|---|---|
| 1 | Objective, `symdev new --language rust` … `symdev debug` | keep | `new/build/run` exist for C++; `test`/`debug` do not exist for any language |
| 2 | RustTarget ≠ SymbianVersion ≠ SDK ≠ Device ≠ RuntimeProfile | keep | Correct and important. One target `arm-symbian-e32`; SDK profile and device profile are data |
| 3 | Target spec fields | correct | Do not "investigate conceptually": derive the JSON from rustc's built-in `armv5te-none-eabi` and override the fields in §4 below; then run experiment 65. `armv5te-none-eabi` is indeed not Symbian, but it is the right *starting point* (same CPU, same EABI soft-float calling convention) |
| 4 | Stage 1 no_std | keep | The MVP; §5 below gives the exact contract, most of which is already observed |
| 5 | Stage 2 alloc | keep | `User::Alloc`/`Free`/`ReAlloc` from `euser.dso`, callable without a shim (static members, plain C ABI); alignment and failure semantics in §6 |
| 6 | symbian-core modules | keep | List is fine; order of implementation in §11 |
| 7 | Descriptors | keep | Layout is fully known from the SDK headers on this host (`e32des16.h`, `e32des8.h`); a `TPtrC16` is a 4-byte header + pointer, type in the top nibble. Verify against the headers, not from memory |
| 8 | Error model | keep | `e32err.h` has the codes; preserve the raw `TInt` |
| 9–10 | Cleanup stack, leaves, panic/leave boundary | keep | Leaves are C++ exceptions on EKA2/GCCE (`XLeaveException`); Rust frames must never be on the stack when one is thrown. Hence the C++ shim with `TRAP` around every leaving call, and `panic=abort` |
| 11 | symbian-runtime | correct | `E32Main` is C++-mangled (`_Z7E32Mainv`): the startup code in `eexe.lib` calls it; the observed link error is "undefined reference to `E32Main()`". A Rust `#[export_name = "_Z7E32Mainv"] extern "C" fn` is the whole entry point for Stage 1 |
| 12 | Async on Active Objects | later | `CActive` is a C++ class with virtual `RunL`/`DoCancel`; needs a shim class that forwards to a Rust callback. Design is sound, not for this phase |
| 13–14 | Threads, sync | unknown | ARMv5TE has no `LDREX`/`STREX`; what euser 9.3 offers for user-mode atomics is UNKNOWN on this host (experiment 68) |
| 15 | Filesystem | later | `RFs`/`RFile` are non-leaving for `Connect`/`Open`/`Read`/`Write`; the first shim-free API candidate after alloc |
| 16 | Networking | later | capability-gated; nothing to design until Stage 3 |
| 17–18 | Time, entropy | later | `User::NTickCount`, `TTime::HomeTime`, `Math::Random`; entropy quality UNKNOWN |
| 19–20 | symbian-sys + C shim | keep | The shim is compiled by the *existing* GCCE path and linked into the same EXE. It is a static library of `extern "C"` functions, not a DLL |
| 21 | Rust `fn main()` | keep | Generated `_Z7E32Mainv` wrapper calls it |
| 22 | UI | later | Avkon needs the app framework (`CAknApplication` …) — a C++ shim mountain. Console/InfoPrint first |
| 23 | SDK/API profiles | keep | `s60-3rd-fp2` = today's `SYMDEV_EPOCROOT`. `symbian3`: unknown, no SDK here |
| 24 | Capability system | done / keep | Manifest validates the six user-grantable capabilities; MMP `CAPABILITY` is cross-checked (exp 61). "Requesting ≠ authorised" is exactly the current rule |
| 25 | Signing separate from build | done | `Build → Package → Sign` is the current `SisPackage`; `SelfSigner` exists; a developer-cert pair is accepted via `[signing] cert/key`; `RemoteSigner` later |
| 26 | Dev mode / trust | later | Nothing device-side exists |
| 27–28 | std progression, capability-based cfg | keep | §11 order. `cfg(symbian_capability = …)` needs `--check-cfg` plumbing; fine |
| 29–30 | Target and device profiles | keep | Device profile = data file; must not leak into the Rust target |
| 31 | Emulator backend | done / keep | install + launch + pid + log exist; `stop`, `reset`, `collect_test_results`, `debug` do not |
| 32 | Physical device | unknown | No transport exists; nothing sent to an E52 yet |
| 33–34 | Test layers, result protocol | keep | Host tests = `cargo test` on the SDK crates' portable parts; emulator tests need a way to read results back out of EKA2L1 (a file on drive `E:` plus the `eka2l1.log`) — design in §9 |
| 35–37 | Legacy build ecosystem, MMP generation, Windows VM / Rosetta / remote environments | **correct: drop** | Out of date. No MMP is generated for Rust, no legacy tool is invoked, no Windows environment exists or is needed. `ExecutionEnvironment` in `symdev-core` is the only remnant and is `LocalEnv` |
| 38 | Toolchain manager, `symdev.lock` | keep (later) | Real gap; today it is env vars. A pinned Rust nightly is the first thing that needs pinning (§4) |
| 39 | `symdev doctor` | keep (later) | Does not exist |
| 40 | Workspace layout | correct | A separate Cargo workspace, because its crates build for a different target with `build-std`; §3 says where |
| 41–43 | Safety, memory model, size | keep | `panic=abort`, LTO, `opt-level=s` are target defaults in §4 |
| 44–45 | Crate tiers, golden corpus | keep | The golden corpus exists for every *tool* stage (SDK examples, Puzzles). A Rust golden corpus starts with experiment 65's E32 |
| 46 | MVP list | correct | Items 4–9 (link, E32, SIS, signing, hello, EKA2L1) exist for C++. The Rust delta is items 1–3 plus the entry point. Item 10 (E52) is Hardware M0 and is open for C++ too |
| 47 | Verify before implementing; mark UNKNOWN | keep | This is `CLAUDE.md`'s rule already: unobserved behaviour is an error, not a guess |
| 48–49 | Final architecture, N8 | keep / unknown | Fine as the north star; the N8 half is unverifiable here |

## 3. Architecture that fits this repository

```
symdev.toml  [language] name = "rust"
        │
   symdev build ── RustBuild (new BuildBackend, beside GcceBuild)
        │             cargo build --target arm-symbian-e32.json -Zbuild-std=core[,alloc]
        │             → target/arm-symbian-e32/release/lib<app>.a       (rustc: objects only)
        │             + C++ shim objects via the existing GCCE compile   (g++, recorded argv)
        │
        ├── existing link  (ld 2.29.1, recorded argv: eexe.lib, usrt2_2.lib, the DSOs)  → .elf
        ├── existing symdev-elf2e32                                                     → .exe (E32)
        ├── existing resources / icons / bitmaps                                        → .rsc/.mif/.mbm
        └── existing SisPackage → sign → symdev run (EKA2L1)
```

Rules that follow from it:

- **rustc never links.** It emits a static library (`--crate-type staticlib`) or objects; symdev owns the link line, which is recorded and byte-verified. This keeps the target JSON free of linker guesses and keeps one link path for both languages.
- **The SDK lives in this repository**, as a second Cargo workspace at `symbian-rs/` (the prompt's §40 layout), not inside `crates/`: its crates build for `arm-symbian-e32` with `build-std` on a pinned nightly, while `crates/` builds for the host on stable. Same repo so that the goldens, the experiment log, the specs and the toolchain notes stay in one place. `rust-toolchain.toml` inside `symbian-rs/` pins the nightly; the host workspace is untouched.
- **One Rust target.** SDK profile (`s60-3rd-fp2`, later `symbian3`) and device profile (`nokia-e52`, later `nokia-n8`) are TOML data read by symdev, never a second target triple. A second triple only if an ABI-level incompatibility is *observed*.
- **The C++ shim is a first-class build input**, compiled with the exact C++ argv `GcceBuild` already uses (so it sees the same headers, the same `gcce.h`, the same varargs repair), archived, and linked next to the Rust static library. No DLL for it.
- **Leaves never cross into Rust; panics never cross into C++.** Every shim function is `extern "C"`, `TRAP`s internally, and returns `TInt`. Rust builds with `panic=abort`; a panic calls `User::Panic` (or `User::Exit`) through the shim so the process dies the Symbian way with a category and a reason.

## 4. The target: `arm-symbian-e32`

Derive the JSON from the compiler, do not hand-type it:

```
rustc +nightly -Z unstable-options --print target-spec-json --target armv5te-none-eabi
```

then override. Fields and the reason for each; "observed" means the value is what the
recorded C++ pipeline already relies on, "hypothesis" means experiment 65 decides.

| Field | Value | Why |
|---|---|---|
| `llvm-target` | `armv5te-none-eabi` | ARMv5TE, EABI, no OS — observed: the SDK compiles `-march=armv5t`, and the makefile (exp 63) passes neither `-mthumb` nor interworking; Rust emits ARM code here |
| `arch` / `data-layout` / `target-pointer-width` / `target-endian` | from `armv5te-none-eabi` | same CPU, same layout |
| `abi` | `eabi` | soft-float calling convention: observed `-msoft-float` |
| `features` | `+soft-float,+strict-align,+v5te` | observed `-msoft-float`; ARMv5 faults on unaligned access |
| `os` | `symbian` | so that `cfg(target_os = "symbian")` exists; nothing in `core` keys on it |
| `env` | `e32` | names the executable model; documentation value |
| `panic-strategy` | `abort` | rule in §3; no unwinder is linked |
| `relocation-model` | `static` | observed: the SDK never uses `-fPIC`; ELF carries `R_ARM_ABS32`, the post-linker rewrites them into E32 relocations. PIC would need a GOT that E32 has no notion of — **hypothesis** that rustc `static` output post-links cleanly (experiment 65) |
| `linker-flavor` / `linker` | unset / n/a | rustc does not link (§3). If cargo insists on a linker for `bin` crates, point it at `/bin/false` and use `staticlib` only |
| `executables` | `false` for the SDK crates | we build static libraries |
| `has-thread-local` | `false` | Symbian TLS is `Dll::Tls()`/`UserSvr` calls, not an ELF TLS segment |
| `max-atomic-width` | `0` in Stage 1 | ARMv5TE has no `LDREX`; whether to expose atomics through euser helpers is experiment 68. `core` and `alloc` build without atomics (`Arc` is gated on `target_has_atomic`) |
| `atomic-cas` | `false` in Stage 1 | same |
| `emit-debug-gdb-scripts` | `false` | no gdb on the target |
| `eh-frame-header` | `false` | no unwinder |
| `c-enum-min-bits` | 32? | UNKNOWN — check `-fshort-enums` in the observed argv (it is not there, so GCC default for EABI: enums are `int` unless `-fshort-enums`) |
| `frame-pointer` | as `armv5te-none-eabi` | |
| default `opt-level` | `s` | prompt §43; observed SDK uses `-O2`, size matters more for us |

Hypotheses experiment 65 must also settle (each is a yes/no with bytes as evidence):

1. `.ARM.exidx` / `.ARM.extab`: rustc with `panic=abort` and `-C force-unwind-tables=no` emits none; if it emits some, does our post-linker accept them as the C++ path's do? (The C++ objects carry them and post-link fine.)
2. Symbol versioning on DSO imports (`--default-symver` on the link): a Rust object's undefined symbol `_ZN4User9InfoPrintERK7TDesC16` binds to `euser.dso`'s versioned export exactly as a C++ object's does — expected yes, it is the linker's job, not the compiler's.
3. Archive pull-in — **settled by 65a:** the reference to `_Z7E32Mainv` comes from `usrt2_2.lib` in the `-( -)` group *after* the object position, so a `.a` there is not pulled; `-u _Z7E32Mainv` before it is the fix (`--whole-archive` not needed).
4. No reference from Rust code to `__aeabi_*` helpers that `-lgcc` does not provide (division, memcpy/memset via `compiler_builtins` — build-std provides `compiler_builtins` with `mem` feature; check for duplicate `memcpy` against `-lgcc`/`usrt`).
5. Size of the E32 for hello: baseline for the size report of prompt §43.

Spike stand-in, before nightly is available: `armv5te-unknown-linux-gnueabi` is Tier 2 with a prebuilt `core`. Its `core` object code is CPU-identical (soft-float EABI, ARMv5TE); only `cfg(target_os)` lies, which a `#![no_std] #![no_main]` crate never consults. It is acceptable for experiment 65 *only*, and the experiment record must say so.

## 5. Stage 1 contract: `#![no_std]` hello through the existing pipeline

Known from the C++ pipeline (observed):

- Entry: `eexe.lib`'s `_E32Startup` → `CallThrdProcEntry` → `E32Main()`; the symbol is C++-mangled `_Z7E32Mainv`, returns `TInt`. `--entry _E32Startup -u _E32Startup` on the link line as today.
- Link inputs: exactly the recorded argv (`driver/link.rs`), with the Rust `.a` (and the shim `.a`) in the object position. `-lsupc++ -lgcc` stay; the six runtime DSOs stay (the C++ startup needs `drtaeabi`/`usrt2_2` symbols regardless of what the app is written in).
- Post-link: `symdev-elf2e32` with the same argv as a C++ EXE (`--uid1 0x1000007a`, uid2, uid3, capabilities, `--fpu=softvfp`, `--targettype=EXE`).
- SIS: unchanged. Run: unchanged.

Rust side, Stage 1 only:

```rust
#![no_std]
#![no_main]
#[panic_handler] fn panic(_: &core::panic::PanicInfo) -> ! { /* User::Exit(-1) via FFI; loop {} until then */ }
#[unsafe(export_name = "_Z7E32Mainv")]
pub extern "C" fn e32main() -> i32 { /* observable effect; return 0 */ }
```

Observable effect for the spike, without any shim: `User::InfoPrint(const TDesC16&)` — a
non-leaving euser export (`_ZN4User9InfoPrintERK7TDesC16`), shows a note on screen; then
`User::After(TTimeIntervalMicroSeconds32)` (`_ZN4User5AfterE25TTimeIntervalMicroSeconds32`,
argument is a 4-byte struct passed as `i32` under EABI) so the note is visible in the
screenshot loop. The `TPtrC16` argument is built from the header layout in
`epoc32/include/e32des16.h` (verify there; type nibble `EPtrC = 1`, length in the low 28
bits, then the pointer). Mangled names come from `nm` on the `.dso`, never from memory.

Pass criterion for Stage 1: `symdev build && symdev package && symdev run` on a
`language = "rust"` project shows the note in EKA2L1 (PID-bound screenshot), the process
exits 0, and the `.exe`'s E32 header round-trips through our own reader. That is
experiment 65 done in symdev proper; the throwaway spike (experiment 65a) may do the same
by hand first.

## 6. Stage 2: `alloc`

`GlobalAlloc` over euser (static member functions, plain C ABI, no shim):

| Rust | euser export (verify with `nm`) | Semantics to record |
|---|---|---|
| `alloc` | `User::Alloc(TInt)` | returns null on failure (non-leaving variant); alignment of the heap cell is UNKNOWN until read from `RHeap` docs/headers — **hypothesis** 4 or 8 bytes; over-aligned requests must be refused or padded |
| `dealloc` | `User::Free(TAny*)` | |
| `realloc` | `User::ReAlloc(TAny*, TInt, TInt)` | mode 0 |
| OOM handler | `User::Panic` or `User::Exit(KErrNoMemory)` | never a Rust panic that unwinds |

Thread heap: `User::Alloc` uses the *current thread's* heap; a second thread created through
`RThread::Create` gets its own unless told otherwise — this matters for Stage 4 threads and
is why `alloc` must not assume one global heap. Record in the memory-model note (prompt §42).

## 7. Stage 3: `symbian-sys`, the shim, `symbian-core`

- `symbian-sys`: raw `extern "C"` declarations, one module per DLL (`euser`, `efsrv`, …),
  each symbol's mangled name taken from `nm -D <dll>.dso` and recorded next to the
  declaration. Anything that leaves is *not* declared here; it goes through the shim.
- `shims/s60/` (C++, compiled by `GcceBuild`'s argv): `extern "C" TInt symrs_<name>(…)` wrappers
  that `TRAP` leaving calls, translate descriptors to `(ptr, len)` pairs, and never let a
  leave or a C++ exception out. One `.cpp` per subsystem; linked as a static archive.
- `symbian-core`: `SymbianError(TInt)` with `e32err.h` mapping; descriptors (`Des16Buf<N>`,
  `HeapDes16`, `&str ↔ UTF-16` without allocation when the caller provides the buffer);
  `Uid`; `Path` with drive semantics (`C:`, `E:`, `Z:` as data, never POSIX mounts).
- First real API after alloc: files (`RFs::Connect`, `RFile::Replace/Open/Read/Write/Close`
  are non-leaving and take descriptors by reference) — provable in EKA2L1 by reading the file
  back out of the emulator's `drives/e/`.

## 8. Runtime, threads, async (Stage 4+; design only)

- `symbian-runtime`: generated `_Z7E32Mainv` that sets up the panic hook, calls the user's
  `fn main() -> Result<(), SymbianError>` (or `()`), maps the result to `E32Main`'s `TInt`.
  A `CActiveScheduler` is installed only when the app asks for async.
- Threads: `RThread::Create` with an explicit stack size and heap choice; no POSIX semantics.
  Atomics: experiment 68 first.
- Async: a shim `CActive` subclass whose `RunL` calls a Rust `extern "C"` waker; the Rust
  executor is single-threaded and lives on the active scheduler. Not before Stage 3 is solid.

## 9. symdev integration

| Piece | Change |
|---|---|
| Manifest | `language.name = "rust"`; `[rust] target = "arm-symbian-e32"` optional (only one value exists); `[symbian] sdk = "s60-3rd-fp2"` optional (only one value exists) |
| Scaffold | `symdev new hello --language rust`: `symdev.toml`, `Cargo.toml` (`staticlib`), `src/main.rs`, `rust-toolchain.toml`, `.cargo/config.toml` pointing at the target JSON symdev ships; **no** `bld.inf`, no `.mmp` |
| Build backend | `RustBuild: BuildBackend` beside `GcceBuild`: runs cargo (nightly, `build-std`), compiles the shim with `GcceBuild`'s compile argv, then reuses `link_args_for` / `elf2e32_args_for` / resources / icons unchanged |
| Toolchain | `SYMDEV_CARGO` (or `rustup run <pinned>`); the nightly version pinned in `symbian-rs/rust-toolchain.toml` and echoed in `symdev doctor` when that exists |
| `symdev test --emulator` | Stage 3+: the test binary writes a JSON result file (prompt §34 shape) to `E:\symdev\results\<uid3>.json`; symdev reads it from `~/.local/share/EKA2L1/data/drives/e/` after the process exits. Host tests are plain `cargo test` on the crates' portable parts |
| Size report | `symdev size`: E32 header fields (code/data/bss sizes) our reader already parses |

## 10. UNKNOWN list (each becomes an experiment before it becomes code)

1. Heap cell alignment and the over-alignment story for `User::Alloc` (§6).
2. User-mode atomics on EKA2 9.3 / ARMv5TE: what euser exports (`User::LockedInc/Dec`, anything `__e32_atomic_*`), and whether `RFastLock` is an acceptable fallback for `Mutex`.
3. Whether `--check-cfg` accepts `symbian_capability = …` values from a custom target without warnings.
4. Entropy: what `Math::Random` is seeded from; whether the crypto DLLs expose a real RNG.
5. Everything Symbian^3 / N8: no SDK, no ROM, no device on this host.
6. Hardware M0: stock E52 install of *any* symdev output.
7. Whether `compiler_builtins`' `mem` symbols collide with `-lgcc`/`usrt2_2` at link (§4 item 4).
8. `c-enum-min-bits` / any other ABI field where the SDK's GCC 3.4.3 default might differ from EABI's — settle by compiling a tiny C++ probe with the observed argv and reading its DWARF.

## 11. Order of work, with pass criteria

| # | Experiment / slice | Passes when |
|---|---|---|
| 65a | Spike: no_std hello by hand (stand-in target, prebuilt `core`) → recorded link → native elf2e32 → SIS → EKA2L1 | **passed 2026-09-20**: notifier log line identical to the C++ control; EKA2L1 halts on `InfoPrint` rendering for both (emulator issue, filed) |
| 65 | The same through `symdev build/package/run` with `language = "rust"`, custom target JSON, pinned nightly + `build-std` | same, from a scaffolded project; E32 size recorded |
| 66 | `alloc` over euser; a `Vec` and a `String` in hello | heap used and freed; alignment hypothesis settled |
| 67 | First shim + `symbian-core` file API; hello writes and reads a file | file content visible in the emulator's drive directory |
| 68 | Atomics / locks survey on 9.3 | table of what euser offers, with `nm` evidence |
| 69 | `symdev test --emulator` result protocol | a failing test is reported as failing from the emulator run |
| — | Threads, async, UI, networking, N8 | after the above, each with its own spec |

Each experiment gets a backlog entry with the bytes and the argv; the first passing E32 for
each stage joins the golden corpus (prompt §45).

## 12. Non-goals for this phase

Full `std`; Avkon UI; networking/TLS; any Symbian^3 work; any physical-device transport;
`symdev doctor`/toolchain manager (tracked, not here); replacing EPOCROOT (product C —
never for this phone).
