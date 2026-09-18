# macOS bring-up (T5 inventory)

Read-only research. No implementation. Does **not** unpark T5.

Facts are **Verified** against in-tree types/docs at `origin/main` ~`7f17630` unless labeled **Unknown** / **Needs experiment**. Do not invent Darwin `g++`/`ld` argv. Do not claim E52. Wine PE is not on the product path.

Cites: [experiment-backlog.md](experiment-backlog.md) parked table + experiments 1–6, 38–42; [legacy-sdk-leftover.md](legacy-sdk-leftover.md); [pipeline-and-tools.md](pipeline-and-tools.md); [ported-tools-crates.md](ported-tools-crates.md); [m0-bare-metal-runbook.md](m0-bare-metal-runbook.md); north-star spec §4.3 / §14.3 / §16.4 / §18; `crates/symdev-build/src/{toolchain,driver}.rs`; GCC4Symbian `build-toolchain.sh` as observed in experiment 2 (`fedor4ever/GCC4Symbian` master `fe1b15a`).

Locked (not reopened): Linux is the supported host today; no Windows VM; never invent argv; never claim E52; native tools stay types+methods; Wine PE is an argv museum, not Wave 0.

## Executive answer

`symdev` **does not run a Wave 0 compile on macOS today.** The SIS/UID/RSC crates are ordinary portable Rust. The gap T5 names is the **host compiler/linker** (`arm-none-symbianelf-g++` + GNU ld **2.29.1**), plus a Darwin-runnable `elf2e32` (rebuild `elf2e32_next`, or T4 Rust encode). Copying `EPOCROOT` is data, not a port. Apple codesign is irrelevant to SIS.

T5 stays parked on this Linux host until a macOS edit machine is actually present ([experiment-backlog.md](experiment-backlog.md) 2026-09-18). This note is the checklist for that day — not permission to start it now.

Two topologies exist in the spec. Only the first is T5:

| Topology | What it is | Status |
|---|---|---|
| **T5 native Darwin GCC** | Host Mach-O `arm-none-symbianelf-g++` / `ld` 2.29.1 on the Mac | **Not done.** No `darwin*` GCC4Symbian branch; script `OSTYPE` has `linux*` / `mingw*` / `msys*` only |
| **M3 SSH** | macOS CLI → SSH `ExecutionEnvironment` → Linux GCC (north-star §14.3) | Parked with T5; `LocalEnv` only in-tree |
| **Linux Docker on a Mac** | Same Ubuntu toolchain, not Darwin GCC | Workaround, **not** T5. Do not report “macOS GCC done” |

## 1. What already works on Darwin (cargo / Rust)

No `cfg(target_os = "linux")` in the crates. Product CLI spawns Wine nowhere ([legacy-sdk-leftover.md](legacy-sdk-leftover.md)). Default `cargo test` does not spawn Wine.

| Piece | Crate / type | Wave 0 | Darwin expectation |
|---|---|---|---|
| UID CRC | `UidCrc` + bin `uidcrc` | library used by SIS/RSC | portable; goldens are bytes |
| Unsigned SIS / signed SISX | `SisUnsigned` / `SisPackage` | `symdev package` | portable; `flate2` `zlib` needs host libz (macOS has it) |
| makekeys `-cert` | `SelfSignedDsa` | native PKCS#8 DSA | portable; no Wine |
| RSC body encode | `Rsc` / `RscAppRegistration` | not on hello `.pkg` | portable goldens |
| Manifest / scaffold / clap | `symdev-manifest`, `symdev-cli` | `new` / `package` | should compile |
| Orchestrator | `GcceBuild`, `Toolchain::from_env` | `symdev build` | **compiles**, but `build` fails without Darwin `SYMDEV_GXX` / `SYMDEV_LD` / `SYMDEV_ELF2E32` |
| elf2e32 flags | `Elf2E32::from_args` | argv only | portable; `encode()` is `TODO` |
| Wine `*Tool` | `SisTools`, `MakekeysTool`, `RcompTool`, `UidCrcTool` | not spawned | argv museum; `/usr/bin/wine` default is Linux-shaped and unused on the product path |

**Needs experiment (do not claim green):** `cargo test` on a real Mac with rustc matching workspace `rust-version` `1.98.1`. `LocalEnv` test uses `Command::new("true")` (exists on Darwin). `PathStyle::Posix` is already the local env.

`symdev package` does **not** read `SYMDEV_EPOCROOT`. A Mac with only Rust + libz can write `.sisx` from an existing E32; it cannot produce that E32.

## 2. What must be built / ported (the T5 gap)

Wave 0 spawn list from `Toolchain::from_env` (all six required, no PATH search, no defaults):

| Env | Linux today (experiment 2/3/5) | Darwin need |
|---|---|---|
| `SYMDEV_EPOCROOT` | extracted FP2 tree | copy the tree (data) |
| `SYMDEV_GXX` | ELF `arm-none-symbianelf-g++` **12.1.0** | **Mach-O** same *target*, new *host* |
| `SYMDEV_LD` | ELF `arm-none-symbianelf-ld` **2.29.1** | **Mach-O** GNU ld **2.29.1**, not 2.35 |
| `SYMDEV_ELF2E32` | ELF `elf2e32_next` 3.0 Build 2 | **Mach-O** rebuild, or T4 Rust |
| `SYMDEV_GCC_LIB` | `…/lib/gcc/arm-none-symbianelf/12.1.0` | whatever prefix the Darwin GCC install uses |
| `SYMDEV_GCC_TARGET_LIB` | `…/arm-none-symbianelf/lib` | same |

`GcceBuild` then joins lowercase SDK paths (`epoc32/include/gcce/gcce.h`, `symbian_os_v9.3.hrh`, `armv5/urel`, `armv5/lib`) and recorded compile/link/`elf2e32` argv. Do not change those flags for Darwin. Host OS is not in the argv; only the binary paths change.

### 2a. GCC `arm-none-symbianelf` on Darwin — **blocker, not done**

Verified:

- Pipeline: `arm-none-symbianelf-g++` → `arm-none-symbianelf-ld` → `elf2e32` → native SIS ([pipeline-and-tools.md](pipeline-and-tools.md)).
- No `darwin*` GCC branch (research prompt / runbook / pipeline note). A `linux*` `OSTYPE` arm exists in GCC4Symbian `build-toolchain.sh`.
- That script (experiment 2 clone `fe1b15a`) sets `PREFIX` only for `linux*` / `mingw*` / `msys*`. Darwin `OSTYPE` (`darwin*`) leaves `PREFIX` unset. Script binutils pin is **2.35**, languages `c,c++,lto`, `--target=arm-none-symbianelf`.
- SourceForge GCC 14.2 / 15.2 zips are **Win32 prebuilts** (“Supported OS: WinXP and newer”). Blog: Windows zip; other OS = build the script. There is no Darwin tarball to download.
- This host’s product compiler is GCC **12.1.0** from that script, not 14.2/15.2. Manifest `toolchain.compiler = "gcce-14"` is ignored (`SYMDEV_GXX` is the compiler).

Needs experiment on a Mac (do not invent configure argv):

1. Host compiler to *build* GCC 12.1.0 (Homebrew gcc vs Apple clang; GCC 12 as an **Apple Silicon host** is extra Unknown).
2. Whether `build-toolchain.sh` can be given a Darwin `PREFIX` without retargeting flags.
3. Whether the resulting `g++` accepts the recorded experiment-5 compile argv and produces a usable `hello.o`.

Until those three are recorded, **do not say macOS GCC exists.**

### 2b. GNU ld **2.29.1**, not 2.35 — **blocker**

Experiment 5: same recorded link argv, ld **2.35** → `euser.dso: .gnu.version_d invalid entry` / `error adding symbols: bad value`. ld **2.29.1** linked `hello.elf`. Same failure is documented on the fedor4ever GCC 15.2.0 testing post.

T5 therefore is not “any binutils that GCC4Symbian ships.” After a Darwin GCC build, still produce a **separate** `arm-none-symbianelf-ld` 2.29.1 (Linux used `PREFIX=$HOME/gcc-builds/binutils-2.29.1`, did not overwrite gcc-12.1.0’s ld 2.35). Point `SYMDEV_LD` at 2.29.1. Do not invent a flag workaround; experiment 5 found none.

### 2c. `elf2e32` as a macOS binary, or T4 — **blocker for `symdev build`**

Linux `elf2e32_next` is ELF x86-64; it will not run on Darwin. README: C++14 + `-D__EABI__`; “easy build for different OS: x86 and x64”; experiment 3 used host `g++` + cbp flags + `-include stdint.h` (GCC 15). Sources `#include <unistd.h>` (POSIX).

| Option | What | Honest size |
|---|---|---|
| Rebuild `elf2e32_next` with Darwin `clang++`/`g++` | Host Mach-O that still reads ARM ELF + `$EPOCROOT/…/armv5/lib` | Smaller than T4; **Needs experiment** (no recorded Darwin argv) |
| **T4** `Elf2E32::encode` | In-tree crate is flag parse + `TODO: native ELF→E32 encode` | T2-sized reverse-engineering; still needs `--libpath` DSOs |

Do not vendor original `elf2e32` (EPL-1.0) into this tree ([licensing.md](licensing.md)). Do not pretend T4 is done because the crate exists.

## 3. EPOCROOT files (headers / `.dso` are data)

Not host-architecture binaries. Experiment 1 tree (lowercase via `unshield -L`) is what `GcceBuild` joins:

- `-include` `epoc32/include/gcce/gcce.h` (CRLF text)
- `-D__PRODUCT_INCLUDE__` `epoc32/include/variant/symbian_os_v9.3.hrh`
- `-I` `epoc32/include`, `epoc32/include/variant`
- `-L` `epoc32/release/armv5/urel` (`eexe.lib`) and `armv5/lib` (`usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, …)
- `elf2e32 --libpath=` that `armv5/lib`

Transfer: copy the already-extracted Linux `EPOCROOT` (or re-extract the user’s legal SDK archive on the Mac). Never curl SDK URLs. Never commit SDK. PE tools under `epoc32/tools/*.exe` are unused on Wave 0.

Replacing headers/import libs is not a tool-port slice ([legacy-sdk-leftover.md](legacy-sdk-leftover.md) “What stays SDK forever”).

## 4. Filesystem / case

Linux extraction used lowercase names. Driver hardcodes those lowercase relative paths. Default macOS APFS is **case-insensitive**: a mixed-case Windows SDK copy would likely still open those paths. A **case-sensitive** APFS volume must match the lowercase layout. That is a transfer check, not a code change.

`LocalEnv` is POSIX path style. Do not introduce Windows `\` in host argv. Device dest in `.pkg` stays `!:\sys\bin\` (Symbian, not host).

## 5. Codesign (Apple) is irrelevant

Wave 0 signing is Symbian **self-sign DSA** (`SelfSignedDsa` / SIS type 39), not Apple codesign. Gatekeeper/quarantine on a user-built `g++` is host friction (**nice-to-have**), not SIS validity. Do not codesign `.sisx`. Do not claim notarization.

## 6. CI

No GitHub workflows in this repo. Spec §16.4 (when CI exists): `cargo test`, `fmt`, `clippy` on stubs; **no** SDK, ROM, GCC, SIS tools, phone. Experiments stay in `docs/research/`, never CI (§16.3). M5 EKA2L1 in CI is separately parked.

A future `macos-latest` job may run **cargo only**. Do not put T5 GCC bootstrap in CI until a recorded Darwin experiment exists. Skip experiments 10–11 still do not authorize emulator/E52 claims.

## 7. Ordered checklist (when a Mac is present)

Do not start this on the Linux host. Do not invent argv; copy GCC4Symbian / elf2e32_next docs during the run, then record.

1. Install rustc ≥ workspace `1.98.1`. `cargo test` (no `SYMDEV_*`). Record pass/fail. **This is not T5.**
2. Copy legal `EPOCROOT` (lowercase `gcce.h` / `armv5/lib`). Confirm the four driver paths exist. Case-sensitive volume: verify lowercase.
3. **T5 experiment (new):** build `arm-none-symbianelf-g++` as a Darwin host binary. Record `OSTYPE`, `PREFIX`, host cc, `--version`. Fail closed if `PREFIX` is empty (script has no `darwin*`).
4. **T5 experiment:** build GNU binutils **2.29.1** `arm-none-symbianelf-ld` on that Mac (not the script’s 2.35). Confirm `--version` is 2.29.1.
5. Replay experiment 5 compile argv with Darwin `g++` (path substitution only, never `-fPIC`). Then link with Darwin ld 2.29.1. If 2.35 is tried, expect `euser.dso` `.gnu.version_d` failure — record it; do not “fix” with new flags.
6. **elf2e32:** Darwin-build `elf2e32_next` (C++14, `-D__EABI__`, record argv) **or** only then consider T4 encode against a frozen `hello.exe`. Replay experiment 6 argv. Linux ELF `elf2e32` will not execute.
7. Point the six `SYMDEV_*` at those Darwin paths. `symdev build` then `symdev package`. Same recorded argv dialect.
8. Optional later: macOS `cargo test` in CI; M3 SSH; Docker Linux on the Mac as a **non-T5** workaround.

## 8. Blockers vs nice-to-haves

**Blockers (native `symdev build` on Darwin):**

- Darwin-host `arm-none-symbianelf-g++` (T5). Not built. No darwin script arm. Win32 zips do not help.
- Darwin-host GNU ld **2.29.1** (2.35 rejected by SDK `euser.dso`).
- Darwin-runnable `elf2e32` (Mach-O `elf2e32_next` or T4). Linux binary is the wrong OS ABI.
- User `EPOCROOT` on that machine (data copy).
- A macOS edit host in the first place (why T5 is parked).

**Not blockers:**

- Native SIS / makekeys / uidcrc / RSC encode (already Rust).
- Wine / `makesis.exe` / `signsis.exe` (off product path).
- Apple codesign / notarization.
- M3 SSH (alternate topology, also parked).
- M5 EKA2L1, experiment 11 E52 (unchanged skips).
- RSS parse / `START RESOURCE` / mifconv / MMP `LIBRARY` wiring (Linux gaps too).
- CI macOS runners (nice-to-have after cargo is recorded).

**Nice-to-haves:** Homebrew rust; libz via Xcode CLT; Gatekeeper allow for self-built GCC; case-sensitive APFS test; Docker/Colima Linux builder (workaround, still Linux GCC); documenting Darwin `PREFIX` patch **after** an experiment, not before.

## 9. What not to say

- Do not say “SIS tools block macOS.” They do not.
- Do not say “macOS GCC is done” or “just run `build-toolchain.sh` on a Mac.”
- Do not unpark T5 in [experiment-backlog.md](experiment-backlog.md) from this note.
- Do not treat Linux Docker on Apple Silicon as T5.
- Do not invent Darwin configure/make argv here.
- Do not claim E52 because a Mac can `cargo test`.
