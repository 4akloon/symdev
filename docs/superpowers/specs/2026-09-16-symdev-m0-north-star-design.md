# symdev M0 north-star + local de-risk package

Date: 2026-09-16
Status: approved design (brainstorming). This document is the source of truth for the next implementation plan. It does not itself add crates, Dockerfiles, or runbooks.

Technical facts marked **Verified** come from the research-informed implementation prompt (`symdev-prompt.md`, v2) and must not be re-litigated. Command fragments copied here are only those Verified sequences. Anything not observed in a primary source is **Unknown** or **Needs experiment**. This spec does not invent argv, URLs, or install steps.

A second “Master Development Prompt” was discussed during brainstorming. It is **not** an authority for this document. The only pieces kept from it are: extra **empty trait names** in `symdev-core` (§6), and a **north-star command list** that must not appear in the §17 binary (§8.2).

## 1. Context and goal

symdev is a Rust CLI that orchestrates a modern developer workflow for **legacy Symbian hardware**. It is not a Symbian compiler and not a Rust-to-Symbian language product.

Nokia E52 is the **first validation target**, not the architecture.

North-star MVP (later milestones, not this cycle):

```
symdev new hello --target nokia-e52 --lang cpp
cd hello
symdev build
symdev package
symdev deploy
```

Intended first device: **Nokia E52** (RM-469, “Stella”), Symbian OS 9.3 / EKA2, **S60 3rd Edition Feature Pack 2**, ARM11 / ARMv5, no NEON. Platform UID for `.pkg` is **0x102752AE** (Verified).

This repository is empty except for git init. The first **implementation** after this spec is a thin lock of names and interfaces plus the research prompt’s §17 de-risk package, all on **this Linux machine as both edit host and build host**.

Two different “M0” meanings are used on purpose:

| Name | Meaning | When it applies |
|---|---|---|
| **§17 accept** | Runbook + Dockerfile + three crates + parser spec + experiment list (+ research bootstrap, EKA2L1 appendix). No requirement that a `.sisx` has been produced. | This spec’s implementation cycle |
| **Hardware M0** | Hand-built `.sisx` installs and the app launches on a **stock E52** | Required before anyone may claim E52 support |

A macOS-hosted loop (edit on Apple Silicon, remote build, emulator, device) is a **later milestone list**. It is not the definition of done for this document.

This spec **does not authorize M1 coding** (no `bld.inf`/`.mmp` driver, no spawning GCC from the CLI). Completing §17 does not lift that gate.

## 2. Conflicts decided

Do not re-open these.

| # | Decision |
|---|---|
| Session | Spec = thin north-star + detailed §17 de-risk. Not full M1–M4. Not a master-prompt “complete DX” MVP. |
| Host | This Linux machine is edit + build. macOS is later. No Windows VM in this spec’s planned path. Future execution environments are possible; none is scheduled here as the next backend. |
| Conflict 1 | First build path is Linux GCC ELF: `arm-none-symbianelf-g++` → `arm-none-symbianelf-ld` → `elf2e32` → `makesis` → `signsis`. Bypass `abld`/`makmake` on this path. Bare-metal first; Docker after the runbook is believed correct. SDK bind-mounted, not baked into the image. |
| Conflict 2 | §17 creates workspace + `symdev-cli` + `symdev-core` + `symdev-manifest` only. Core declares the empty trait names in §6. No `symdev-project` crate, no `platforms/` tree. |
| Conflict 3 | clap implements only `new`, `build`, `package`, `deploy` (all not-implemented). Other verbs are a spec list, not binary surface. |
| Conflict 4 | Host / emulator / device tests are a north-star distinction only. No test protocol schema, no test runner, no `symdev test` command. |
| Conflict 5 | No UIQ, Java ME, or S40 sections. Java ME is out of this spec. |
| Conflict 6 | Done for this document = runbook + Dockerfile + three crates + parser spec + experiment list. Optional EKA2L1 only with a user-supplied ROM. Never claim E52 support until stock E52 install+launch. |
| License | Undecided. No `LICENSE` file this cycle. Still forbid proprietary blobs in git. |
| Hard gate | Do not write the build driver until a hand-built `.sisx` is proven. |

## 3. Non-goals

Out of this spec and out of the §17 implementation plan:

- Writing the compiler/linker/`elf2e32` driver.
- Implementing `new` / `build` / `package` / `deploy` beyond clap stubs and manifest validation.
- Putting `doctor`, `test`, `run`, `debug`, `sdk`, `toolchain`, `emulator`, or `devices` in the binary.
- Java ME, UIQ, S40, Rust as an application language, hot reload.
- macOS as edit host; SSH execution environment; a Windows VM backend.
- Native ARM64 macOS GCC.
- Native Rust ports of `makesis` / `signsis` / `makekeys` / `elf2e32` / `rcomp`.
- Shadow mode (run legacy + native, diff, do not fail).
- Claiming “E52 supported” or “emulator supported” as a product statement.
- A `LICENSE` file.
- Committing, bundling, baking into Docker, downloading, or scraping: S60 SDKs, WTK, proprietary JARs, ROM/firmware dumps.
- Copying EKA2L1 source (GPL-3.0) into this tree. Invoke it as a **separate process** only.
- CI jobs that need GCC, SDK, SIS tools, EKA2L1, or a phone.
- Extra crates (`symdev-project`, `symdev-config`, `exec-env`, `platform-symbian`, `lang-cpp`, `build-symbian`, `pkg-sis`, `tool-elf2e32`, transports, `emu-eka2l1`, or a `platforms/` tree).

## 4. Architecture

### 4.1 Role

Rust orchestration around **Wave 0 legacy binaries**. Backends are traits so a binary can later be replaced by a native implementation without changing the CLI. Registration is **static** (match on backend names, as cargo/rustup do). No plugin loading.

### 4.2 Verified pipeline (do not invent)

```
arm-none-symbianelf-g++  →  arm-none-symbianelf-ld  →  elf2e32  →  makesis  →  signsis
```

`abld` and `makmake` (Perl) are bypassed entirely on this path. Never pass `-fPIC` or `-fPIE` (Verified: causes “Import relocation does not refer to code segment”).

**Compile flags** (Verified against fedor4ever’s manual-build write-up):

```
-O2 -fexceptions -march=armv5t -mapcs -mthumb-interwork -mthumb -msoft-float
-D__SYMBIAN32__ -D__EPOC32__ -D__MARM__ -D__GCCE__ -D__EXE__
-include <SDK>/GCCE.h
-D__PRODUCT_INCLUDE__="<SDK>/Symbian_OS.hrh"
```

Optional size flags (Verified as a size-reduction technique, not required for a first hello): `-ffunction-sections -fdata-sections` plus linker `--gc-sections --strip-discarded`.

**Link flags** (Verified):

```
--target1-abs --no-undefined -nostdlib -shared
-Ttext 0x8000 -Tdata 0x400000 --strip-debug
--entry _E32Startup -u _E32Startup
```

Link against: `eexe.lib`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, `drtaeabi.dso`, `scppnwdl.dso`, plus `-lsupc++ -lgcc`.

Exact library search paths, C runtime objects, and flag order are **Needs experiment**. The runbook may only assemble argv from Verified fragments plus experimentally recorded paths.

**elf2e32** (Verified fragment):

```
elf2e32 --uid1=0x1000007a --uid3=<UID3> --capability=<caps> --fpu=softvfp
        --targettype=EXE --output=hello.exe --elfinput=hello.elf
        --linkas=hello{000a0000}[<UID3>].exe --libpath=<SDK>/armv5/LIB
```

soname / `--linkas` format is `<name>{version}[uid3].<ext>` and **must match** between the linker `-soname` and `elf2e32 --linkas` (Verified).

**Signing** is **self-sign only** in this product phase. Symbian Signed is closed (Verified). `makekeys -expdays 3650` requires Symbian 9.2+ tools (Verified). Without `-expdays` a certificate lasts about one year.

### 4.3 Host topology

| Role | This cycle | Later (not scheduled as the next backend here) |
|---|---|---|
| Edit | This Linux machine | macOS Apple Silicon |
| Build | This Linux machine, bare metal first | Same toolchain in Docker on Ubuntu 24.04; other execution environments are possible |
| Transport to builder | Local filesystem | SSH when macOS is the edit host |
| Transport to device | Not automated | Print `.sisx` path, then memory-card copy, then Bluetooth OBEX |

A **local** `ExecutionEnvironment` is the north-star impl for this host. It is not created in §17. Future environments are possible; this spec does not name a Windows VM as the next step.

### 4.4 Toolchain packaging

1. Write a **bare-metal runbook** for this host.
2. When that command sequence is **believed correct** (internally consistent with Verified flags, Unknowns labeled — **not** “already executed successfully”; SDK may still be absent), write a **Dockerfile**.
3. The image contains GCC + ELF/SIS tools. The **SDK is bind-mounted**, never copied into the image. A ROM, if any, is likewise bind-mounted / read from a host path.

“Believed correct” is a documentation standard, not an empirical green build.

### 4.5 Capabilities and UIDs (Verified)

Twenty capabilities, three tiers. **User-grantable (self-signable), exactly six:** `LocalServices`, `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location` (Location added in FP2).

System (7, need DevCert): `PowerMgmt`, `ProtServ`, `ReadDeviceData`, `SurroundingsDD`, `SwEvent`, `TrustedUI`, `WriteDeviceData`.

Manufacturer (3): `AllFiles`, `DRM`, `TCB`.

§17 and all self-signed builds **hard-error** on any capability outside the six. There is no opt-in identity in this spec. Until a later spec adds one, “refuse privileged capabilities unless the user opts in” is implemented as **always refuse**.

UID policy:

- Default / hello: test range **0xE0000000–0xEFFFFFFF** (never required registration).
- Self-signed range **0xA0000000–0xAFFFFFFF** is allowed if the user sets `uid3` explicitly.
- Protected range **< 0x80000000** is a hard error under `signing.mode = "self-signed"` (will not install).

`.pkg` must include platform dependency **0x102752AE**. Omitting it produces “App is incompatible with phone” (Verified).

### 4.6 Wave 0 vs later porting

Wave 0: every tool is a subprocess. GCC/`ld` stay external. `elf2e32` stays the C++ port (fedor4ever) until a mature golden corpus exists. `abld`/`makmake` are never wrapped on this path; they are replaced by a Rust driver in **M1**, which this spec does not schedule.

Shadow mode and native reimplementations are later: run both, diff, log, **do not fail the build** on mismatch. Not in §17.

## 5. Crate map

### 5.1 Created in §17

Workspace root `Cargo.toml` (resolver `"3"`), edition **2024**, `rust-version = "1.98.1"` (current stable as of 2026-09-03; native `async fn` in traits, no `async-trait` crate).

| Crate | Path | Role in §17 |
|---|---|---|
| `symdev-cli` | `crates/symdev-cli` | Binary `symdev`. clap derive. Four subcommands exist and refuse to run. |
| `symdev-core` | `crates/symdev-core` | Trait + shared type definitions only. No backend impls. |
| `symdev-manifest` | `crates/symdev-manifest` | `symdev.toml` schema, parse, validate. This validation is real. |

`symdev-cli` depends on the other two. `symdev-core` does not depend on clap. `symdev-manifest` does not depend on clap. No `tokio` dependency in §17 (async traits are defined; nothing is executed). CLI deps: `clap` (derive). Manifest deps: `serde`, `toml`. `thiserror` is allowed in core and manifest for error types.

### 5.2 Not created

§17 must not add other crates or a `platforms/` tree. Names from later plans (`exec-env`, `build-symbian`, `pkg-sis`, `emu-eka2l1`, `symdev-project`, and similar) stay unused.

### 5.3 Directories

| Path | This cycle |
|---|---|
| `crates/*` | The three crates above |
| `docker/` | `Dockerfile` after the runbook is believed correct |
| `docs/research/` | Bootstrap notes, runbook, EKA2L1 appendix, parser spec, experiment backlog |
| `golden/` | Do **not** create. |
| `fixtures/` | Do **not** create. M4 later. |

## 6. Traits (`symdev-core`)

Names below are locked. Types exist so signatures compile. No method is called from the §17 CLI. There are no impls.

The six new names (`PlatformBackend`, `LanguageBackend`, `SDKBackend`, `ToolchainBackend`, `TestBackend`, `RuntimeBackend`) are **empty marker traits**. The original six keep the previously approved method signatures.

```rust
pub enum PathStyle { Posix, Windows }

pub struct RemotePath(/* opaque; Display/Debug only in §17 */);
pub struct Output { pub status: i32, pub stdout: Vec<u8>, pub stderr: Vec<u8> }

pub struct Project { pub root: std::path::PathBuf }
pub struct Artifact { pub path: std::path::PathBuf }
pub struct Package {
    pub primary: std::path::PathBuf,
    pub companions: Vec<std::path::PathBuf>,
}

pub trait PlatformBackend {}
pub trait LanguageBackend {}
pub trait SDKBackend {}
pub trait ToolchainBackend {}
pub trait TestBackend {}
pub trait RuntimeBackend {}

pub trait ExecutionEnvironment {
    async fn run(&self, cmd: std::process::Command, cwd: &RemotePath) -> Result<Output>;
    async fn push(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()>;
    async fn pull(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()>;
    fn path_style(&self) -> PathStyle;
}

pub trait BuildBackend {
    fn build(&self, project: &Project) -> Result<Vec<Artifact>>;
}

pub trait PackageBackend {
    fn package(&self, artifacts: &[Artifact]) -> Result<Package>;
}

pub trait DeviceTransport {
    fn deliver(&self, package: &Package) -> Result<()>;
}

pub trait EmulatorBackend {
    fn list(&self) -> Result<Vec<String>>;
    fn start(&self, id: &str) -> Result<()>;
    fn stop(&self, id: &str) -> Result<()>;
    fn install(&self, id: &str, package: &Package) -> Result<()>;
    fn launch(&self, id: &str, app: &str) -> Result<()>;
    fn logs(&self, id: &str) -> Result<String>;
}

pub trait DebuggerBackend {
    fn attach(&self, target: &str) -> Result<()>;
    fn breakpoint(&self, spec: &str) -> Result<()>;
    fn r#continue(&self) -> Result<()>;
    fn step(&self) -> Result<()>;
    fn backtrace(&self) -> Result<String>;
}
```

`Result` is `symdev-core::Result<T>` using a small `Error` type that can represent `NotImplemented { feature: &'static str, milestone: &'static str }` plus opaque `Other`. Trait methods have no default bodies that panic.

`BuildBackend` returns `Vec<Artifact>` from a chain of steps, not a hard-coded triple. C++ Wave 0 on this host is still compile → link → `elf2e32`.

`ExecutionEnvironment::path_style`: local Linux is `Posix`. SDK tools may still want Windows-style paths inside `.pkg`; that translation is a runbook/M1 concern (**Needs experiment**).

## 7. Manifest schema (`symdev-manifest`)

File name: `symdev.toml` at the project root. `symdev.lock` is north-star only: **do not parse or write it in §17**.

Canonical example (hello defaults, not a working template):

```toml
[package]
name = "hello"
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
# uid3 omitted → allowed; M4 `new` will write a test-range UID.
capabilities = []
vendor = "symdev"

[signing]
mode = "self-signed"
```

### 7.1 Fields

| Field | Required | Rule |
|---|---|---|
| `package.name` | yes | Non-empty; `[A-Za-z][A-Za-z0-9_]*` |
| `package.version` | yes | Three non-negative integers `MAJOR.MINOR.PATCH` (maps later to `.pkg` `major,minor,patch`) |
| `target.device` | yes | Exactly `nokia-e52`. Any other string is a hard error. This names the intended first validation device; it is not a claim that the device is supported. |
| `platform.family` / `version` / `feature_pack` | no | If any platform key is present, all three must be present and equal `s60`, `3rd`, `fp2`. If omitted, inferred as that triple from `nokia-e52`. |
| `language.name` | yes | Exactly `cpp`. |
| `toolchain.sdk` | no | If present, exactly `s60-3rd-fp2`. |
| `toolchain.compiler` | no | If present, `gcce-14` or `gcce-15`. Default when omitted: `gcce-14`. Which binary that maps to is **Needs experiment**. |
| `symbian.uid3` | no | String `0x` + 8 hex digits. If present with self-sign: must be in `0xE0000000–0xEFFFFFFF` or `0xA0000000–0xAFFFFFFF`. If omitted, parse succeeds and the in-memory struct has `uid3: None`. §17 does **not** generate UIDs. M4 `new` generates a test-range UID and writes it. |
| `symbian.capabilities` | no | Default `[]`. Each entry must be one of the six user-grantable names, exact spelling and case. Duplicates: hard error. Unknown names: hard error. Privileged names: hard error. |
| `symbian.vendor` | no | Default `"symdev"`. Non-empty if present. |
| `signing.mode` | no | Default `"self-signed"`. Any other value: hard error. |
| `signing.cert` / `signing.key` | no | If present, non-empty relative or absolute paths. §17 does not check that files exist. |

Forbidden: `language.name` other than `cpp`; unknown top-level tables beyond `package`, `target`, `platform`, `language`, `toolchain`, `symbian`, `signing`. Unknown tables/keys: hard error (fail closed).

`target.device = "nokia-e52"` is the only legal target identity in **§17**.

### 7.2 Hello defaults (schema documentation, not M4 templates)

When `symdev new` exists (M4), it will write: auto `uid3` in `0xE0000000–0xEFFFFFFF`, `capabilities = []`, `vendor = "symdev"`, `language.name = "cpp"`, `target.device = "nokia-e52"`. Real `.cpp` / `.mmp` / `bld.inf` / `.pkg` / `_reg.rss` skeletons are **M4**, not §17.

## 8. CLI surface

Binary: `symdev`.

### 8.1 clap in §17 (only these)

```
symdev new <name> --target <device> --lang <lang>
symdev build
symdev package
symdev deploy
```

- `new`: positional `name` required (same pattern as `package.name`). `--target` required; only value `nokia-e52`. `--lang` optional, default `cpp`, only value `cpp`. After clap accepts: **not implemented**. Does not create files. Does not look for `symdev.toml`.
- `build` / `package` / `deploy`: no extra arguments. Look for `./symdev.toml`. Missing file: **hard error** (not “not implemented”). Present: parse + validate; on failure, validation error; on success: **not implemented**.
- No args or `--help`: clap help, exit 0.
- Unknown subcommand/args: clap usage error.

**§17 CLI never spawns** GCC, `ld`, `elf2e32`, `makesis`, `signsis`, `makekeys`, or EKA2L1. Humans follow the runbook.

### 8.2 North-star command names (spec list only)

These verbs may appear in later milestones. They **must not** be clap subcommands in §17. `--help` must not list them.

`doctor`, `test`, `run`, `debug`, `sdk`, `toolchain`, `emulator`, `devices`.

### 8.3 Exit codes

| Situation | Exit | stderr |
|---|---|---|
| Help / clap success | 0 | clap’s help on stdout |
| Clap usage error | 2 | clap default |
| Not implemented | 1 | `error: not implemented: 'symdev <sub>' (unlocks at <milestone>)` |
| Missing or invalid manifest | 1 | `error: invalid manifest: <reason>` (missing file uses this prefix too, reason `no symdev.toml in current directory`) |

Milestones in the not-implemented message: `new` → M4; `build` → M1; `package` → M2; `deploy` → M4 (deploy automation; printing a path could come earlier, but this spec does not implement it).

## 9. §17 deliverables

Implementation after this spec follows this **order**. The Dockerfile must not be written before the runbook exists and is believed correct.

### 9.1 `docs/research/` bootstrap

Promote facts from the Telegram prompt into notes split **Verified / Likely / Unknown / Needs experiment**. No new command sequences. Suggested files (names may be combined if they stay clearly sectioned):

- `docs/research/pipeline-and-tools.md`
- `docs/research/uids-capabilities-signing.md`
- `docs/research/licensing.md`

Do not treat “Likely” as Verified. Do not drop Unknowns.

### 9.2 M0 bare-metal runbook

Path: `docs/research/m0-bare-metal-runbook.md`. Outline in §10. Audience: a human on this Linux host.

### 9.3 Dockerfile

Path: `docker/Dockerfile`. Outline in §11.

### 9.4 EKA2L1 appendix

Path: `docs/research/eka2l1.md`. Outline in §12.

### 9.5 Cargo workspace stubs

As §5–§8. Plus a root `.gitignore` that ignores at least `/target`, `*.sis`, `*.sisx`, `*.cer`, `*.key`, and documented local directories `/sdk/`, `/rom/`, `/third_party/sdk/`, `/third_party/rom/`. Do not ignore `/**/EPOC32/` globally (too aggressive). Never gitadd proprietary blobs even if a user copies them in.

### 9.6 `.mmp` / `bld.inf` parser specification

Path: `docs/research/mmp-bld-inf-parser.md`. Content outline in §13. **Specification only** — no parser crate.

### 9.7 Experiment backlog

Path: `docs/research/experiment-backlog.md`. Ordered list in §16.

## 10. Runbook outline

The runbook is a procedure, not a script in CI. Every step is either a Verified command fragment, a user-supplied path, or `UNKNOWN — requires experiment`.

### 10.1 Preconditions (human)

- x86_64 Linux preferred. The research prompt’s documented distro is **Ubuntu 24.04 LTS**. If this host is another distro, the runbook says: try the same tool names; record the distro; Docker is the Ubuntu 24.04 fallback after the image exists.
- User has **legal access** to an S60 3rd FP2 SDK. Path is an input, never downloaded by the runbook.
- ROM for EKA2L1 is optional and user-supplied.
- Nokia E52 is optional for §17 accept; required for hardware M0.

Environment variables the runbook uses (and nothing else invented):

| Variable | Meaning |
|---|---|
| `EPOCROOT` | User SDK root. Symbian tools historically want a trailing backslash; whether a trailing `/` works on Linux is **Needs experiment**. |
| `SYMDEV_ROM` | Absolute path to a user-supplied ROM image for EKA2L1. Unset → skip emulator steps. |
| `SYMDEV_EKA2L1` | Absolute path to an EKA2L1 executable the user installed. Unset → skip emulator steps. |

The runbook **never** curls ROM/SDK URLs.

### 10.2 Chapters

1. **Host packages** — whatever Ubuntu 24.04 needs to *build* fedor4ever GCC. Exact package list: **Unknown** until recorded from that project’s docs and a local trial.
2. **GCC** — `arm-none-symbianelf-g++` / `ld` from fedor4ever (GCC 14.2/15.2 mentioned in the research prompt; `linux*` branch exists, no `darwin*` — Verified). Clone URL, commit, and build flags: **Unknown** (copy from that project’s documentation during runbook writing, then experiment). Do not invent a GitHub path if the research prompt did not give one; look it up from primary sources and cite them.
3. **elf2e32** — C++ port (fedor4ever). How to obtain/build on Linux: **Unknown**.
4. **SIS tools** — `makesis`, `signsis`, `makekeys`. Whether native Linux binaries exist or Wine is required: **Needs experiment**.
5. **SDK placement** — user copies SDK to a path outside git; export `EPOCROOT`; confirm `GCCE.h` and `Symbian_OS.hrh` exist. Layout on Linux vs a Windows SDK tree: **Needs experiment**.
6. **Hello sources** — a minimal EXE the human creates by hand (not `symdev new`). The runbook may include a source listing only if it is copied from a Verified example or marked as experiment. Whether a `_reg.rsc` / `rcomp` step is required for the app to appear and launch: **Needs experiment**.
7. **Compile / link / elf2e32** — Verified flags from §4.2. Full argv: **Needs experiment** (paths, crt, `-soname` matching `--linkas`).
8. **`.pkg`** — structure from the Verified template (language `&EN`, name+UID+version `TYPE=SA`, vendor lines, platform UID `0x102752AE`, EXE → `!:\sys\bin\`, optional reg rsc → `!:\private\10003a3f\import\apps\`). Host-side source path in the `.pkg` (`$(EPOCROOT)Epoc32\release\armv5\urel\...`) is Windows-style in the template; Linux translation: **Needs experiment**.
9. **`makekeys` / `makesis` / `signsis`** — Verified:

```
makekeys -cert -expdays 3650 -password <pw> -len 2048 \
  -dname "CN=... OU=... OR=... CO=... EM=..." mykey.key mycert.cer
makesis MyApp.pkg
signsis MyApp.sis MyApp.sisx mycert.cer mykey.key
```

   Password handling: local files, never committed. Exact `-dname` field requirements: **Unknown**.

10. **Print the `.sisx` path** — the north-star first deploy step. The runbook ends by printing the artifact path.
11. **EKA2L1** — see §12; skip if `SYMDEV_ROM` or `SYMDEV_EKA2L1` unset.
12. **Physical E52** — App. Mgr → Settings → **Software installation = All**, **Online certificate check = Off** (Verified sufficient for self-signed user-grantable caps). Delivery: memory card or Bluetooth OBEX (Verified practical paths). `gnokii`/`gammu` cannot install SIS on S60 3rd (Verified). Nokia Suite is Windows-only and EOL (Verified). This chapter is informational until hardware exists.

The runbook does not claim a chapter is done when the tool was not run.

## 11. Docker

Written **after** §10 exists and is believed correct.

- Base: `ubuntu:24.04`.
- Install/build GCC + `elf2e32` + SIS tools using the **same** steps as the runbook. If a runbook step is Unknown, the Dockerfile comments that step as Unknown rather than guessing a `wget`.
- Do not `COPY` an SDK or ROM. Document `docker run` with bind mounts, for example SDK at `/opt/symbian/sdk` and `EPOCROOT` pointing there.
- The image is not required to build a hello in CI. It is an artifact so the runbook can be reproduced on Ubuntu 24.04.
- No EKA2L1 inside the image (GPL process, user-installed; ROM cannot be in the image).

## 12. EKA2L1

EKA2L1 is **GPL-3.0**. Use it as a subprocess. Never vendor its source.

§17 writes `docs/research/eka2l1.md` covering:

- User installs EKA2L1 themselves; set `SYMDEV_EKA2L1`.
- User supplies a ROM they have the right to use (typically dumped from hardware they own); set `SYMDEV_ROM`. Path stays outside git.
- Expected on-disk layout: **Unknown** until observed.
- Headless install + launch + screenshot is **M5**, not §17.
- Exact CLI flags to install a SISX and launch an app: **Unknown — requires experiment**. The appendix must not invent them.

**Skip rule:** if either env var is unset, or the files are missing, the emulator experiment is **skipped** (recorded in `docs/research/`, not a failed cargo test). Skipping does not fail §17 accept. Skipping does not authorize claiming emulator or E52 support.

An emulator success, if it ever happens, still does **not** mean “E52 supported.”

## 13. Parser spec outline

Document: `docs/research/mmp-bld-inf-parser.md`. For **M1**, not implemented now. Fail closed: unknown directives are parse errors.

M1 on this path builds **EXE / ARMV5 / UREL / GCCE** only. Parser output is a structured model for a Rust driver that invokes g++ directly, not Makefile generation.

### 13.1 `.mmp` — support

`TARGET`, `TARGETTYPE`, `UID`, `TARGETPATH`, `SOURCE`, `SOURCEPATH`, `SYSTEMINCLUDE`, `USERINCLUDE`, `LIBRARY`, `STATICLIBRARY`, `CAPABILITY`, `EPOCSTACKSIZE`, `EPOCHEAPSIZE`, `EPOCALLOWDLLDATA`, `START RESOURCE` … `END`.

`TARGETTYPE`: only `EXE` accepted. `UID`: two or three UIDs as in existing MMP practice; mapping to `elf2e32 --uid1/--uid3` is M1 (**Needs experiment** for uid2).

`START RESOURCE` is parsed as a block. Emitting and compiling resources still shells out to legacy `rcomp`/`epocrc` in Wave 0/1; that invocation is not specified here.

### 13.2 `bld.inf` — support

`PRJ_PLATFORMS`, `PRJ_EXPORTS`, `PRJ_MMPFILES`, `PRJ_TESTMMPFILES`, `#if` / `#else` / `#endif`.

`PRJ_PLATFORMS`: we only build ARMV5 UREL GCCE. Extra platforms listed are **ignored**. If the directive is present and does not include a platform we can interpret as that build, error. If omitted, default ARMV5 UREL GCCE. Which tokens (`ARMV5`, `GCCE`, `UREL`) appear in real FP2 `bld.inf` files: **Needs experiment**.

`PRJ_TESTMMPFILES`: parsed so we do not choke; M1 default build does not compile test MMPs unless a future flag says so. §17 does not define that flag.

`#if` syntax is supported. Which macros the SDK defines (`__GCCE__`, etc.): **Needs experiment**. Until then: unimplemented macro in `#if` is a parse error (fail closed), not silent false.

### 13.3 Reject

- `TARGETTYPE` other than `EXE` (`DLL`, `LIB`, `EXEDLL`, `IMPLIB`, …).
- `OPTION`, `OPTION_GCCE`, `MACRO`, `ARMFPU`, `SMPSAFE`, `DEFFILE`, `NOSTRICTDEF`, `VENDORID`, `SECUREID`, `AIF`, `LANG`, `SOURCEEXPORT` — unless a later experiment proves hello-on-E52 requires one of them; then a **new spec** adds it. Do not silently ignore.
- `PRJ_EXTENSIONS`, `PRJ_TESTEXPORTS`.
- `abld` / `makmake` / `Makefile` generation as an output of the parser.
- Directives that exist only for other language or platform tracks.

Line continuation, comments (`//` and `/* */`), and case-insensitivity of directive names: **Likely** but **Needs experiment**. Starting point to record against: directive names case-insensitive ASCII; `#` and `//` comments. Record mismatches.

## 14. Data flow

### 14.1 Today (this host, Wave 0, mostly human)

Developer writes or copies `symdev.toml` (identity only) → later, `BuildBackend` would read sources + manifest → **local** `ExecutionEnvironment` would run Verified tools against `EPOCROOT` → ELF → E32 → `.pkg` (platform UID `0x102752AE`) → `makesis` → `signsis` → `.sisx`.

In **§17** that pipeline is not wired. The CLI validates the manifest (for `build`/`package`/`deploy`) and stops. The human performs compile through sign using the runbook.

`deploy` is not automated. North-star order when it exists: (1) print artifact path, (2) memory-card / mass-storage copy, (3) Bluetooth OBEX. None of these are implemented in §17.

SSH is not on the path until a macOS iteration.

### 14.2 EKA2L1 (optional interim)

Same `.sisx` the human built → later `EmulatorBackend` spawns EKA2L1 with `SYMDEV_ROM`. Missing ROM: skip that experiment only. A skip is not a passed emulator test.

### 14.3 Later (not this spec)

macOS CLI → SSH `ExecutionEnvironment` → same Docker or bare tools on Linux. Native tool crates may shadow legacy binaries (diff, success follows legacy). Other execution environments remain possible without being designed here.

## 15. Error handling

Honesty over green CI. Docs use Verified / Likely / Unknown / Needs experiment. Code and docs must not invent flags or claim E52 or emulator support.

| Class | Behaviour |
|---|---|
| CLI stubs | §8.3 |
| Manifest | Real validation; hard error; no network |
| Missing SDK / `EPOCROOT` | Runbook stops. When M1 exists, build fails with “SDK not configured.” Never fetch. |
| Missing ROM | Skip EKA2L1 experiment; do not fail §17 accept; do not fail cargo test |
| Tool non-zero exit (runbook / later driver) | Fail that step; record argv + stderr in the experiment note |
| Shadow mismatch (later) | Report only; do not fail the build |
| Privileged capabilities | Hard error (no opt-in in this spec) |
| Non-self-signed mode | Hard error |
| Proprietary blobs in git | Out of spec; `.gitignore`; refuse to add |

Do not approximate `symdev doctor` in §17.

## 16. Testing

### 16.1 North-star distinction (not implemented)

**HOST TEST ≠ EMULATOR TEST ≠ DEVICE TEST.** A later test system must not silently downgrade an emulator or device test into a host test. If the requested runtime cannot run the test, it errors. This paragraph is product intent only: no protocol schema, no runner, no `symdev test` command in this spec.

### 16.2 In §17 (`cargo test`, no SDK)

**`symdev-manifest`**

- Accept: `nokia-e52` + `cpp` + omitted or test-range `uid3` + empty caps + default vendor.
- Accept: explicit `uid3` in `0xA…` with self-sign; user-grantable caps listed without duplicates.
- Reject: unknown `target.device`; `language.name` ≠ `cpp`; `uid3` in protected range; privileged or unknown capability; duplicate caps; unknown table; `signing.mode` other than `self-signed`; bad `package.name` / `version`.

**`symdev-cli`**

- `--help` exits 0 and lists `new`, `build`, `package`, `deploy` only.
- Those four subcommands exist.
- `new hello --target nokia-e52` exits 1 with `error: not implemented:` (no files created).
- `new hello --target nokia-e52 --lang java` is a clap error (2).
- `build` with no `symdev.toml` exits 1 `invalid manifest`.
- `build` with a valid fixture (run from a temp dir) exits 1 `not implemented`.
- `build` with an invalid fixture exits 1 `invalid manifest` (not `not implemented`).
- `symdev doctor` / `symdev test` (and the other §8.2 names) are unknown subcommands (clap exit 2), not stubs.

**`symdev-core`**

- The crate compiles, including marker traits and signed traits. `cargo test -p symdev-core` succeeding is enough.

### 16.3 Not tests in §17

Compiling C++, running `elf2e32`, SIS tools, EKA2L1, E52. Those are **experiments** whose pass/fail is written under `docs/research/`, never CI.

### 16.4 CI (when it exists)

`cargo test`, `cargo fmt --check`, `cargo clippy -D warnings` on the stubs. No SDK, no ROM. EKA2L1 in CI is **M5+** and still needs a runner-supplied ROM.

### 16.5 Human accept for §17

- Research notes, runbook, Dockerfile, EKA2L1 appendix, parser spec, experiment backlog exist.
- `cargo test` passes on stubs.
- Emulator chapter is skippable without a ROM.
- Nobody has claimed E52 support.

Hardware accept (not §17): `.sisx` installs and launches on a **stock E52**.

## 17. Experiment backlog (M0 → M1 unblockers)

Each experiment records: procedure, expected result, decision it unblocks, outcome (pass/fail/skip). Skip is valid when a user-supplied input is absent.

Ordered:

1. **SDK layout on this Linux host** — Given a user SDK path, locate `GCCE.h`, `Symbian_OS.hrh`, `armv5/LIB`. Unblocks: runbook `EPOCROOT` chapter, compile include flags. Skip if no SDK.
2. **fedor4ever GCC build on this host / Ubuntu 24.04** — Produce `arm-none-symbianelf-g++` and `ld`. Unblocks: compile/link chapters and Docker. Cite the project’s own docs for commands.
3. **elf2e32 binary on Linux** — Tool runs `--help` or equivalent. Unblocks: post-link chapter.
4. **SIS tools on Linux** — `makekeys`, `makesis`, `signsis` run. Unblocks: packaging chapter. (Wine vs native is the result.)
5. **Hello compile + link without `-fPIC`** — Object + ELF using Verified flags. Unblocks: argv assembly for the runbook. Requires 1–2.
6. **soname / `--linkas` match** — `elf2e32` accepts the ELF. Unblocks: E32 image. Requires 3, 5.
7. **`.pkg` path translation** — `makesis` accepts a `.pkg` whose host paths exist on Linux. Unblocks: SIS. Requires 4, 6.
8. **Self-sign** — `signsis` produces `.sisx`. Unblocks: an artifact a human could copy. Requires 7.
9. **Registration resource** — Does launch-on-phone/emulator require `_reg.rsc` via `rcomp`? Unblocks: M0 hello contents and whether rcomp is on the Wave 0 critical path.
10. **EKA2L1 install + launch** — User ROM + observed CLI. Unblocks: interim de-risk. Skip without ROM. Does **not** unblock “E52 supported.”
11. **Stock E52 install + launch** — Unblocks: claiming device support; this is Hardware M0. Skip without phone.
12. **bld.inf `PRJ_PLATFORMS` tokens and `#if` macros in the FP2 SDK** — Unblocks: M1 parser evaluation rules. Requires SDK.

**M1 coding remains unauthorized** until a later review after a hand-built `.sisx` exists. Preferred proof is experiment 11. If the E52 is still unavailable, a **new design decision** (not this spec) may accept experiments 8+10 as an interim M1 gate. Do not assume that acceptance now.

## 18. Milestones after this spec

| Milestone | What | Authorized by this spec? |
|---|---|---|
| §17 | §9 deliverables | Yes — next implementation plan |
| Hardware M0 | Hand-built `.sisx` on stock E52 | No (human + runbook + hardware) |
| M1 | `symdev build` on this host; driver parses `bld.inf`/`.mmp`; E32 matches hand-built | No |
| M2 | `symdev package` + sign → `.sisx` | No |
| M3 | SSH `ExecutionEnvironment` from macOS | No |
| M4 | `symdev new` templates + deploy automation | No |
| M5 | EKA2L1 in CI | No |

Research-prompt T-track (native tool ports + golden corpus) remains later sequencing once M1+ exists: T1 with M2 (`uidcrc`, `makekeys`, shadow), T2 with M3 (`makesis`/`signsis` + golden), T3 with M4 (`rcomp`/`mifconv`/`bmconv`), T4 post-MVP (`elf2e32` Rust), T5 native macOS GCC. Not authorized now.

When replacing a legacy tool later, normalize E32 timestamp / header CRC / UID checksum / tools version / padding and SIS creation time / signatures / checksums / 4-byte padding before byte compare (Verified). Not this cycle.

## 19. Licensing and repo hygiene

- Repo license: **undecided**. Do not add `LICENSE` in this cycle.
- Never commit SDK, WTK, ROM, certificates, or private keys.
- EKA2L1: process only.
- Original `elf2e32` is EPL-1.0; `rcomp`/`bmconv`/`petran`/`uidcrc` use the Symbian Example Source Code License (Verified). Any future C++ ports live in **separate modules/submodules** with notices intact — not in §17.
- Future Rust reimplementations are **clean-room** from format specs and golden behaviour.

## 20. Open unknowns (intentionally documented)

These are gaps, not unfinished spec sections. They do not block writing §17 docs/stubs. They do block claiming a working toolchain.

- Exact fedor4ever repository location, commit, and GCC build/install commands on Ubuntu 24.04.
- How to obtain/build `elf2e32` and SIS tools on Linux; Wine vs native.
- Linux `EPOCROOT` trailing-separator and whether a Windows SDK tree works without path rewriting.
- Full compile/link argv (crt objects, `-L` paths, `-soname`).
- `.pkg` host-path syntax on Linux.
- `makekeys -dname` mandatory fields.
- Whether M0 hello needs `rcomp` and `_reg.rsc` to launch.
- `rcomp`/`epocrc` availability on Linux.
- EKA2L1 CLI for install/launch; ROM file layout.
- `bld.inf` platform tokens and live `#if` macros in the FP2 SDK.
- `UID` directive ↔ `elf2e32` uid1/uid2/uid3 mapping beyond uid1 `0x1000007a` for EXE (Verified) and uid3 from the manifest.
- Distro differences if this edit/build host is not Ubuntu 24.04.
- Mapping `gcce-14` / `gcce-15` to actual binary names and GCC 14.2 vs 15.2.

Likely but not Verified (do not code against them as facts): MMP comment syntax and case-insensitivity; Docker bind-mount plus `EPOCROOT` being sufficient for the same argv as bare metal.

## 21. Scope of the next implementation plan

One plan, covering only §9. Success is the human accept in §16.5. The plan must not include M1 driver work, extra clap commands, extra crates, Java ME, SSH, a LICENSE file, or network acquisition of SDK/ROM.
