# M0 bare-metal runbook

Procedure for a human on this Linux host. Not a CI script. Every step is a **Verified** command fragment, a user-supplied path, or `UNKNOWN — requires experiment`.

This file is **believed correct**: internally consistent with spec §4.2 / §10 and the Task 1 notes. It is **not** an empirical green build. No chapter below was executed for this document. Do not claim a chapter complete until that tool has been run and the outcome recorded.

Cites (do not contradict):

- Spec: `docs/superpowers/specs/2026-09-16-symdev-m0-north-star-design.md` §4.2, §10, §12
- [pipeline-and-tools.md](pipeline-and-tools.md)
- [uids-capabilities-signing.md](uids-capabilities-signing.md)
- [licensing.md](licensing.md)

Bypass `abld` / `makmake` on this path (Verified). Pipeline (Verified):

```
arm-none-symbianelf-g++  →  arm-none-symbianelf-ld  →  elf2e32  →  makesis  →  signsis
```

Never `curl` SDK or ROM URLs. Never commit SDK, ROM, certificates, or private keys ([licensing.md](licensing.md)).

Env vars only (nothing else invented):

| Variable | Meaning |
|---|---|
| `EPOCROOT` | User SDK root. Symbian tools historically want a trailing backslash; whether a trailing `/` works on Linux is **Needs experiment**. |
| `SYMDEV_ROM` | Absolute path to a user-supplied ROM image for EKA2L1. Unset → skip emulator steps. |
| `SYMDEV_EKA2L1` | Absolute path to an EKA2L1 executable the user installed. Unset → skip emulator steps. |

Verified fragments use basename `hello` for compile / `elf2e32` and `MyApp` for SIS tools. Pick one basename and keep it consistent; neither has been built here.

---

## Preconditions

- **This host (recorded):** Ubuntu 26.04.1 LTS (`resolute`), x86_64. Spec / research prompt prefer **Ubuntu 24.04 LTS**. Try the same tool names; record any distro-specific failure. After the image exists, **Docker Ubuntu 24.04** is the fallback (spec §10.1 / §11).
- User has **legal access** to an S60 3rd Edition Feature Pack 2 SDK. Path is an input, never downloaded.
- ROM for EKA2L1 is optional and user-supplied.
- Nokia E52 is optional for §17 accept; required before anyone may claim E52 support.

---

## 1. Host packages

**Status:** not executed on this host.

Whatever Ubuntu 24.04 needs to *build* fedor4ever GCC. Exact package list: **Unknown** until recorded from that project’s docs and a local trial ([pipeline-and-tools.md](pipeline-and-tools.md)).

```
UNKNOWN — requires experiment
```

On this Ubuntu 26.04 host: try the same package names the GCC project documents for Ubuntu 24.04; record what differs. Do not invent an `apt` list here.

---

## 2. GCC

**Status:** not executed on this host.

Need `arm-none-symbianelf-g++` and `arm-none-symbianelf-ld` from fedor4ever. GCC 14.2 / 15.2 are mentioned in the research prompt; a `linux*` branch exists; no `darwin*` branch (Verified). Mapping `gcce-14` / `gcce-15` to binary names and 14.2 vs 15.2: **Unknown** (spec §20).

Plan-time sources ([pipeline-and-tools.md](pipeline-and-tools.md)):

- Project: [https://github.com/fedor4ever/GCC4Symbian](https://github.com/fedor4ever/GCC4Symbian) (`build-toolchain.sh`; README warns incompatible with `abld` — matches this path’s bypass).
- Prebuilt GCC 14.2.0 + binutils 2.29.1: [https://sourceforge.net/projects/gcce4symbian/files/GCC-14.2.0_BINUTILS-2.29.1/](https://sourceforge.net/projects/gcce4symbian/files/GCC-14.2.0_BINUTILS-2.29.1/)
- Blog: [https://fedor4ever.wordpress.com/2024/09/22/gcc-14-1-0-for-symbian-out/](https://fedor4ever.wordpress.com/2024/09/22/gcc-14-1-0-for-symbian-out/)

Upstream README (fetched as that project’s documentation during runbook writing): goal is building Binutils, GCC, and GDB; **Warning**: incompatible with the `abld` build system; tested on S60_5th_Edition_SDK_v1.0 (not S60 3rd FP2); details at https://fedor4ever.wordpress.com/. The README does not give clone commit or build flags.

Exact clone URL usage, commit, `linux*` branch name, `build-toolchain.sh` argv, and whether to build from source vs the SourceForge tarball:

```
UNKNOWN — requires experiment
```

Do not invent `git clone` / `wget` argv. Copy commands from that project’s docs during the experiment, then record them. Never curl SDK/ROM.

---

## 3. elf2e32

**Status:** experiment 6 executed on this host (`pass`). Linux `elf2e32_next` binary produced `hello.exe`. Recorded argv: [experiment-backlog.md](experiment-backlog.md) experiment 6.

C++ port (fedor4ever). Plan-time sources ([pipeline-and-tools.md](pipeline-and-tools.md)):

- [https://github.com/fedor4ever/elf2e32_next](https://github.com/fedor4ever/elf2e32_next) (tested S60_3rd_FP2_SDK_v1.1)
- Older C++14 port: [https://github.com/fedor4ever/elf2e32](https://github.com/fedor4ever/elf2e32)

How to obtain/build on Linux: recorded in experiment 3 (`elf2e32_next` master `6a1d311`; binary `$HOME/src/elf2e32_next/bin/Release/elf2e32`). Original `elf2e32` is EPL-1.0 ([licensing.md](licensing.md)). Do not vendor it into this tree in §17.

Invocation fragment (Verified, spec §4.2) — used in chapter 7, not invented here:

```
elf2e32 --uid1=0x1000007a --uid3=<UID3> --capability=<caps> --fpu=softvfp
        --targettype=EXE --output=hello.exe --elfinput=hello.elf
        --linkas=hello{000a0000}[<UID3>].exe --libpath=<SDK>/armv5/LIB
```

---

## 4. SIS tools

**Status:** not executed on this host.

Need `makesis`, `signsis`, `makekeys`. Whether native Linux binaries exist or Wine is required: **Needs experiment** ([pipeline-and-tools.md](pipeline-and-tools.md)).

```
UNKNOWN — requires experiment
```

Do not invent Wine argv. Record which binaries ran, from where (SDK tree vs Wine), and the working names. Signing is **self-sign only**; Symbian Signed is closed (Verified).

---

## 5. SDK placement

**Status:** not executed on this host.

User copies an S60 3rd FP2 SDK they have legal access to, **outside git**. Export `EPOCROOT` to that root. Never curl the SDK.

Confirm these exist (names from Verified compile flags):

- `GCCE.h` (used as `-include <SDK>/GCCE.h`)
- `Symbian_OS.hrh` (used as `-D__PRODUCT_INCLUDE__="<SDK>/Symbian_OS.hrh"`)

Also needed later: `armv5/LIB` for `elf2e32 --libpath=` (Verified fragment). Exact on-disk layout of a Windows SDK tree on Linux: **Needs experiment**. Trailing `\` vs `/` on `EPOCROOT`: **Needs experiment**.

```
UNKNOWN — requires experiment
```

Skip this chapter’s confirmations if the user has no SDK; do not download one.

---

## 6. Hello sources

**Status:** not executed on this host.

A minimal EXE the human creates **by hand** (not `symdev new`; that command must not create files in §17). No Verified hello listing exists in spec §4.2 or the Task 1 notes. Do not invent a full app here.

```
UNKNOWN — requires experiment
```

UID / caps constraints while writing sources ([uids-capabilities-signing.md](uids-capabilities-signing.md), Verified):

- Default / hello UID3: test range **0xE0000000–0xEFFFFFFF**
- Self-signed range **0xA0000000–0xAFFFFFFF** only if the user sets `uid3` explicitly
- Protected **< 0x80000000** is a hard error under self-sign
- Self-signable capabilities, exactly six: `LocalServices`, `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location`. Anything else is refused in this product phase.

Whether a `_reg.rsc` / `rcomp` step is required for the app to appear and launch: **unknown until experiments 10/11** (experiment 9: `rcomp` runs via Wine; a `_reg.rsc` can be produced from a copied SDK example `.rss`; launch not proven). Optional `.pkg` dest if a reg resource exists: `!:\private\10003a3f\import\apps\` (Verified template). `rcomp` on Linux via Wine: **recorded** (experiment 9). Host `epocrc.pl` did not run (FindBin `\` rewrite).

---

## 7. Compile / link / elf2e32

**Status:** experiments 5–6 executed on this host (`pass`). Compile produced `hello.o`. Link with GNU ld 2.29.1 produced `hello.elf` (same argv; experiment-2 ld 2.35 fails on SDK `euser.dso`). `elf2e32` accepted that ELF (`--linkas` matched linker `-soname`) and wrote `hello.exe`. Recorded argv: [experiment-backlog.md](experiment-backlog.md) experiments 5 and 6.

Assemble argv from Verified fragments plus experimentally recorded paths only. Full argv (include/lib search paths, C runtime objects, flag order, `-soname` matching `--linkas`): compile + link argv recorded; ELF produced with GNU ld 2.29.1; E32 produced with the Verified `elf2e32` fragment.

Never pass `-fPIC` or `-fPIE` (Verified: causes “Import relocation does not refer to code segment”).

**Compile flags** (Verified against fedor4ever’s manual-build write-up):

```
-O2 -fexceptions -march=armv5t -mapcs -mthumb-interwork -mthumb -msoft-float
-D__SYMBIAN32__ -D__EPOC32__ -D__MARM__ -D__GCCE__ -D__EXE__
-include <SDK>/GCCE.h
-D__PRODUCT_INCLUDE__="<SDK>/Symbian_OS.hrh"
```

Optional size flags (Verified as a size-reduction technique, **not** required for a first hello): `-ffunction-sections -fdata-sections` plus linker `--gc-sections --strip-discarded`.

**Link flags** (Verified):

```
--target1-abs --no-undefined -nostdlib -shared
-Ttext 0x8000 -Tdata 0x400000 --strip-debug
--entry _E32Startup -u _E32Startup
```

Link against: `eexe.lib`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, `drtaeabi.dso`, `scppnwdl.dso`, plus `-lsupc++ -lgcc`.

soname / `--linkas` format is `<name>{version}[uid3].<ext>` and **must match** between the linker `-soname` and `elf2e32 --linkas` (Verified).

**elf2e32** (Verified fragment):

```
elf2e32 --uid1=0x1000007a --uid3=<UID3> --capability=<caps> --fpu=softvfp
        --targettype=EXE --output=hello.exe --elfinput=hello.elf
        --linkas=hello{000a0000}[<UID3>].exe --libpath=<SDK>/armv5/LIB
```

Replace `<SDK>`, `<UID3>`, and `<caps>` from the user SDK and chapter 6. Non-empty `--capability=` for the Verified user-grantable six was recorded in experiment 6. How empty capabilities are spelled for `--capability=`: **Unknown**.

Fail the step on non-zero exit; record argv + stderr in an experiment note (spec §15).

---

## 8. `.pkg`

**Status:** experiment 7 executed on this host (`pass`). `makesis` (Wine) accepted a `.pkg` whose host path exists here and wrote `hello.sis`. Recorded syntax: [experiment-backlog.md](experiment-backlog.md) experiment 7.

Structure from the Verified template (spec §10.2). `.pkg` **must** include platform dependency **0x102752AE**. Omitting it produces “App is incompatible with phone” (Verified). EXE dest **must** include `!:\sys\bin\`.

Verified tokens (not a complete validated file):

- language `&EN`
- name + UID + version with `TYPE=SA`
- vendor lines
- platform UID `0x102752AE`
- EXE → `!:\sys\bin\`
- optional reg rsc → `!:\private\10003a3f\import\apps\`
- host-side source path in the template: `$(EPOCROOT)Epoc32\release\armv5\urel\...` (Windows-style)

Host-path translation, **recorded from experiment 7** (copied from SDK `locationsatviewrefapp_armv5.pkg`; only name / UID3 / host path substituted): **relative filename** next to the `.pkg` (`"hello.exe"`), not Unix absolute and not Wine `Z:\…` (those dialects were not tried). Optional `_reg.rsc` mapping was omitted from the experiment-8 SISX (no such host file then). Experiment 9 later produced example `_reg.rsc` files via Wine `cpp.exe`+`rcomp.exe`; they were **not** packed into `hello.sisx`. Launch with vs without remains experiments 10/11.

Exact `.pkg` line syntax, **recorded from experiment 7** (CRLF; comments omitted here):

```
&EN
#{"hello"},(0xe79e4cf9),1,0,24,TYPE=SA
%{"Vendor-EN"}
:"Vendor"
[0x102752AE], 0, 0, 0, {"S60ProductID"}
"hello.exe"		-"!:\sys\bin\hello.exe"
```

Do not invent extra `.pkg` keys. `makesis` accepting a `.pkg` whose host paths exist on Linux is experiment 7 in spec §17.

---

## 9. `makekeys` / `makesis` / `signsis`

**Status:** experiment 8 executed on this host (`pass`). Wine `makekeys` then `signsis` wrote `hello.sisx`. Recorded argv: [experiment-backlog.md](experiment-backlog.md) experiment 8. `makesis` recorded in chapter 8 / experiment 7.

Self-sign only (Verified). `makekeys -expdays 3650` requires Symbian 9.2+ tools (Verified). Without `-expdays` a certificate lasts about one year. Password handling: local files, never committed.

Verified block (spec §4.2 / §10.2):

```
makekeys -cert -expdays 3650 -password <pw> -len 2048 \
  -dname "CN=... OU=... OR=... CO=... EM=..." mykey.key mycert.cer
makesis MyApp.pkg
signsis MyApp.sis MyApp.sisx mycert.cer mykey.key
```

**Observed `-dname` fields** (copied from this SDK `makekeys` no-arg usage, not from the spec placeholders `OR`/`CO`):

- `CN` Common Name, e.g. `CN=Joe Bloggs`
- `C` Country, e.g. `C=GB`
- `O` Organisation, e.g. `O=Acme Ltd`
- `OU` Organisational Unit, e.g. `OU=Development`
- `EM` E-Mail address, e.g. `EM=noone@nowhere.com`

Usage: `-dname` is required on `-cert`/`-req`; “A distinguished name strings needs at least two attributes.” Experiment 8 used the tool’s own Example Usage string (`CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com`); a two-attribute minimum was not separately trialed. Local password (`-password`, ≥4 characters) was used and passed as the `signsis` passphrase. Wine (same PE tools as experiment 4). Never commit `*.cer` / `*.key`.

---

## 10. Print the `.sisx` path

**Status:** experiment 8 executed on this host (`pass`). Absolute path produced: `/home/genius/src/symdev-experiment-5/hello.sisx`.

North-star first deploy step. After `signsis` succeeds, print the artifact path (Verified fragment name `MyApp.sisx`):

```
echo "$(pwd)/MyApp.sisx"
```

If the working directory is not where `signsis` wrote the file, print the absolute path actually produced. This is the hand-off for memory-card or Bluetooth copy (chapter 12). Emulator / hardware chapters below are optional and do not replace this print.

---

## 11. EKA2L1

**Status:** experiment 10 skipped on this host (`SYMDEV_EKA2L1` and `SYMDEV_ROM` unset; no EKA2L1 binary or ROM on common paths). Evidence: [experiment-backlog.md](experiment-backlog.md) experiment 10.

See [eka2l1.md](eka2l1.md) (Task 3). EKA2L1 is GPL-3.0: invoke as a **separate process** only; do not copy its source into this tree ([licensing.md](licensing.md)).

**Skip** if `SYMDEV_ROM` or `SYMDEV_EKA2L1` is unset, or if those paths are missing. Skipping is valid; it does not fail §17 accept; it does not authorize claiming emulator or E52 support (spec §12).

Expected on-disk layout and exact CLI flags to install a SISX and launch: still **Unknown** (not observed).

```
UNKNOWN — requires experiment
```

An emulator success, if it ever happens, still does **not** mean “E52 supported.”

---

## 12. Physical E52

**Status:** experiment 11 skipped. No Nokia E52 provided. Evidence: [experiment-backlog.md](experiment-backlog.md) experiment 11. Not executed.

Intended first device: Nokia E52 (RM-469, “Stella”), Symbian OS 9.3 / EKA2, S60 3rd Edition Feature Pack 2 (Verified). Hardware M0 = the hand-built `.sisx` installs and the app launches on a **stock** E52. Nobody may claim E52 support before that.

On the phone: App. Mgr → Settings → **Software installation = All**, **Online certificate check = Off** (Verified sufficient for self-signed user-grantable caps).

Delivery: memory card or Bluetooth OBEX (Verified practical paths). `gnokii` / `gammu` cannot install SIS on S60 3rd (Verified). Nokia Suite is Windows-only and EOL (Verified). Do not use gnokii or gammu.

This chapter does not run until an E52 is present. Skip without a phone.
