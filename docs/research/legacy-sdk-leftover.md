# Legacy SDK / Wine leftover (origin/main `7f17630`)

Inventory only. No implementation. Facts are **Verified** against in-tree types/methods at `7f17630` (`Drop the unused UidCrc::parse_token stub…`) unless labeled **Unknown**.

Cites: [experiment-backlog.md](experiment-backlog.md) experiments 1–9, 13–42; [ported-tools-crates.md](ported-tools-crates.md); [pipeline-and-tools.md](pipeline-and-tools.md); [hardcoded-values.md](hardcoded-values.md) (uncommitted); crates `symdev-build`, `symdev-cli`, `symdev-sis`, `symdev-uidcrc`, `symdev-makekeys`, `symdev-rcomp`, `symdev-elf2e32`. `.cursor/rules/rust-types-and-methods.mdc`: Wine/env/argv stay on `*Tool`, not value types.

Locked (not reopened): Linux host edit+build; Wave 0 `arm-none-symbianelf-g++` → `ld` → `elf2e32` → `makesis` → `signsis`; no Windows VM; self-sign; never invent argv; never claim E52 until stock install+launch.

Parked (not this host’s next slice): **T5** macOS GCC; **M3** SSH `ExecutionEnvironment`; **M5** EKA2L1 (`SYMDEV_EKA2L1` / `SYMDEV_ROM` unset); **E52** experiment 11 `skip`.

## Executive answer

`symdev build` / `symdev package` on this host **do not spawn Wine PE**. Packaging is native (`SisUnsigned` + `SelfSignedDsa`). The live Wave 0 spawns are host ELF binaries from `Toolchain::from_env`: `SYMDEV_GXX`, `SYMDEV_LD`. The post-link step is native (`symdev-elf2e32`, experiments 45–47) unless the optional `SYMDEV_ELF2E32` points at an external Linux `elf2e32_next`.

What is still “the SDK” is **files under `SYMDEV_EPOCROOT`**: headers, `.lib` / `.dso`, and `elf2e32 --libpath`. Wine `*Tool` types still *name* PE paths and pin recorded argv; nothing in `GcceBuild` / `SisPackage` / `symdev` CLI `Command::new`s them.

**Primary next slice:** drop the unused `symdev-build` crate deps (`symdev-uidcrc`, `symdev-rcomp`, `symdev-elf2e32`) left by the crate split. Do **not** start T3 RSS parse, T4 native E32 encode, or MMP `LIBRARY` wiring.

---

## 1. What is left of the legacy SDK?

### 1a. Still spawned (Wine PE)

**None on the product path** (`symdev build`, `symdev package`, native tool bins that Wave 0 actually runs).

`LocalEnv::run_blocking` is only used from `GcceBuild::run_tool`, and `args[0]` is `gxx` / `ld` / `elf2e32` from env — not `/usr/bin/wine`. CLI `package` never calls `SisTools::from_env`. Test `package_missing_epocroot_still_packages` removes `SYMDEV_EPOCROOT` and `SYMDEV_WINE` and still writes `.sisx`.

Wine PE adapters that **exist but are not spawned** (argv museums + unit tests; default `cargo test` does not run Wine):

| PE | Type | How the path is built | Recorded argv | Spawned by |
|---|---|---|---|---|
| `makesis.exe` | `SisTools` | `SisTools::from_env`: `SYMDEV_EPOCROOT` + `epoc32/tools/makesis.exe`; wine = `SYMDEV_WINE` or `/usr/bin/wine` | `makesis_args`: `wine makesis.exe -v pkg sis` (experiment 7) | nobody |
| `signsis.exe` | `SisTools` | same dir `signsis.exe` | `signsis_args`: positionals sis sisx cer key password (experiment 8) | nobody |
| `makekeys.exe` | `MakekeysTool` | `MakekeysTool::from_env`: `SYMDEV_EPOCROOT` + `epoc32/tools/makekeys.exe`; same wine default | `args`: `-cert -expdays 3650 -password … -len 2048 -dname "CN=Joe Bloggs…"` (experiment 8) | nobody (CLI uses `SelfSignedDsa::generate`) |
| `rcomp.exe` | `RcompTool` | `new(wine, rcomp)` only; **no** `from_env` | `-u -o… -s… -i…` and optional `-h…` (experiments 9 / 41) | nobody (`symdev build` has no rcomp verb) |
| `uidcrc.exe` | `UidCrcTool` | `new(wine, uidcrc)` only | `wine uidcrc.exe 0x… 0x… 0x… [outfile]` (experiment 13) | nobody (native `uidcrc` bin) |

No `*Tool` type for SDK `cpp.exe` (`epoc32/gcc/bin/cpp.exe`), `elf2e32.exe` (PE), `mifconv.exe`, `bmconv.exe`, `petran`, or `epocrc.pl`. Those PEs are research-only (experiments 3, 9). `WINEPATH=<EPOCROOT>/epoc32/tools` was required for Wine `rcomp.exe` to find sibling `uidcrc.exe`; that env is not in any type.

`Elf2E32Tool` is **not** Wine: it prefixes `SYMDEV_ELF2E32` (Linux ELF). `GcceBuild::elf2e32_args` builds the recorded argv; without `SYMDEV_ELF2E32` it feeds that argv to native `Elf2E32::from_args` + `encode` instead of spawning.

### 1b. Still consumed as files (EPOCROOT — not a PE spawn)

`Toolchain::from_env` requires all six (no defaults):

| Env | Role |
|---|---|
| `SYMDEV_EPOCROOT` | SDK root (`epoc32/…`) |
| `SYMDEV_GXX` | host `arm-none-symbianelf-g++` |
| `SYMDEV_LD` | host `arm-none-symbianelf-ld` (Wave 0: GNU ld **2.29.1**) |
| `SYMDEV_ELF2E32` | optional external Linux `elf2e32` (`elf2e32_next`, experiment 3); unset → native |
| `SYMDEV_GCC_LIB` | GCC libdir (`…/lib/gcc/arm-none-symbianelf/12.1.0`) |
| `SYMDEV_GCC_TARGET_LIB` | target lib (`…/arm-none-symbianelf/lib`) |

`GcceBuild` joins `epocroot` (experiment 5–6 recorded paths):

**Headers / `-I` (compile):**

- `$EPOCROOT/epoc32/include/gcce/gcce.h` (`-include`)
- `$EPOCROOT/epoc32/include/variant/symbian_os_v9.3.hrh` (`-D__PRODUCT_INCLUDE__`)
- `$EPOCROOT/epoc32/include`
- `$EPOCROOT/epoc32/include/variant`
- plus `$SYMDEV_GCC_LIB/include` (host GCC, not SDK)

**Link search + objects:**

- `$EPOCROOT/epoc32/release/armv5/urel` → `-l:eexe.lib`
- `$EPOCROOT/epoc32/release/armv5/lib` → `-l:usrt2_2.lib` then `-l:euser.dso -l:dfpaeabi.dso -l:dfprvct2_2.dso -l:drtaeabi.dso -l:scppnwdl.dso -l:drtrvct2_2.dso`
- plus host `-lsupc++ -lgcc` from `SYMDEV_GCC_*`

**Post-link:** `elf2e32 --libpath=$EPOCROOT/epoc32/release/armv5/lib` (import DSOs). Native `Elf2E32::encode` does not exist yet; the Linux binary still reads that tree.

`symdev package` does **not** read `SYMDEV_EPOCROOT`.

### 1c. Fully native already

| Piece | Type / bin | Evidence |
|---|---|---|
| UID CRC | `UidCrc::{checked,bytes,line}` + bin `uidcrc` | experiment 13 goldens; bin writes 16-byte file or prints the stdout line |
| Unsigned SIS | `SisUnsigned::encode` / `Makesis::run` | experiment 38; Wave 0 `TYPE=SA` `&EN` + platform `0x102752AE`; EXE + optional `_reg.rsc` (experiment 43) |
| Signed SISX | `SisUnsigned::encode_signed` | experiments 39–40; `SisPackage::package` |
| makekeys `-cert` | `SelfSignedDsa::generate` + `Makekeys::run` | experiment 40; PKCS#8 `BEGIN PRIVATE KEY`, DSA 1024/160, recorded DN |
| RSC body goldens | `Rsc` / `RscAppRegistration` / `RscLtext16` / `RscUid` | experiments 41–42; byte-equal Wine `driveinfo_reg.rsc` / `filebrowseapp_reg.rsc` hex |

Product CLI uses the libraries, not the tool bins ([ported-tools-crates.md](ported-tools-crates.md)).

### 1d. Stub TODO (no fake encode)

| Gap | Where | Wave 0 impact |
|---|---|---|
| `Elf2E32::encode` beyond observed EXE/softvfp (DLLs, exports, data sections, other capabilities) | `symdev-elf2e32` | EXE hello path is native (experiments 44–47) |
| RSS source parse / `.rsg` | `rcomp` bin `todo!("RSS source parse / .rsg")`; `Rcomp::from_args` TODO on `-v -p -l -force -{uid2,uid3}` | not on Wave 0; `START RESOURCE` not compiled |
| `signsis` inflate of an existing `.sis` | `Signsis::run` TODO; bin `todo!` | not Wave 0 (`encode_signed` on the library) |
| makesis flags `-h -i -s -d`; pkg files beyond EXE + `_reg.rsc` (`TYPE=SA`); caps from E32 | `Makesis` | Wave 0 `.pkg` has no caps line; caps come from Manifest → elf2e32 / SIS type 41 |
| makekeys `-req`/`-view`; `-expdays` ≠ 3650; `-len` ≠ `2048` token; other `-dname` | `Makekeys::from_args` | native keygen ignores `-len 2048` (DSA 1024/160) |
| `mifconv` / `bmconv` | no crate | not on hello |
| MMP `LIBRARY` / `SYSTEMINCLUDE` / `USERINCLUDE` / `STATICLIBRARY` / `CAPABILITY` / `UID` / `TARGETPATH` / `START RESOURCE` | parsed onto `Mmp`, **ignored** by `GcceBuild::build` | scaffold MMP is `TARGET`/`TARGETTYPE`/`SOURCEPATH`/`SOURCE` only |
| `START RESOURCE` rss filename | `Mmp::parse` drops the `RESOURCE <file>` token; keeps inner lines only | experiment 41: wiring stays later |

---

## 2. What we should do next

### Primary slice: drop unused `symdev-build` deps

`crates/symdev-build/Cargo.toml` at `7f17630` depends on `symdev-uidcrc`, `symdev-rcomp`, and `symdev-elf2e32`. `lib.rs` / `driver.rs` / `package.rs` never `use` them. Split commit `411eb65` added the deps; orchestrator still builds argv by hand and packages via `symdev-sis` + `symdev-makekeys` only.

Ponytail: deletion. One `Cargo.toml` (and lockfile) change. Does not invent argv. Does not touch E52. Keep the Wine `*Tool` types in their own crates — they are the recorded experiment pins (`never invent argv`), not live spawns.

Do **not** expand that slice into deleting `SisTools` / `MakekeysTool` / `RcompTool` / `UidCrcTool` unless a later cleanup explicitly retires the argv goldens.

### Rejected as next (why)

| Candidate | Why not now |
|---|---|
| **T3 remainder** (RSS parse / `.rsg` / `START RESOURCE`) | Native body encode already matches the two `_reg.rsc` goldens. Wave 0 hello `.pkg` has no rsc. Experiment 9: whether launch needs `_reg.rsc` is **Unknown** until 10/11, both parked. Preprocess still Wine `cpp.exe` if you re-run PE rcomp. |
| **T4** native `elf2e32` encode | Last *named* Wave 0 tool spawn, but it is already Linux C++ (`elf2e32_next`), not Wine PE. Encode is T2-sized reverse-engineering; in-tree crate is a flag parser + `TODO`. Native encode would still need `$EPOCROOT/…/armv5/lib` (`--libpath`). |
| **MMP `LIBRARY` / include wiring** | Would invent `-l:` / extra `-I` beyond experiment-5’s fixed DSO set. Scaffold has no `LIBRARY`. A second app needs a **recorded** extra-lib experiment first, then wire `Mmp.library` into `GcceBuild::link_args`. |
| **signsis bin inflate** | Product path already signs via `encode_signed`. Inflating a foreign `.sis` is not Wave 0. |
| **T5 / M3 / M5 / E52** | Parked. Skip does not authorize device/emulator claims. |

### After the primary (not this slice)

1. If a real MMP grows `LIBRARY` / `SYSTEMINCLUDE`: record argv (same dialect as experiment 5), then wire. No TOML `libs = []`.
2. If packing `_reg.rsc` becomes required (launch proof, not this host): T3 RSS parse + `START RESOURCE` (keep rss filename) + pkg dest `!:\private\10003a3f\import\apps\`. Until then, do not spawn `rcomp.exe`.
3. T4 only after an E32 golden dump experiment (hello.exe layout), same discipline as T2. Do not claim EPOCROOT can go away: headers and import libs stay.

### What stays SDK forever on Wave 0

`gcce.h`, `symbian_os_v9.3.hrh`, `epoc32/include`, `eexe.lib` / `usrt2_2.lib`, the recorded `.dso` set, and `elf2e32 --libpath`. Replacing those is not a tool-port slice.

---

## Sources (in-tree)

- `crates/symdev-build/src/{toolchain,driver,package,mmp,lib}.rs`
- `crates/symdev-cli/src/main.rs` (`build_project` / `package_project`)
- `crates/symdev-sis/src/package.rs` (`SisTools`)
- `crates/symdev-makekeys/src/lib.rs` (`MakekeysTool`, `SelfSignedDsa`)
- `crates/symdev-rcomp/src/lib.rs` (`RcompTool`, `Rcomp`)
- `crates/symdev-uidcrc/src/lib.rs` (`UidCrcTool`)
- `crates/symdev-elf2e32/src/lib.rs` (`Elf2E32::encode` TODO)
- [ported-tools-crates.md](ported-tools-crates.md); [experiment-backlog.md](experiment-backlog.md) parked table + experiments 3–9, 38–42
