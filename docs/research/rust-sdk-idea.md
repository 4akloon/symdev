# “Full-fledged Rust SDK”: three products, one phrase

Investigation only. No implementation. Does **not** reopen Wave 0 locks.

Cites: [2026-09-16-symdev-m0-north-star-design.md](../superpowers/specs/2026-09-16-symdev-m0-north-star-design.md); [legacy-sdk-leftover.md](legacy-sdk-leftover.md); [ported-tools-crates.md](ported-tools-crates.md); [pipeline-and-tools.md](pipeline-and-tools.md); [licensing.md](licensing.md); [experiment-backlog.md](experiment-backlog.md) experiments 5–6, 9–11; `GcceBuild` (`crates/symdev-build/src/driver.rs`); `LanguageBackend` (`crates/symdev-core/src/traits.rs`); hello template (`crates/symdev-cli/templates/hello.cpp`). Master Development Prompt (2026-09-16 Telegram; **not in-tree**; north-star §1: not an authority).

Locked today (not reopened): Linux host; Wave 0 **C++** `arm-none-symbianelf-g++` → GNU ld **2.29.1** → `elf2e32` → native SIS; EPOCROOT headers / `.dso` stay; self-sign; never claim E52 until stock install+launch; **T5 / M3 / M5** parked.

## Phrase hunt

The string **“Rust SDK”** does **not** appear in `docs/`, specs, or plans. There is no `docs/superpowers/brainstorms/` tree. Closest in-tree wording is the north-star (status: “approved design (brainstorming)”):

> “symdev is a Rust CLI that orchestrates a modern developer workflow for **legacy Symbian hardware**. It is not a Symbian compiler and not a Rust-to-Symbian language product.”

Non-goals in the same spec include: “Java ME, UIQ, S40, **Rust as an application language**, hot reload.” Also listed as a §17 non-goal (later T-track, since partly done): “Native Rust ports of `makesis` / `signsis` / `makekeys` / `elf2e32` / `rcomp`.”

Manifest: `language.name` is exactly `cpp`. `--lang` only `cpp`. `LanguageBackend` / `SDKBackend` are empty marker traits.

The user’s 2026-09-16 **Master Development Prompt** (discussed in brainstorming, then demoted) is the source of the *later* language idea. It never says “Rust SDK” either. It does name:

- **Phase 7 — Rust on Symbian:** “Begin with: `no_std`, ARM object generation, linking, basic runtime, FFI. Then incrementally build: `symbian-core`, `symbian-alloc`, `symbian-std`, networking, async, higher-level APIs.”
- **§39 Ultimate architecture:** language split “C++ | Rust” under S60.
- **Phase 9 — Native replacements:** clean-room Rust ports of `rcomp` / `makesis` / UID tools / packaging (tooling, not app language).

North-star §1: that prompt “is **not** an authority for this document.” Returning to Phase 7 is a **new** decision, not restoring a locked Wave 0 path.

## Which SDK?

| | What | In-tree name | Feasible for stock E52 |
|---|---|---|---|
| **A. Tooling SDK** | Host CLI + format codecs in Rust so Wine PE is not required | “Rust orchestration around Wave 0 legacy binaries”; T-track ports | **Now** (mostly done). Still not a compiler. |
| **B. App SDK in Rust** | Write the *app* in Rust (`no_std` / custom rustc target), `extern "C"` / bindgen against **existing** EPOCROOT headers and import `.dso` | Master Phase 7; north-star **non-goal** “Rust as an application language” | **Later**, after C++ hello is proven on device. Research, not Wave 0. |
| **C. Replace Nokia SDK** | Reimplement S60 / Avkon / E32 *APIs* in Rust so EPOCROOT goes away | Not specified. Closest: leftover note “Replacing those is not a tool-port slice.” | **Never for stock E52.** That is a new userspace / ROM. |

A, B, and C are different products. Mixing them is how “full-fledged Rust SDK” sounds cheaper than it is.

## A — tooling (current product)

**Now.** This *is* what the repo is.

Done on the product path: native SIS encode + self-sign (`SisUnsigned` / `SelfSignedDsa`); `uidcrc`; RSC *body* encode for recorded goldens; `GcceBuild` driver that spawns host ELF `g++` / `ld` / Linux `elf2e32_next`. Wine PE is an argv museum, not a live spawn ([legacy-sdk-leftover.md](legacy-sdk-leftover.md)).

Still not A, and not next unless separately scheduled: T3 RSS parse; T4 native `Elf2E32::encode` (crate is flags + `TODO`); `mifconv` / `bmconv`; MMP `LIBRARY` wiring.

A does **not** remove `SYMDEV_EPOCROOT`. Headers, `eexe.lib` / `usrt2_2.lib`, the recorded `.dso` set, and `elf2e32 --libpath` stay. T4 encode would still read that libpath.

“Future Rust reimplementations are **clean-room** from format specs and golden behaviour” ([licensing.md](licensing.md)) is A/T-track, not B.

## B — write E52 apps in Rust

**Later.** Honest cost: a second compiler ABI on top of Wave 0, not a rename of `language.name`.

Wave 0 hello is **C++** `E32Main` + `e32cons` (`Console::NewL` / `Write`), not Avkon GUI. Recorded link is `-nostdlib -shared`, `--entry _E32Startup`, `eexe.lib`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, `dfprvct2_2.dso`, `drtaeabi.dso`, `scppnwdl.dso`, `drtrvct2_2.dso`, `-lsupc++ -lgcc`. Never `-fPIC`. GNU ld **2.29.1** is required; 2.35 dies on `euser.dso` `.gnu.version_d`.

What B would actually do (after C++ `.exe` installs on a stock E52 — Hardware M0 — so failures are not “maybe the SIS is wrong”):

1. Hand-written rustc **target spec** (Hypothesis: `armv5te-unknown-none-eabi` / a custom `arm-unknown-symbian` JSON). `os: none`, soft-float, **relocation-model static**, panic=abort. Not `arm-unknown-linux-*` (glibc syscalls). Not in-tree; **Needs experiment**.
2. `#![no_std]` crate, `#[no_mangle] extern "C" fn E32Main()` or a tiny C++ trampoline still compiled by `g++`. rustc does not magically provide `_E32Startup`; that symbol lives in `eexe.lib`.
3. Same **ld 2.29.1** argv and DSO set, then the same `elf2e32` → native SIS. rustc/LLVM **does not** replace that pipeline. LLD is untested; Hypothesis: same version-script pain as ld 2.35.
4. FFI to **C**, not to Symbian C++ as-is. EPOC headers are C++ with leaves, `TDesC`, `CleanupStack`. bindgen-on-EPOCROOT is not a week of work. Console hello needs a C shim around `Console::NewL` / `Write`, or keep a `.cpp` stub. Avkon (`CAknAppUi`, CONE, resources, `_reg.rss`) is a different mountain; experiment 9 still **Unknown** whether hello even needs `_reg.rsc` to launch (10/11 parked).
5. **panic / alloc / CRT:** `usrt2_2` + `scppnwdl` are the Symbian C++ new/delete / compiler-support DSOs Wave 0 already links. Rust `alloc` would wrap `User::Alloc` / `User::Free` (euser), not jemalloc. `panic=unwind` vs `-fexceptions` is a mixing hazard — start abort-only. Do not pull libstd.

Cost vs Wave 0 C++ hello: C++ hello already compiles and packages. A Rust console “Hello” is a research spike measured in ABI experiments, not a scaffold flag. A GUI/app-framework SDK is closer to a new language product (the thing the north-star explicitly is not).

If they pick B, **do not** set `language.name = "rust"` until an experiment produces an E32 that installs. Do not abandon `g++`.

## C — replace Nokia / EPOCROOT

**Never for stock E52**, if it means the phone runs a Rust reimplementation of E32/S60 instead of ROM `euser` / Avkon.

The E52 already has those DLLs. Import `.dso` files are link-time stubs whose ordinals must match ROM. Deleting EPOCROOT means either (1) still calling ROM via hand-written ordinals (that is **B** with extra pain), or (2) shipping a new userspace/ROM (not this device, not self-signed SIS).

[legacy-sdk-leftover.md](legacy-sdk-leftover.md): “Do not claim EPOCROOT can go away: headers and import libs stay. Replacing those is not a tool-port slice.”

Master Phase 7’s later crates (`symbian-std`, “higher-level APIs”) are **wrappers** (B), not a replacement kernel/userspace. Do not sell them as “SDK goes away.”

## Recommendation

Return to the idea as a **later track**, not by abandoning Wave 0.

- Finish Hardware M0: C++ `hello.exe` SIS installs and launches on stock E52 (experiment 11; not claimed).
- Keep A as the current job (optional T4 encode is still A).
- Park B until that install exists; then a `no_std` + C shim + same `ld`/DSO/`elf2e32` spike, still no Avkon.
- Do not schedule C for this phone.

`LanguageBackend` staying empty is enough of a seam. Empty traits are not a rustc target.
