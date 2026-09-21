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
| 13–14 | Threads, sync | settled by exp 72 | ARMv5TE has no `LDREX`/`STREX` and euser 9.3 exports only `User::LockedInc/Dec` and `SafeInc/Dec` (`TInt` ±1, no CAS). Nothing on the link line defines a `__atomic_*`/`__sync_*` libcall, so every atomic is a link error today; a `__atomic_*` shim over one process-wide `RFastLock` makes Rust's `AtomicU32` link and behave correctly. Threads are `RThread::Create` + `Logon`. [eka2-concurrency.md](../../research/eka2-concurrency.md) |
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
| `features` | `+soft-float,+strict-align` | observed `-msoft-float`; ARMv5 faults on unaligned access. **Exp 65:** kept as derived; `+v5te` is implied by `llvm-target` |
| `os` | `symbian` | so that `cfg(target_os = "symbian")` exists; nothing in `core` keys on it |
| `env` | `e32` | names the executable model; documentation value |
| `panic-strategy` | `abort` | rule in §3; no unwinder is linked |
| `relocation-model` | `static` | observed: the SDK never uses `-fPIC`; ELF carries `R_ARM_ABS32`, the post-linker rewrites them into E32 relocations. PIC would need a GOT that E32 has no notion of — **hypothesis** that rustc `static` output post-links cleanly (experiment 65) |
| `linker-flavor` / `linker` | as derived (`gnu-lld` / `rust-lld`) | rustc does not link (§3): never run for a `staticlib`. **Exp 65:** `executables: false` makes cargo refuse `bin` targets, so a `src/main.rs` library needs `autobins = false`; no `/bin/false` trick needed |
| *(the linker symdev runs)* | GNU ld **2.29.1**, not lld | **Exp 82:** lld rejects 428 of the SDK's import libraries as malformed (`.dynstr` not NUL-terminated), refuses the absolute relocations the E32 model is built on, and does not define the RVCT section symbols `eexe.lib` references. The fields above name what *rustc* would use and never does |
| `executables` | `false` for the SDK crates | we build static libraries |
| `has-thread-local` | **`false`**, revisited and kept (exp 88) | The field governs `#[thread_local]` statics, which need an ELF `PT_TLS` segment; `symdev-elf2e32` keeps only `PT_LOAD` program headers (`elf/image.rs:63`), so a `PT_TLS` would be silently dropped. Symbian's own TLS is a kernel call — and **not** `Dll::Tls`, which does not exist in this SDK: `e32std.h` has no `Dll` class and euser exports none, so it is `UserSvr::DllTls`/`DllSetTls`/`DllFreeTls` (`e32svr.h` lines 39-43). `symbian_std::thread_local!` uses no compiler support at all, so `true` would buy a feature the post-linker would have to reject |
| `max-atomic-width` | **`32`** (exp 80; `0` before it) | ARMv5TE has no `LDREX`. **Settled, exp 72:** raise to `32` *only* in the change that also puts a `__atomic_*` shim on the link line — on its own the raise turns a compile error into seven undefined `__atomic_*` references. **Done in exp 80**, together with `crates/symbian-libcalls`. Not 64: at 32 no `_8` libcall is ever emitted, which is what keeps `AtomicU64` out of dependency code that would silently get a lock |
| `atomic-cas` | **`true`** (exp 80; `false` before it) | same; `true` together with the archive. euser exports no CAS at all, so the lock is the only compare-exchange on 9.3 |
| `emit-debug-gdb-scripts` | `false` | no gdb on the target |
| `eh-frame-header` | `false` | no unwinder |
| `c-enum-min-bits` | `32` | **settled, exp 65:** the derived JSON says 8 (AAPCS short enums) but a probe compiled with the observed GCCE argv has `sizeof(enum) == 4`; override to 32 |
| `frame-pointer` | as `armv5te-none-eabi` | |
| default `opt-level` | `s` | prompt §43; observed SDK uses `-O2`, size matters more for us. **Exp 65:** a `[profile.release]` setting (`opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`), not a target field |

Hypotheses experiment 65 must also settle (each is a yes/no with bytes as evidence):

1. `.ARM.exidx` / `.ARM.extab` — **settled by 65:** rustc emits `.ARM.exidx` (`CANTUNWIND` entries) even with `panic=abort` and `default-uwtable=false`; the linked ELF carries `.ARM.exidx`/`.ARM.extab` as a C++ EXE does and the native post-linker accepts it.
2. Symbol versioning on DSO imports (`--default-symver` on the link): a Rust object's undefined symbol `_ZN4User9InfoPrintERK7TDesC16` binds to `euser.dso`'s versioned export exactly as a C++ object's does — expected yes, it is the linker's job, not the compiler's.
3. Archive pull-in — **settled by 65a:** the reference to `_Z7E32Mainv` comes from `usrt2_2.lib` in the `-( -)` group *after* the object position, so a `.a` there is not pulled; `-u _Z7E32Mainv` before it is the fix (`--whole-archive` not needed).
4. `compiler_builtins` vs the link line — **settled by 68** (see §10 item 7); partly settled by 65: build-std's `compiler_builtins` member defines `__aeabi_*`, libm and `mem*` as *weak* symbols and is not pulled for hello; `memcpy`/`memset`/`memmove` are strong exports of `euser.dso`, `__aeabi_mem*` of `drtaeabi.dso`, nothing exports `memcmp`. A program that references one pulls the weak member first (the archive precedes the DSOs) — observe in 66.
5. Size of the E32 for hello: **752 bytes** (exp 65; 65a's stand-in build was 755).
6. **Exp 65:** cargo on this nightly needs `-Zjson-target-spec` for a `.json` target; `-Zbuild-std=core,alloc` also builds `compiler_builtins` and locks its crates.io dependencies in `symbian-rs/Cargo.lock`. Pinned: `nightly-2026-09-19` in `symbian-rs/rust-toolchain.toml`.

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

## 6a. The API principle (owner's direction, 2026-09-20)

**An application developer should almost never touch a Symbian-specific API.** Where Rust
already has a name and a shape for something, the SDK uses that name and that shape; where
Symbian genuinely differs, the SDK says so plainly instead of hiding it behind a pretend
abstraction.

Concretely:

| The developer writes | Not |
|---|---|
| `&str`, `String`, `format!`, `write!` | descriptors — `TDesC16`, `Buf16`, `HBuf16` are a boundary detail |
| `fs::File::create(path)?.write_all(b"…")` | `RFs`, `RFile`, `TDes8` |
| an error with `kind()` like `std::io::ErrorKind`, `?` across the app | a bare `TInt` |
| `Duration`, `Instant`, `SystemTime` | `TTimeIntervalMicroSeconds32`, `TTime` |
| `Vec`, `String`, `BTreeMap` | Symbian's array and buffer classes |
| `thread::spawn`, `Mutex`, `mpsc` (after step 72's shim) | `RThread`, `RFastLock`, `RMsgQueue` |

The descriptor-typed and handle-typed entry points stay, in a clearly lower-level module
(`raw`/`des`/`sys`), because the shim and the crates below need them and because an
application that must reach further should be able to. They are the escape hatch, not the
road.

**Size does not outrank ergonomics here.** `core::fmt` costs about 2.4 kB (experiment 77) on
a device with ~128 MB of user RAM; that is the feature working, not waste. The 10 kB and
107 kB figures that experiment 77 fixed were a *duplicate* of routines already in ROM, which
is a different thing. A zero-cost path (euser's own `Append`/`AppendNum` through the shim)
stays available and documented for a program that deliberately wants no formatter.

**What must not be hidden**, because pretending would be a lie the first crash exposes:

- **Capabilities and platform security.** There is no std analogue; an app declares them in
  `symdev.toml` and the SIS, the certificate and the device each get a say.
- **Drives and data caging.** `Path` can carry `C:`/`E:`/`Z:`, but `\private\<uid>` and the
  caging rules are real and different from a POSIX mount.
- **The event loop.** An Avkon application is not `fn main()` running to completion; the
  framework owns an active scheduler and calls into the app (`avkon-rust-spec.md`).
- **Leaves.** Hidden by the shim — correctly, because a leave reaching Rust is undefined
  behaviour, and a `Result` is the honest translation.
- **No `std::net` semantics for free.** Sockets work (step 74) and they wear `std`'s names,
  but choosing an access point — `RSocket::Open`'s `RConnection` overload — is a Symbian
  concept with no std equivalent, and `symbian_std::net` says so in its own documentation
  rather than picking one silently. Written out in
  [net-access-points.md](../../research/net-access-points.md).

The end state this points at is a real `symbian-std` (§27 of the master prompt, step 79
below): a facade mirroring `std`'s module tree so that porting a crate is changing a `use`,
and eventually a genuine `std` for `target_os = "symbian"`. The layering already built —
`symbian-sys` → shim → `symbian-core` → `symbian-runtime` — is exactly what such a std sits
on, so nothing here is thrown away.

## 7. Stage 3: `symbian-sys`, the shim, `symbian-core`

### The rule: when a call needs a C++ shim

Settled by **experiment 78**, written out in full in `symbian-rs/shims/common/symrs_shim.h`
(that header is the authority; this is the summary). A Symbian call needs a wrapper in the
shim when **any** of these is true, and otherwise `symbian-sys` declares it and Rust calls
it directly:

1. **It can leave.** The SDK's trailing `L`/`LC`/`LD` is a hint, not the authority:
   confirm against the declaration in the header, and treat a header that says nothing
   either way as leaving. This one is absolute — `__LEAVE_EQUALS_THROW__` makes a leave a
   real C++ exception, rustc emits one `cantunwind` `.ARM.exidx` entry over the whole Rust
   text, and an exception that reaches a Rust frame ends the process with **no diagnostic
   at all** (experiment 76, reproduced in 78). The `TRAP` goes around the leaving SDK call
   *inside* the shim, never around a call into Rust.
2. **Its signature is not a C signature**: a class returned by value with a non-trivial
   copy constructor (sret), a `TRefByValue` varargs function such as `TDes16::Format`, a
   virtual call.
3. **It is a virtual member, or a member of a class with multiple or virtual
   inheritance**, where `this` may need adjusting. Not observed in this repository, so not
   attempted.

**Being a non-static member function is not a reason.** That was the open question of
experiment 69 and is now *observed*: a probe compiled with the recorded GCCE argv and
disassembled shows `d->Append(*s)` as a bare `bl _ZN6TDes166AppendERK7TDesC16` with no
register shuffle, and `d->AppendNum((TInt64)n)` as `movs r2,r1; asrs r3,r1,#31` — `this`
is argument 0 under the ordinary AAPCS assignment and everything else follows it. The one
exception, also observed: a class returned by value is sret, with the return slot as
argument 0 and `this` displaced to argument 1 (rule 2). So `TDes16::Copy`/`Append`/
`AppendNum` and every `RFs` member are declared in `symbian-sys` and called directly.

**A panic is not a leave and no `TRAP` catches it.** `e32panic.h` documents
`ETDes16Overflow = 11` (category USER) for every copying, appending and formatting
descriptor member. A Rust wrapper that could provoke one must make it unreachable by
checking first; `symbian-core` checks the room from the length word and `N` it already
holds, with no call, and returns `KErrOverflow`.

### The pieces

- `symbian-sys`: raw `extern "C"` declarations, one module per DLL (`euser`, `efsrv`,
  `des16`, …) plus `shim` for the shim's own entry points; each symbol's mangled name
  taken from `nm -D <dll>.dso` and recorded next to the declaration.
- `shims/common/` (C++, compiled by `GcceBuild`'s argv): `extern "C" TInt symrs_<name>(…)`
  wrappers that `TRAP` leaving calls and never let a leave or a C++ exception out. One
  `.cpp` per subsystem. `shims/s60/` is reserved for the Avkon subclasses of step 75,
  which need their own libraries and must not be forced onto a console application.
- Every shim symbol is `__attribute__((visibility("hidden")))`: the shim is an
  implementation detail of the SDK, not an export of the application.
- `symbian-core`: `SymbianError(TInt)` with `e32err.h` mapping; the descriptor family;
  `FileServer` (`RFs`); `shim::leave_if_error`, the run-time check that the trap harness
  is in place.
- **The API an application sees takes Rust types.** `FileServer::make_dir_all(&str)`, not
  `(&impl DesC16)`; a descriptor is something an application never has to name. The
  descriptor-typed entry points (`Buf16::append_des`, `copy_des`) exist for text that
  arrives *from* Symbian and are documented as the lower-level path. Errors keep the raw
  `TInt` underneath an `ErrorKind`, so `?` works and the exact code survives.
- First real API after alloc: files (`RFs::Connect`, `RFile::Replace/Open/Read/Write/Close`
  are non-leaving) — `f32file.h` declares no leaving member on `RFs` at all, so step 71 is
  entirely shim-free.

### Build integration

`RustBuild` compiles `symbian-rs/shims/common/*.cpp` with **`GcceBuild::compile_args`** —
the identical C++ argv a project source gets, so the shim sees `gcce.h`, the GCC-12
varargs repair and every define — into `build/shims/*.o`, archives them as
`build/shims/libsymrs.a` and puts that immediately after the Rust archive. The
application lists, names and configures nothing: the shim is part of the SDK.

It has to be an **archive**, not loose objects, and the reason is measured (experiment
78): objects are linked whole, so an unused wrapper's reference to its DLL still makes ld
record a `DT_NEEDED`, and `--gc-sections` cannot undo it because as-needed is decided
during symbol resolution and collection happens afterwards. `hello` grew 3 187 → 3 219
bytes and loaded `bafl.dll` to call nothing. From an archive the member is never pulled.
The DSOs the SDK itself may import (`efsrv.dso`, `bafl.dso`) go on the line under
`--as-needed`/`--no-as-needed`. `ar` is derived from `SYMDEV_LD` (`…-ld` → `…-ar`), with
`SYMDEV_AR` as an optional override: no new required environment variable.

## 8. Runtime, threads, async (Stage 4+; design only)

- `symbian-runtime`: generated `_Z7E32Mainv` that sets up the panic hook, calls the user's
  `fn main() -> Result<(), SymbianError>` (or `()`), maps the result to `E32Main`'s `TInt`.
  A `CActiveScheduler` is installed only when the app asks for async.
- Threads and atomics: **shipped, experiment 80.** `crates/symbian-libcalls` defines the 31
  `__atomic_*`/`__sync_synchronize` entry points LLVM emits, over one process-wide
  `RFastLock` created on first use by `User::LockedInc`; it is linked as its own archive, so
  a program that performs no atomic operation carries none of it. `symbian_std::sync::Mutex`
  is a one-token `RSemaphore` — **not** the `RFastLock` experiment 72 proposed, because
  `RSemaphore` is the only primitive in 9.3 with a timed wait and a mutex cannot have `lock`
  and `try_lock` on two different kernel objects. `Once` is the three-state word with a
  yielding spin. `symbian_std::thread::spawn` uses `RThread::Create`'s **own-heap** overload
  and switches the new thread onto the creator's heap with `User::SwitchAllocator`; the
  shared-allocator overload destroys the creator's heap when the worker exits.
- Async: a shim `CActive` subclass whose `RunL` calls a Rust `extern "C"` waker; the Rust
  executor is single-threaded and lives on the active scheduler. Not before Stage 3 is solid.

## 9. symdev integration

| Piece | Change |
|---|---|
| Manifest | `language.name = "rust"`; `[rust] target = "arm-symbian-e32"` optional (only one value exists); `[symbian] sdk = "s60-3rd-fp2"` optional (only one value exists) |
| Scaffold | `symdev new hello --language rust`: `symdev.toml`, `Cargo.toml` (`staticlib`, `autobins = false`, the SDK crates by absolute path), `src/main.rs`, `rust-toolchain.toml`, `.cargo/config.toml` pointing at the target JSON symdev ships; **no** `bld.inf`, no `.mmp`. **Exp 65:** the SDK is found through `SYMDEV_RUST_SDK` or the checkout's `symbian-rs/` (`RustSdk`); the two SDK files are `include_str!` copies. Stopgap until an installed layout exists. **Exp 81:** the written `src/main.rs` is `#![no_std]`, `#[symbian_std::main]` and a `fn main() -> Result<()>` — no `#![no_main]` (rustc never looks for a `main` in a `staticlib`) and no entry macro at the bottom of the file; the dependencies are `symbian-std` (the road: the attribute, `io`, `fs`, the prelude, and the runtime underneath it) and `symbian-core` (the descriptors the escape hatch needs) |
| Build backend | `RustBuild: BuildBackend` beside `GcceBuild`: runs cargo (nightly, `build-std`), compiles the shim with `GcceBuild`'s compile argv, then reuses `link_args_for` / `elf2e32_args_for` / resources / icons unchanged. **Exp 65:** `cargo build --release --target <json> -Zbuild-std=core,alloc -Zjson-target-spec --target-dir build/cargo`, then `GcceBuild::link_args` + `-u _Z7E32Mainv`, then `GcceBuild::elf2e32_args`; no shim, resources or icon yet |
| Toolchain | **Exp 65:** `SYMDEV_CARGO`, else `cargo` on `PATH` (the rustup proxy), with the project's `rust-toolchain.toml` (a copy of `symbian-rs/`'s) choosing the nightly; `RUSTUP_TOOLCHAIN` is removed from cargo's environment so a proxied symdev cannot override it. Echo in `symdev doctor` when that exists |
| `symdev test --emulator` | Stage 3+: the test binary writes a JSON result file (prompt §34 shape) to `E:\symdev\results\<uid3>.json`; symdev reads it from `~/.local/share/EKA2L1/data/drives/e/` after the process exits. Host tests are plain `cargo test` on the crates' portable parts |
| Size report | `symdev size`: E32 header fields (code/data/bss sizes) our reader already parses |

## 10. UNKNOWN list (each becomes an experiment before it becomes code)

1. ~~Heap cell alignment and the over-alignment story for `User::Alloc` (§6)~~ — settled by experiment 68: measured 8 bytes on this ROM's heap (32 cells, sizes 1…257, cell sizes always a multiple of 8, `User::AllocLen` always `4 (mod 8)`), and `RHeap` is not even declared in this SDK's headers, so the allocator trusts 8 and pads anything larger by hand.
2. ~~User-mode atomics on EKA2 9.3 / ARMv5TE~~ — settled by experiment 72: euser exports only `User::LockedInc/Dec` and `SafeInc/Dec` (`TInt` ±1, old value returned, `Safe*` only when > 0), there is no `e32atomics.h` and no CAS; `RFastLock` is process-local and non-recursive and is the right backing for `Mutex`. [eka2-concurrency.md](../../research/eka2-concurrency.md)
3. Whether `--check-cfg` accepts `symbian_capability = …` values from a custom target without warnings.
4. Entropy: what `Math::Random` is seeded from; whether the crypto DLLs expose a real RNG.
5. Everything Symbian^3 / N8: no SDK, no ROM, no device on this host.
6. Hardware M0: stock E52 install of *any* symdev output.
7. ~~Whether `compiler_builtins`' `mem` symbols collide with `-lgcc`/`usrt2_2` at link (§4 item 4)~~ — settled by experiment 68: no collision. `memcpy`/`memset`/`__aeabi_mem*` all resolve to `compiler_builtins`, because the Rust archive precedes the DSOs; euser's and drtaeabi's copies are simply unused. The cost is that `compiler_builtins` is one codegen unit, so one reference pulls the whole crate (104 560-byte E32) — `--gc-sections` on the Rust link line brings it back to 11 499.
8. ~~`c-enum-min-bits`~~ — settled by experiment 65: a probe compiled with the observed GCCE argv gives `sizeof(enum) == 4`, so the target sets 32.

Numbers 66 and 67 went to the `SECUREID` override and the no-edit third-party build; the Rust
track resumes at 68.

## 11. Order of work, with pass criteria

The owner's direction (2026-09-20): **the SDK is driven by example applications, one per
subsystem**, each of which is the verification harness for the thing it exercises. Not one
flagship app. Every example lives in `symbian-rs/examples/<name>`, builds through
`symdev build` like any project, and reports in a way a machine can read.

### How an example reports

Three channels, in order of cost:

1. **`User::InfoPrint`** — already proven: EKA2L1 logs `[Service.Notifier]: Trying to display: …`, so a `grep` of `build/eka2l1.log` is a pass/fail signal with no infrastructure. Good for the first milestones.
2. **A result file** on drive `E:` — the example writes `E:\symdev\results\<uid3>.json` (the shape in the master prompt's §34) and symdev reads it from `~/.local/share/EKA2L1/data/drives/e/` after the process exits. Needs files (step 71). This is what `symdev test --emulator` will use.
3. **A PID-bound screenshot** — for anything visual. The loop is in the `eka2l1-host` skill.

### Steps

| # | Slice | Passes when |
|---|---|---|
| 68 | `alloc` over euser (`User::Alloc`/`Free`/`ReAlloc`); `examples/alloc` | **done** (2026-09-20): a `Vec` and a `String` live and die; alignment measured at 8; `mem*` resolve to `compiler_builtins`; `--gc-sections` added to the Rust link line |
| 69 | `symbian-core`: `SymbianError` from `e32err.h`; the descriptor family (`Des16` borrowed view, `Buf16<N>` on the stack, `HBuf16` on the heap), `&str` ↔ UTF-16 with no heap round-trip, `core::fmt::Write` | **done** (2026-09-20): `examples/hello` rewritten with `write!` into a `Buf16`, no `unsafe` anywhere in it; the descriptor type nibbles observed on the device's euser |
| 70 | The C++ shim: a static library built by the existing GCCE argv, one `extern "C"` `TRAP` wrapper per leaving call, and the rule for which calls need one | **done** (2026-09-20, experiment 78): `User::LeaveIfError(-12)` through the shim returns `Err(KErrNotFound)` and the process prints four more fields after it; `RFs::MkDirAll` and euser's `TDes16` members are non-static members called directly, because the member ABI was observed rather than guessed. `hello` is still 3 187 bytes and still has six `NEEDED`. `examples/shim`, `corpus/78-shim/` |
| 71 | Files and the `symbian-std` facade (`fs`, `io`, `prelude`); the result protocol and `symdev test --emulator` | **done** (2026-09-20, experiment 79): `examples/files` reports 16 passing cases from inside the emulator and `symdev test` exits 0; two deliberate failures are reported as `2 failed, 16 passed` with exit 1, and a missing report is a failure too. The facade landed here rather than as a late step, per §6a. |
| 72 | **done** (2026-09-20, experiment 80): the target now says `max-atomic-width: 32` / `atomic-cas: true`, so `AtomicU32`, `AtomicPtr`, `fence` and — for the first time — `alloc::sync`'s `Arc` exist. The 31 `__atomic_*`/`__sync_synchronize` entry points they lower to are defined in `crates/symbian-libcalls` over one process-wide `RFastLock`, created on first use by `User::LockedInc`, and linked as their own archive so a program that uses no atomic carries none of it. `symbian_std::sync` has `Mutex` (a one-token `RSemaphore`, because that is the only primitive with a timed wait, so `try_lock` is possible), `MutexGuard`, `Once` and `Arc`; `symbian_std::thread` has `spawn`, `JoinHandle::join`, `sleep` and `yield_now`. `examples/atomics` reports 23 passing cases: `fetch_add` from two threads is 4000 of 4000 and a load-then-store beside it is about half that. The survey's unexplained access violation turned out to be the shared-allocator `RThread::Create` overload destroying the creator's heap; a worker now gets its own heap and switches. Sizes: `hello` 3 187, `hello-raw` 752 and `shim` 4 475 unchanged; `alloc` 4 320 → 4 474 and `files` 10 423 → 10 552, both the heap's new lock and neither the atomics; `atomics` 11 582. `symbian-rs/corpus/80-atomics/` | [eka2-concurrency.md](../../research/eka2-concurrency.md) is the survey it implements, annotated **[80]** where the implementation contradicted it |
| 73 | **Async**: a `CActive` subclass in the shim whose `RunL` wakes a Rust waker, a single-threaded executor on `CActiveScheduler`; `examples/async` awaits an `RTimer` | two timers awaited concurrently finish in the right order, reported through the result file, with no extra thread |
| 74 | **Networking**: `RSocketServ`, `RHostResolver`, `RSocket` in `std`'s shape — **blocking, and it needed no async bridge**; `examples/net` | **done** (2026-09-21, experiment 84): `symbian_std::net` with `TcpStream`, `TcpListener`, `UdpSocket`, `ToSocketAddrs` over `core::net`'s address types; `examples/net` reports **22 passing** cases through `symdev test --emulator`, resolving `localhost`, doing TCP round trips in both directions against a Python peer on this host's loopback, and binding UDP. No shim (nothing in `es_sock.h`/`in_sock.h` leaves) and **no executor**: every `TRequestStatus` is waited for with `User::WaitForRequest`, which is the blocking form `std::net` means. `corpus/84-net/`, 13 183 bytes |
| 75 | **UI**: the Avkon app framework — `CAknApplication`/`CAknDocument`/`CAknAppUi`/`CCoeControl` are C++ classes with virtual methods, so the shim must *define the subclasses* and forward each virtual to a Rust function pointer; `examples/ui` draws and handles a key | a PID-bound screenshot shows the drawn view and a key press changes it |
| 76 | **TLS and time**: `Dll::Tls`/`UserSvr` behind `thread_local!`; `Instant`, `SystemTime`, with monotonic-versus-wall-clock stated honestly | **time: done** (2026-09-20, experiment 85): `symbian_std::time` gives `Duration`, `Instant`, `SystemTime`, `UNIX_EPOCH` and `SystemTimeError`, and `examples/time` reports 29 passing cases from inside the emulator — including the criterion, a `User::SetUTCTime` hour that moves `SystemTime` and leaves the `Instant` at 0. `Instant` is `User::TickCount` + `UserHal::TickPeriod` and **not** `NTickCount`, whose period no call on this link line will state; `SystemTime` is `UniversalTime` and not `HomeTime`, at an epoch measured from euser's own calendar because the computed one is 12 days wrong. **TLS: done** (2026-09-21, experiment 88): `symbian_std::thread_local!`, `LocalKey`, `with`/`try_with` and `const { … }` initialisers, and `examples/tls` reports **45 passed**. The premise was wrong — **there is no `Dll` class in this SDK** and the whole surface is five `@internalAll` `UserSvr` statics — and so was the fear: an EXE gets **many** slots, keyed by a `TInt` it chooses, and they are **per thread** (64 held at once; a worker reads null where its creator stored a value). The SDK still takes **one** slot and puts its own table behind it, because the kernel will not enumerate a thread's slots and `Drop` needs that list. **One access is 100 ns, of which the kernel call is 53; `AtomicU32::fetch_add` beside it is 158** — a thread-local is the cheap per-thread state on this device. Destructors run at the end of every `spawn`ed thread; the main thread calls `thread::drop_thread_locals()` itself, which is step 77's `lang_start`'s job. |
| 77 | **`std` for `target_os = "symbian"`** — the owner's decision, 2026-09-20. Fork `rust-src`, add `library/std/src/sys/pal/symbian` over the crates below, build it with `-Zbuild-std=std`. The facade's `fs`/`io`/`sync`/`thread` are re-hosted there rather than rewritten, and `#[symbian_std::main]` becomes std's `lang_start`. | an application drops `#![no_std]`, writes `use std::fs::File`, and builds; then a crate from crates.io that needs only `std` compiles unchanged |

**Step 77 is where `#![no_std]` disappears, and it is gated on real work, not on a rename.
Every gate is now open.** `std` needs, pervasively: `alloc` (68, done), atomics and
`Mutex`/`Once` (72, done), **TLS** (76, **done** — it was called the largest risk in this plan and
it was not one: the platform gives many per-thread slots, not the one an EXE was feared to have,
and an access is cheaper than an atomic), `time` (76, **done**), `fs`/`io` (71, done,
and already in `std`'s shape so it re-hosts rather than gets rewritten), and `net` (74). `process`
and `env` will be mostly `Unsupported`, which `std` supports as a platform answer; where `println!`
goes on a phone with no console is an open question for that step. The prize is not the missing
attribute — it is that a crate which needs only `std` starts compiling. A cheap precursor worth
five minutes first: in a `no_std` crate the name `std` is free, so `use symbian_std as std;` may
already make `std::fs::File` resolve; test it before assuming the port is the only route.

Dependencies that fix the order: 73 before a real 75 (the app framework runs an active
scheduler). **73 before 74 was wrong, and step 74 disproved it**: every socket operation is
indeed `TRequestStatus`-based, but `std::net` is blocking and the blocking form of a Symbian
asynchronous call is the request followed by `User::WaitForRequest` — no `CActive`, no
scheduler, no executor. Step 73 now sits *beside* `symbian_std::net` on the same `symbian-sys`
declarations rather than underneath it. **70 before 75**, where almost every Avkon entry point
leaves; esock turned out to declare no leaving member at all, so 74 needed no shim either.

Each step gets a backlog entry with the argv and the bytes; each passing example joins
`symbian-rs/corpus/`.

## 12. Non-goals for this phase

Full `std`; Avkon UI; TLS in the *transport* sense — `securesocket.dso`, which step 74's plain
sockets did not touch, and which is a different thing from the thread-local storage step 76
shipped; any Symbian^3 work; any physical-device transport;
`symdev doctor`/toolchain manager (tracked, not here); replacing EPOCROOT (product C —
never for this phone).
