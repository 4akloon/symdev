# Experiment backlog (M0 → M1 unblockers)

Ordered experiments **1–12** from spec §17. Each records: procedure, expected result, decision unblocked, outcome (`pass` / `fail` / `skip`).

**Skip is valid** when a user-supplied input is absent (SDK, ROM, EKA2L1 binary, phone). Skipping is recorded here, not as a failed `cargo test`. Skipping does not fail §17 accept. Skipping does not authorize claiming emulator or E52 support.

Experiments 1–9 and 12 have been run (`pass`). Experiments 10–11 are `skip` (no EKA2L1/ROM, no Nokia E52). Do not treat this file as evidence of a green toolchain. Skipping 10/11 does **not** authorize claiming emulator or E52 support.

Pass/fail lives in this file (or a linked note under `docs/research/`), **never CI** (spec §16.3).

Cites: spec §17 / §16.3 / §18; [m0-bare-metal-runbook.md](m0-bare-metal-runbook.md); [pipeline-and-tools.md](pipeline-and-tools.md); [eka2l1.md](eka2l1.md); [mmp-bld-inf-parser.md](mmp-bld-inf-parser.md); [uids-capabilities-signing.md](uids-capabilities-signing.md); [licensing.md](licensing.md).

## M1 gate (2026-09-17)

Hand-built `.sisx` exists (experiment 8): `$HOME/src/symdev-experiment-5/hello.sisx` (outside git). Completing the backlog file itself never lifted the gate.

Preferred proof was **experiment 11**. The spec’s fallback interim was experiments **8+10**. Experiment 11 remains `skip` (user: no phone). Experiment 10 remains `skip` (no EKA2L1/ROM).

**Design decision (user, 2026-09-17):** proceed to M1 (`symdev build`) without hardware or emulator proof. This does **not** authorize claiming E52 support. Experiment 11 stays skip until a stock E52 is available.

## Parked (user 2026-09-18): not this host’s priority

This host is Linux. Do **not** pull these ahead of T2 (native SIS on recorded goldens) or later Linux T-track (T3 `rcomp`/`mifconv`/`bmconv` when unblocked, T4 `elf2e32` Rust). They stay in the backlog, not the next slice.

| Item | Why parked |
|---|---|
| **T5** native macOS GCC | This host is Linux. North-star T5 waits for a macOS edit host. |
| **M3** SSH `ExecutionEnvironment` | Needs macOS as the edit host (already skipped on M4). |
| **M5** EKA2L1 in CI | Experiment 10 `skip` (`SYMDEV_EKA2L1` / `SYMDEV_ROM` unset). Do not invent emulator flags. |
| **E52 / Hardware M0** | Experiment 11 `skip` (no stock phone). Skip does not authorize claiming E52 support. |

Experiments 10–11 remain `skip`. Re-open only when the missing host, ROM, or phone is actually present.

## How to record

After a run, set **Outcome** to exactly `pass`, `fail`, or `skip`. Add date, host, and a short evidence note (paths observed, argv actually used, stderr). Do not invent argv here before the run: copy Verified fragments from [pipeline-and-tools.md](pipeline-and-tools.md) / the runbook, or copy commands from the cited project’s own docs during the run, then record what ran.

Never curl SDK or ROM URLs. Never commit SDK, ROM, certificates, or private keys ([licensing.md](licensing.md)).

This host (runbook): Ubuntu 26.04.1 LTS, x86_64. Spec / research prompt prefer Ubuntu 24.04 LTS. Try the same tool names; record distro-specific failure. Docker Ubuntu 24.04 is the fallback after that image exists.

---

## 1. SDK layout on this Linux host

- **Requires:** none (user SDK path is an input)
- **Skip if:** no SDK (user has no legal-access S60 3rd FP2 SDK path)
- **Procedure:** Given a user SDK path (`EPOCROOT`; never downloaded), locate `GCCE.h`, `Symbian_OS.hrh`, and `armv5/LIB`. Record the relative paths that exist. Do not rewrite a Windows SDK tree until this experiment says to. Trailing `\` vs `/` on `EPOCROOT`: record what was tried.
- **Expected result:** Those three names exist under the user SDK. Recorded paths are enough to fill runbook include / `--libpath=` slots.
- **Decision unblocked:** runbook `EPOCROOT` chapter, compile include flags
- **Outcome:** pass
- **Evidence:** 2026-09-17 (rerun), Ubuntu 26.04.1 LTS x86_64. User archive `/home/genius/Downloads/S60_3rd_Edition_SDK_Feature_Pack_2_v1_1_en.zip` (477091187 bytes). Wrapper `/home/genius/sdk/S60_3rd_Edition_FP2_v1.1_installer/` (InstallShield 12, `ISc(` CABs). No sudo. Method 1: official 7-Zip 26.03 `7zz` → `$HOME/.local/bin/7zz`; `7zz l` failed on `data1.cab`/`data1.hdr` (not MS-CAB); `data2.cab` misread as a small zip. GitHub `twogood/unshield` 1.6.2 has no release binaries; used Ubuntu pool debs `unshield_1.6.2-2_amd64.deb` + `libunshield1_1.6.2-2_amd64.deb` extracted to `$HOME/.local/` (wrapper `$HOME/.local/bin/unshield` → `$HOME/.local/lib/unshield.bin` + `libunshield.so.1.6.2`). Method 2: cloned `twogood/unshield` @ `1.6.2` (`51de441`); compiled C sources with host gcc-15 (no `make`/`cmake`); zlib headers from `zlib1g-dev` deb, linked `/usr/lib/x86_64-linux-gnu/libz.so.1` → `$HOME/.local/bin/unshield-from-src` (`unshield -V` 1.6.2). Extract: `unshield -L -d $HOME/sdk/S60_3rd_FP2 -g CPP_API|CPP_Toolchain|CPP_SymbianToolchain|Common_toolchain x data1.cab` then copied those group `epoc32` trees into one **EPOCROOT** (outside git). Trailing `EPOCROOT` `\` not tried; Linux `/` used. No exact `Symbian_OS.hrh` / `GCCE.h` (installer names are lowercase). Recorded files:
  - `EPOCROOT=/home/genius/sdk/S60_3rd_FP2/` (unified `epoc32` 229M)
  - `gcce.h`: `/home/genius/sdk/S60_3rd_FP2/epoc32/include/gcce/gcce.h` (3894 bytes; C text, CRLF)
  - OS HRH: `/home/genius/sdk/S60_3rd_FP2/epoc32/include/variant/symbian_os_v9.3.hrh` (34127 bytes)
  - `armv5/lib`: `/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/lib` (1140 entries, incl. `euser.lib` / `estlib.lib`)
  - `elf2e32.exe`: `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/elf2e32.exe` (679936; PE32)
  - `makesis.exe`: `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/makesis.exe` (733184)
  - `signsis.exe`: `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/signsis.exe` (1241088)
  Emulator/docs/examples groups not unpacked. Tools are Windows PE (Wine still unset; experiment 4).

## 2. fedor4ever GCC build on this host / Ubuntu 24.04

- **Requires:** none (host packages are whatever that project’s docs name)
- **Skip if:** not applicable as a missing user blob; if the experiment cannot start, record `fail` and why, do not invent a tarball URL
- **Procedure:** Produce `arm-none-symbianelf-g++` and `ld` (`arm-none-symbianelf-ld`). **Cite the project’s own docs for commands** ([pipeline-and-tools.md](pipeline-and-tools.md) plan-time sources: GCC4Symbian, SourceForge GCC 14.2.0 + binutils 2.29.1, fedor4ever blog). Do not invent clone commit, `build-toolchain.sh` argv, or apt lists here. Record source-tree vs tarball and 14.2 vs 15.2 as observed.
- **Expected result:** `arm-none-symbianelf-g++` and `arm-none-symbianelf-ld` exist on this host (or on Ubuntu 24.04 if this distro fails) and identify as those tools.
- **Decision unblocked:** compile/link chapters and Docker
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. **Source tree, not SourceForge tarball.** Cloned `fedor4ever/GCC4Symbian` master `fe1b15a` (`$HOME/src/GCC4Symbian`); remote heads: `master` only. Script as committed: `GCCC=gcc-12.1.0`, `BINUTILS=binutils-2.35`, `GDB=gdb-10.2`, `TARGET=arm-none-symbianelf`, Linux `PREFIX=$HOME/gcc-builds/$GCCC`, `MAKEJOBS=-j6`. Ran exactly `./build-toolchain.sh` (no extra argv) from that repo; log `$HOME/src/GCC4Symbian/experiment-2-build.log`. **14.2 vs 15.2 vs what built:** SourceForge [GCCE4Symbian files](https://sourceforge.net/projects/gcce4symbian/files/) + `Readme.md` are **Windows prebuilts** (`Supported OS: WinXP and newer`; `gcc-14.2.0_win32.zip`, latest `gcc-15.2.0_win32.zip` under `GCC-15.2.0_BINUTILS-2.29.1`; binutils **2.29.1**). Blog 2024-09-22: “Grab windows build… For other os – use script to build.” Cloned tree has no 14.2/15.2 script; did not retarget. **Observed product: GCC 12.1.0 + binutils 2.35.** Host C++ (no sudo): Ubuntu resolute debs extracted into existing `$HOME/.local/native-cc` (gcc-15 prefix) — `make_4.4.1-3`, `g++-15`/`g++-15-x86-64-linux-gnu`/`libstdc++-15-dev` `15.2.0-16ubuntu1`, `flex_2.6.4-8.2build2`, `bison_3.8.2+dfsg-1build4`, `m4_1.4.21-1`, plus `gawk`/`zlib1g-dev`/`libfl2`; wrappers `$HOME/.local/bin/{make,g++,gcc,flex,bison,m4}`. Experiment 1 `gcce.h` / HRH / `armv5/lib` still present (`test -f`). gcc `make -k` as in script; `install-gcc.log`: `crtfastmath.o` missing (matches blog). gdb `make install` failed (`cd: ./bfd: No such file`; no `arm-none-symbianelf-gdb`) — not required here. Recorded binaries:
  - `arm-none-symbianelf-g++`: `/home/genius/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-g++` (1580328; ELF 64-bit LSB x86-64, stripped; `--version`: `arm-none-symbianelf-g++ (GCC) 12.1.0`)
  - `arm-none-symbianelf-ld`: `/home/genius/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-ld` (2009808; ELF 64-bit LSB pie x86-64, stripped; `--version`: `GNU ld (GNU Binutils) 2.35`)

## 3. elf2e32 binary on Linux

- **Requires:** none
- **Skip if:** not applicable as a missing user blob; obtain/build path is Unknown until recorded ([pipeline-and-tools.md](pipeline-and-tools.md))
- **Procedure:** Get an `elf2e32` that runs on Linux (plan-time sources: `elf2e32_next`, older C++14 port — do not invent build argv). Run `--help` or equivalent. Do not vendor original `elf2e32` into this tree in §17 ([licensing.md](licensing.md)).
- **Expected result:** The tool starts and prints help or equivalent usage. Record which binary and how it was obtained.
- **Decision unblocked:** post-link chapter
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Cloned `fedor4ever/elf2e32_next` master `6a1d311` (`$HOME/src/elf2e32_next`; outside git). **No Makefile/CMake.** README Build instruction: “Code block users can import and build” / “Other - need C++14 compiler and pass -D__EABI__”. Code::Blocks not installed; followed **Other** using flags from `elf2e32_next.cbp` (global `-D__EABI__` `-std=c++14` `-fexceptions` `-fpermissive` `-Iinclude -Isrc -Ilib/elf -Ilib/e32 -Ilib/getopt`; Release `-O2`; linker `-s -static-libstdc++ -static-libgcc`; output `bin/Release/elf2e32`). Host `g++` 15.2.0 (`$HOME/.local/bin/g++`). First `g++` of cbp Units failed: GCC 15 no longer leaks `uint32_t` (headers such as `huffman.h` / `getopt.hpp` use it without `#include`; `struct Opts` then appears to lack `binary_arg1`). Retry with host workaround `-include stdint.h` (not in project files). cbp Unit `lib/e32/deflate_manger.cpp` absent from tree — omitted. cbp `-static` not used on the successful link (dynamic `libc`). Log `$HOME/src/elf2e32_next/experiment-3-build.log`. `--help` is the project’s own option (`cmdlineprocessor.cpp` “--help: This command.”). SDK `elf2e32.exe` / Wine not used. Recorded binary:
  - `elf2e32`: `/home/genius/src/elf2e32_next/bin/Release/elf2e32` (1538424; ELF 64-bit LSB pie x86-64, stripped, dynamically linked; `--help` / no-args: `Symbian Post Linker, Elf2E32 v 3.0 (Build 2)` then usage; exit 0). Matches `include/elf2e32_version.hpp` (`iMajor=3`, `iMinor=0`, `iBuild=2`). `--version` is a required-argument module-version option, not a tool-version switch — not run.

## 4. SIS tools on Linux

- **Requires:** none (SDK tools vs Wine is the result)
- **Skip if:** no way to invoke the tools without curling proprietary blobs; then `skip` or `fail` with the reason — do not download an SDK to “make it pass”
- **Procedure:** Run `makekeys`, `makesis`, and `signsis` (help or equivalent is enough for this experiment). **Wine vs native is the result.** Do not invent Wine argv. Record which binaries ran and from where.
- **Expected result:** All three names run. Recorded: native Linux, Wine, or mixed.
- **Decision unblocked:** packaging chapter
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. **Wine, not native.** No Linux `makesis`/`signsis`/`makekeys` on PATH; SDK copies are PE32 i386 under `EPOCROOT` (`makesis.exe` 733184, `signsis.exe` 1241088, `makekeys.exe` 835584). User installed distro Wine: `sudo apt-get install -y wine` → `ii wine` / `wine64` / `wine-common` / `libwine` `10.0~repack-12ubuntu1`. Loader `/usr/bin/wine` (alternatives → `wine-stable`). Wine’s own help: `wine --help` prints `Usage: wine PROGRAM [ARGUMENTS...]` / `wine --help` / `wine --version` (exit 0); `wine --version` → `wine-10.0 (Ubuntu 10.0~repack-12ubuntu1)`. Tool flags from the PE strings / printed help, not invented: MakeSIS `-h`; SignSIS `-h` or `-?`; MakeKeys no-arg usage. Ran Wine-documented Unix paths (first run created `$HOME/.wine`). stderr on each: `experimental wow64 mode` (Debian wine 10 runs 32-bit PE via wine64). No `.pkg` / signing. Recorded:
  - `wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/makesis.exe -h` — exit 0. `MAKESIS Version 4, 0, 0, 10` then `MakeSIS PKG File format help` (`&aa[(dddd)]`, `#{"NAMEaa"...}`, vendor/`IF` tokens).
  - `wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/signsis.exe -h` — exit 4. `Please name the input file.` then `Usage: SignSIS [-?] [-c...] [-i] [-o[-p]] [-s] [-u] [-v] input [output [certificate key [passphrase] ] ]` (`-? or -h` Output this information).
  - `wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/makekeys.exe` — exit 1. `MakeKeys, version 1.0` then `Usage:` `-cert` / `-req` / `-view` (includes `-expdays` example; `-dname` fields listed, not used here).

## 5. Hello compile + link without `-fPIC`

- **Requires:** experiments 1–2
- **Skip if:** experiment 1 skipped (no SDK) or 2 not passed
- **Procedure:** Compile an object and link an ELF using **Verified** flags from [pipeline-and-tools.md](pipeline-and-tools.md) / runbook chapter 7. **Never** pass `-fPIC` or `-fPIE`. Full argv (crt, `-L`, flag order) is Unknown until this run records it. Fail on non-zero exit; keep argv + stderr.
- **Expected result:** Object + ELF produced. Recorded argv is what the runbook may later assemble (Verified fragments plus these paths only).
- **Decision unblocked:** argv assembly for the runbook
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-5/` (outside git). Hello sources copied from fedor4ever [How to build Symbian app by hand](https://fedor4ever.wordpress.com/2026/03/06/how-to-build-symbian-app-by-hand/) (public-domain Carbide listing; GCC4Symbian README points at this blog). Compile/link argv = Verified fragments (pipeline-and-tools / runbook ch.7) plus that write-up’s `-c` `-nostdinc` `-I` `-L` `-l:` `--default-symver` `-soname` / library order, with experiment-1/2 path substitution. Never `-fPIC`/`-fPIE`. Omitted write-up `-D__S60_50__` (this host SDK is 3rd FP2, not 5th). UID3 `0xe79e4cf9` from the write-up (test range). **Compile exit 0.** Object `/home/genius/src/symdev-experiment-5/hello.o` (3604 bytes). Warnings only (`operator new` dllimport / `TSecureId` temporary). First **link exit 1. No ELF** with experiment-2 GNU ld **2.35**: stderr `euser.dso: .gnu.version_d invalid entry` then `error adding symbols: bad value` (same failure documented in [Testing results for GCC 15.2.0](https://fedor4ever.wordpress.com/2025/09/26/testing-results-for-gcc-15-2-0/); no argv workaround). Retry: built GNU binutils **2.29.1** for `TARGET=arm-none-symbianelf` from `https://gcc.gnu.org/pub/binutils/releases/binutils-2.29.1.tar.bz2` (GCC4Symbian `wget` path; not SDK, not SourceForge Windows zip). Configure flags copied from `$HOME/src/GCC4Symbian/build-toolchain.sh` binutils pass; `PREFIX=$HOME/gcc-builds/binutils-2.29.1` (did not overwrite gcc-12.1.0 / ld 2.35). Host wrappers `$HOME/.local/bin`. `/home/genius/gcc-builds/binutils-2.29.1/bin/arm-none-symbianelf-ld --version`: `GNU ld (GNU Binutils) 2.29.1`. Relinked existing `hello.o` with the recorded link argv, only the `ld` binary path replaced. **Link exit 0.** ELF `/home/genius/src/symdev-experiment-5/hello.elf` (24244 bytes; ELF 32-bit LSB shared object, ARM, EABI4). Map `hello.exe.map` (24771 bytes). Log `$HOME/src/symdev-experiment-5/experiment-5.log`. Compile argv (unchanged) and both link argv:

```
/home/genius/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-g++ \
  -O2 -fexceptions -march=armv5t -mapcs -mthumb-interwork -mthumb -msoft-float \
  -D__SYMBIAN32__ -D__EPOC32__ -D__MARM__ -D__GCCE__ -D__EXE__ \
  -include /home/genius/sdk/S60_3rd_FP2/epoc32/include/gcce/gcce.h \
  -D__PRODUCT_INCLUDE__="/home/genius/sdk/S60_3rd_FP2/epoc32/include/variant/symbian_os_v9.3.hrh" \
  -nostdinc -c \
  -D__MARM_THUMB__ -D__MARM_INTERWORK__ -DNDEBUG -D_UNICODE \
  -D__S60_3X__ -D__SERIES60_3X__ \
  -D__EABI__ -D__MARM_ARMV5__ -D__SUPPORT_CPP_EXCEPTIONS__ \
  -I /home/genius/src/symdev-experiment-5 \
  -I /home/genius/sdk/S60_3rd_FP2/epoc32/include \
  -I /home/genius/sdk/S60_3rd_FP2/epoc32/include/variant \
  -I /home/genius/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0/include \
  -o hello.o hello.cpp
```

```
/home/genius/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-ld \
  -L/home/genius/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0/ \
  -L /home/genius/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib --target1-abs --no-undefined \
  -nostdlib -shared -Ttext 0x8000 -Tdata 0x400000 --default-symver -soname hello{000a0000}[e79e4cf9].exe \
  --target1-abs --no-undefined -nostdlib --strip-debug --entry _E32Startup -u _E32Startup \
  -L/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/urel -l:eexe.lib -o hello.elf \
  -Map hello.exe.map hello.o -( -l:usrt2_2.lib -) \
  -L/home/genius/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib \
  -L/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/lib -l:euser.dso -l:dfpaeabi.dso \
  -l:dfprvct2_2.dso -l:drtaeabi.dso -l:scppnwdl.dso -l:drtrvct2_2.dso -lsupc++ -lgcc
```

```
/home/genius/gcc-builds/binutils-2.29.1/bin/arm-none-symbianelf-ld \
  -L/home/genius/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0/ \
  -L /home/genius/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib --target1-abs --no-undefined \
  -nostdlib -shared -Ttext 0x8000 -Tdata 0x400000 --default-symver -soname hello{000a0000}[e79e4cf9].exe \
  --target1-abs --no-undefined -nostdlib --strip-debug --entry _E32Startup -u _E32Startup \
  -L/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/urel -l:eexe.lib -o hello.elf \
  -Map hello.exe.map hello.o -( -l:usrt2_2.lib -) \
  -L/home/genius/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib \
  -L/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/lib -l:euser.dso -l:dfpaeabi.dso \
  -l:dfprvct2_2.dso -l:drtaeabi.dso -l:scppnwdl.dso -l:drtrvct2_2.dso -lsupc++ -lgcc
```

## 6. soname / `--linkas` match

- **Requires:** experiments 3, 5
- **Skip if:** 3 or 5 not passed
- **Procedure:** Feed the ELF from 5 to `elf2e32`. Linker `-soname` and `elf2e32 --linkas` must match `<name>{version}[uid3].<ext>` (Verified). Use the Verified `elf2e32` fragment (`--uid1=0x1000007a`, `--targettype=EXE`, `--fpu=softvfp`, `--libpath=<SDK>/armv5/LIB`, …). Do not invent extra flags.
- **Expected result:** `elf2e32` accepts the ELF and writes an E32 image (`hello.exe` or the recorded output name).
- **Decision unblocked:** E32 image
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-5/` (outside git). Input ELF `/home/genius/src/symdev-experiment-5/hello.elf` (24244 bytes; experiment 5). `readelf -d` SONAME `hello{000a0000}[e79e4cf9].exe` matches experiment-5 linker `-soname` and `--linkas`. Binary `/home/genius/src/elf2e32_next/bin/Release/elf2e32` (`Symbian Post Linker, Elf2E32 v 3.0 (Build 2)`). `--libpath` = experiment-1 `armv5/lib`. `--uid3=0xe79e4cf9`. `--capability=` copied the Verified user-grantable six from [uids-capabilities-signing.md](uids-capabilities-signing.md) (`LocalServices`, `NetworkServices`, `ReadUserData`, `WriteUserData`, `UserEnvironment`, `Location`) joined with `+` as in `elf2e32_next` capability parser / tests (`AllFiles+TCB`); did not use `All-TCB` (privileged) or invent extra flags. Empty `--capability=` spelling remains Unknown and was not used. **Exit 0.** stdout/stderr empty. E32 `/home/genius/src/symdev-experiment-5/hello.exe` (3588 bytes; `file`: Psion Series 5 executable). Log `$HOME/src/symdev-experiment-5/experiment-6.log`. Experiment 7 not started: exact `.pkg` line syntax still Unknown (runbook ch.8). Argv:

```
/home/genius/src/elf2e32_next/bin/Release/elf2e32 \
  --uid1=0x1000007a --uid3=0xe79e4cf9 \
  --capability=LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location \
  --fpu=softvfp --targettype=EXE --output=hello.exe --elfinput=hello.elf \
  --linkas=hello{000a0000}[e79e4cf9].exe \
  --libpath=/home/genius/sdk/S60_3rd_FP2/epoc32/release/armv5/lib
```

## 7. `.pkg` path translation

- **Requires:** experiments 4, 6
- **Skip if:** 4 or 6 not passed
- **Procedure:** Write a `.pkg` whose **host paths exist on this Linux host** and whose Verified tokens are present: `&EN`, name+UID+version `TYPE=SA`, vendor lines, platform UID **`0x102752AE`**, EXE → `!:\sys\bin\`, optional reg rsc → `!:\private\10003a3f\import\apps\`. Host-side template path is Windows-style `$(EPOCROOT)Epoc32\release\armv5\urel\...`; Linux translation is Unknown. Exact `.pkg` line syntax is Unknown — do not invent header syntax (runbook chapter 8). Run `makesis` on that file.
- **Expected result:** `makesis` accepts the `.pkg` and produces a `.sis`. Record the host-path dialect that worked.
- **Decision unblocked:** SIS
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-5/` (outside git). E32 `/home/genius/src/symdev-experiment-5/hello.exe` (3588 bytes; experiment 6). Blog [How to build Symbian app by hand](https://fedor4ever.wordpress.com/2026/03/06/how-to-build-symbian-app-by-hand/) has **no `.pkg` listing** (hello.h / hello.cpp / g++ / ld / elf2e32 only). Experiment 1 had not unpacked examples; extracted InstallShield groups `Cpp_Examples` and `Symbian_Examples` with `$HOME/.local/bin/unshield -L -d $HOME/sdk/S60_3rd_FP2_examples -g <group> x data1.cab` (no sudo; outside git). **`.pkg` copied from** SDK `cpp_examples/locationsatviewrefapp/sis/locationsatviewrefapp_armv5.pkg` (has `&EN`, `TYPE=SA`, vendor lines, platform UID `0x102752AE`, EXE → `!:\sys\bin\`). Substituted only name `hello`, UID3 `0xe79e4cf9`, and a host path that exists here. Skipped that example’s rsc / `_reg.rsc` / mif / backup xml lines (those host files are absent; optional reg rsc is experiment 9). CRLF as in the SDK file. **Host-path dialect that worked: relative filename** `"hello.exe"` (`.pkg` and `.exe` in the same directory), matching the SDK example’s relative source-path dialect (`"..\..\..\epoc32\release\armv5\urel\SatelliteReference.exe"`). Unix absolute and Wine `Z:\…` were not tried. `makesis -h` usage: `MakeSIS [-h] [-i] [-s] [-v] [-d directory] pkgfile [sisfile]`. **Exit 0.** stderr: Wine `experimental wow64 mode` only. SIS `/home/genius/src/symdev-experiment-5/hello.sis` (4000 bytes; `file`: Symbian installation file (Symbian OS 9.x)). Log `$HOME/src/symdev-experiment-5/experiment-7.log`. `.pkg` and argv:

```
&EN
#{"hello"},(0xe79e4cf9),1,0,24,TYPE=SA
%{"Vendor-EN"}
:"Vendor"
[0x102752AE], 0, 0, 0, {"S60ProductID"}
"hello.exe"		-"!:\sys\bin\hello.exe"
```

```
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/makesis.exe -v hello.pkg hello.sis
```

## 8. Self-sign

- **Requires:** experiment 7
- **Skip if:** 7 not passed
- **Procedure:** Self-sign only (Symbian Signed is closed). `makekeys` then `signsis` using the Verified block in the runbook (`-expdays 3650` on Symbian 9.2+ tools). Exact `-dname` fields: Unknown — do not invent a Distinguished Name; record mandatory fields as observed. Password and `.cer`/`.key` stay local, never committed.
- **Expected result:** `signsis` produces a `.sisx` a human could copy (print the absolute path).
- **Decision unblocked:** an artifact a human could copy
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-5/` (outside git). Input SIS `/home/genius/src/symdev-experiment-5/hello.sis` (4000 bytes; experiment 7). **Wine, not native** (same PE tools as experiment 4). `makekeys` no-arg usage (exit 1): `-cert` / `-req` / `-view`; `-dname` **required** on `-cert`/`-req`; password “At least 4 characters”; `-expdays` optional else defaults to a year. **Observed `-dname` field names** from that usage (not invented; spec/runbook placeholders `OR`/`CO` are **not** listed): `CN` (Common Name), `C` (Country), `O` (Organisation), `OU` (Organisational Unit), `EM` (E-Mail). Usage note: “A distinguished name strings needs at least two attributes.” Example values copied from the tool’s own Example Usage (not a two-attribute trial). Local password used (`-password`, ≥4 chars; also passed as `signsis` passphrase); not committed. `makekeys -view hello.cer` printed `** Error showing certificate!` (exit 0). `signsis` positional form (no extra flags; `-s`/`-v`/`-c*` not passed). stderr: Wine `experimental wow64 mode` only. **Exit 0** both tools. Key `hello.key` (1264 bytes; `file`: PEM DSA private key) despite `-len 2048`. Cert `hello.cer` (1643 bytes; PEM certificate). SISX `/home/genius/src/symdev-experiment-5/hello.sisx` (5172 bytes; `file`: Symbian installation file (Symbian OS 9.x)). Log `$HOME/src/symdev-experiment-5/experiment-8.log`. Argv (password redacted):

```
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/makekeys.exe \
  -cert -expdays 3650 -password <local password, >=4 chars> -len 2048 \
  -dname "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com" \
  hello.key hello.cer
```

```
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/signsis.exe \
  hello.sis hello.sisx hello.cer hello.key <same local password>
```


## 9. Registration resource

- **Requires:** a hello that can be installed (typically after 8; launch evidence from 10 and/or 11 if those inputs exist)
- **Skip if:** no installable artifact (8 not passed) and no other legal hello SISX to compare
- **Procedure:** Determine whether launch-on-phone/emulator requires `_reg.rsc` produced via `rcomp`. Try with and without that resource if possible. `rcomp`/`epocrc` availability on Linux is Unknown — do not invent argv. Optional `.pkg` dest if a reg resource exists: `!:\private\10003a3f\import\apps\` (Verified template).
- **Expected result:** A recorded yes/no: M0 hello does or does not need `_reg.rsc` / `rcomp` to appear and launch. That answers whether `rcomp` is on the Wave 0 critical path.
- **Decision unblocked:** M0 hello contents and whether `rcomp` is on the Wave 0 critical path
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-5/` (outside git). Input SISX `/home/genius/src/symdev-experiment-5/hello.sisx` (5172 bytes; experiment 8; **no** `_reg.rsc` in that `.pkg`). **No phone, no emulator** — install+launch not tried here. Log `$HOME/src/symdev-experiment-5/experiment-9.log`.

  **(a) `rcomp` on Linux via Wine: yes (runs).** PE `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/rcomp.exe` (188416; PE32 i386). `/usr/bin/wine` `wine-10.0 (Ubuntu 10.0~repack-12ubuntu1)`. No-arg usage (Wine `experimental wow64 mode`; tool exit 255): `Resource compiler version 8.1 (Build 004)` then `Usage: rcomp [-vpul] [-force] [-oRSCFile] [-{uid2,uid3}] [-hHeaderFile] [-sSourceFile] [-iBaseInputFileName]` (`u` = Generate Unicode resource binary). Sibling `uidcrc.exe` (40960; PE32) no-arg: `uidcrc <uid1> <uid2> <uid3> [ <outputfile> ]`. Host `perl` `epocrc.pl` did **not** run: `FindBin::Bin` is rewritten `s/\//\\/g` so `@INC` is `\home\genius\sdk\S60_3rd_FP2\epoc32\tools`; `Can't locate lockit_info.pm` then (with `PERL5LIB` to the Linux tools dir) `Can't locate Pathutl.pm` from `lockit_info.pm`. `epocrc.bat` is `perl -S epocrc.pl %*` (Windows). Did not invent `epocrc` argv beyond its printed usage / script source.

  **(b) `_reg.rsc` from a copied SDK example `.rss`: yes.** No hello `.rss` in `rcomp`/`epocrc` help. Copied (not authored) example sources: Cpp_Examples `driveinfo/data/driveinfo_reg.rss` (only `#include <appinfo.rh>`), `locationsatviewrefapp/data/satellitereference_reg.rss` (`appinfo.rh` + `SatelliteReference.rsg`), `helloworldbasic/data/helloworldbasic_reg.rss` (`.rls` + `appinfo.rh` + `.rsg`); also `filebrowseapp_reg.rss` (Symbian_Examples; only `appinfo.rh`). Direct `wine rcomp.exe -u -odriveinfo_reg.rsc -sdriveinfo_reg.rss -idriveinfo_reg.rss` (flags from that usage) **page-faulted** in Wine (exit 5; unpreprocessed `#include`). `epocrc.pl` preprocessor line (not invented): `cpp -nostdinc -undef -C` plus `-D_UNICODE` when Unicode (`-u`); `-I` from `epocrc`/`cpp.exe` help. SDK `cpp.exe`: `/home/genius/sdk/S60_3rd_FP2/epoc32/gcc/bin/cpp.exe` (PE32). Unix `-I` to `epoc32/include` failed (`appinfo.rh: No include path`, exit 33). Wine `Z:\…` `-I` exit 0. Then `rcomp` on the `.rpp` parsed but `Failed to write UIDs to driveinfo_reg.rsc` until **`WINEPATH`** included the SDK tools dir (so `rcomp` can spawn sibling `uidcrc.exe`). Recorded working argv (copied flags only; `WINEPATH` is the Wine lookup for `uidcrc.exe`):

```
WINEPATH=/home/genius/sdk/S60_3rd_FP2/epoc32/tools \
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/gcc/bin/cpp.exe \
  -nostdinc -undef -C -D_UNICODE \
  -I 'Z:\home\genius\sdk\S60_3rd_FP2\epoc32\include' \
  driveinfo_reg.rss -o driveinfo_reg.rpp
```

```
WINEPATH=/home/genius/sdk/S60_3rd_FP2/epoc32/tools \
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/rcomp.exe \
  -u -odriveinfo_reg.rsc -sdriveinfo_reg.rpp -idriveinfo_reg.rss
```

  **Exit 0.** `driveinfo_reg.rsc` 74 bytes; same path `filebrowseapp_reg.rsc` 109 bytes. `satellitereference_reg.rss` / `helloworldbasic_reg.rss` cpp exit 33 (`SatelliteReference.rsg` missing; `Helloworldbasic.rls` / `Helloworldbasic.rsg` missing). **Limitation:** tools help + unpacked examples do not give a hello `.rss` that compiles without a prior `.rsg` from the main app resource. Did not invent a hello registration `.rss`.

  **(c) M0 hello need `_reg.rsc` to appear and launch: unknown until experiments 10/11.** Try-without = experiment 8 SISX (no reg rsc). Try-with = rsc produced from copied examples, not packaged into hello (wrong `app_file` / UID3; no hello `.rss`). Without a phone/emulator this experiment cannot prove the hello appears on a device. Whether `rcomp` is on the Wave 0 critical path stays unknown until 10/11.

## 10. EKA2L1 install + launch

- **Requires:** experiment 8 (a `.sisx` to install). User-supplied `SYMDEV_EKA2L1` and `SYMDEV_ROM` ([eka2l1.md](eka2l1.md))
- **Skip if:** `SYMDEV_ROM` or `SYMDEV_EKA2L1` unset, or either path missing. Skip without ROM. Skip does **not** fail §17 accept.
- **Procedure:** Install the hand-built `.sisx` and launch the app using the user-installed EKA2L1 process and user ROM. **Observe** CLI and on-disk ROM layout; do not invent flags. EKA2L1 is GPL-3.0: process only, never vendor source. Headless install + screenshot is **M5**, not this experiment.
- **Expected result:** Observed install + launch (or a recorded failure). CLI flags and ROM layout written into research notes.
- **Decision unblocked:** interim de-risk. Does **not** unblock “E52 supported.”
- **Outcome:** pass (2026-09-19; see experiments 45–48). Originally skipped 2026-09-17 (evidence below).
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. `SYMDEV_EKA2L1` **unset** (`k in os.environ` is false). `SYMDEV_ROM` **unset**. No download. Common paths missing: `$HOME/src/eka2l1`, `$HOME/src/EKA2L1`, `$HOME/eka2l1`, `$HOME/EKA2L1`, `$HOME/.local/bin/eka2l1`, `/opt/eka2l1`, `/usr/local/bin/eka2l1`, `/usr/bin/eka2l1`; `which eka2l1` / `EKA2L1` not on PATH; `find` under `$HOME/src`, `$HOME/.local`, `/opt`, `/usr/local` returned no `*eka2l1*` names. ROM candidates missing: `$HOME/rom`, `$HOME/roms`, `$HOME/src/rom`, `$HOME/Downloads/*.img` / `*ROM*` / `*.bin`. SISX path not installed: `/home/genius/src/symdev-experiment-5/hello.sisx`. Skip does not fail §17 accept. Does not authorize emulator or E52 support. Does not lift the M1 gate (experiments 8+10 not both present).

## 11. Stock E52 install + launch

- **Requires:** experiment 8 (hand-built `.sisx`)
- **Skip if:** no Nokia E52. Skip without a phone.
- **Procedure:** Copy the `.sisx` by memory card or Bluetooth OBEX (Verified practical paths). On the phone: App. Mgr → Settings → **Software installation = All**, **Online certificate check = Off** (Verified sufficient for self-signed user-grantable caps). Install and launch on a **stock** E52 (RM-469). Do not use `gnokii` / `gammu`. This is **Hardware M0**.
- **Expected result:** The `.sisx` installs and the app launches on that stock E52.
- **Decision unblocked:** claiming device support (Hardware M0)
- **Outcome:** skip
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. No Nokia E52 provided in this session. `lsusb` showed host hubs, Logitech Bolt receiver, Intel AX200 Bluetooth — no Nokia/phone device. Did not use gnokii/gammu. SISX not copied to hardware: `/home/genius/src/symdev-experiment-5/hello.sisx`. Skip without a phone. Skip does not fail §17 accept. Does not authorize claiming E52 support.

## 12. `bld.inf` `PRJ_PLATFORMS` tokens and `#if` macros in the FP2 SDK

- **Requires:** SDK (experiment 1 passed, not skipped)
- **Skip if:** no SDK
- **Procedure:** In the user FP2 SDK tree, inspect real `bld.inf` files (and related headers) for `PRJ_PLATFORMS` tokens and live `#if` macros. Record names as they appear. Do not invent a default macro table or alias list ([mmp-bld-inf-parser.md](mmp-bld-inf-parser.md)). Never commit SDK files.
- **Expected result:** A recorded token list (whether `ARMV5`, `GCCE`, `UREL`, or other spellings appear) and a recorded list of macros used in `#if` in that SDK.
- **Decision unblocked:** M1 parser evaluation rules
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Inspected real `bld.inf` only; **no SDK files committed.** EPOCROOT `/home/genius/sdk/S60_3rd_FP2/` has **0** `bld.inf` (unified `epoc32` from experiment 1). Unpacked examples `/home/genius/sdk/S60_3rd_FP2_examples/`: **199** `bld.inf` (`cpp_examples/` + `symbian_examples/`). Related header: `$EPOCROOT/epoc32/include/variant/symbian_os_v9.3.hrh` contains `#define EKA2` (line 23). No `_CARBIDE_CPP_` in that HRH. Names below are as they appear; no alias table invented.

  **`PRJ_PLATFORMS` presence (199 files):** section present 74; present but **empty** (no tokens) 33 (e.g. `cpp_examples/locationsatviewrefapp/group/bld.inf`, `cpp_examples/webclient/group/bld.inf`); **omitted** 125 (e.g. `symbian_examples/basics/helloworld/bld.inf` starts at `PRJ_MMPFILES`).

  **Token spellings observed** (counts = files/sections contributing that token): `DEFAULT` (28), `ARMV5_ABIV2` (19), `GCCE` (17), `WINSCW` (12), `ARMV5` (9), `default` (1), `gcce` (1). **`UREL` does not appear** as a `PRJ_PLATFORMS` token (`UDEB` appears only inside some `PRJ_EXPORTS` dest paths, e.g. `WINSCW\UDEB\…`). Combinations as they appear: `DEFAULT ARMV5_ABIV2` (19); `WINSCW ARMV5 GCCE` (9, one line e.g. `cpp_examples/aiwconsumerbasics/group/bld.inf`: `WINSCW ARMV5 GCCE`); `DEFAULT GCCE` (5); `DEFAULT` alone (4, e.g. `cpp_examples/helloworldbasic/group/bld.inf`); `WINSCW GCCE` (3, e.g. `symbian_examples/filebrowse/s60/filebrowse/group/bld.inf`); `default gcce` (1: `cpp_examples/openc_ex/opencopenglex/group/bld.inf`).

  **Live `#if` family in those `bld.inf`:** no `#if`, no `#ifndef`, no `#elif`. Only `#ifdef` / `#else` / `#endif`. Macros in `#ifdef` conditions: **`EKA2`** (8 uses across 5 files: `richtexteditor`, `myview`, `localization`, `dynamicsettinglist`, `audiostreamexample` group `bld.inf`); **`_CARBIDE_CPP_`** (2 uses, one file: `cpp_examples/myview/group/bld.inf`). `__GCCE__` does not appear in these `bld.inf` conditionals.

## 13. `uidcrc.exe` golden output (T1)

- **Requires:** experiment 4 (Wine SIS/tools PE run). `uidcrc.exe` usage recorded in experiment 9.
- **Skip if:** no SDK `uidcrc.exe` or no Wine
- **Procedure:** Run the recorded usage `uidcrc <uid1> <uid2> <uid3> [ <outputfile> ]` via Wine. Do not invent flags. Capture stdout (no outfile) and the 16-byte outfile. Derive a clean-room checked-UID function that matches those bytes. Do not copy `uidcrc` C sources (Symbian Example Source Code License).
- **Expected result:** Recorded argv, stdout line, outfile layout, and pinned triples for tests.
- **Decision unblocked:** T1 clean-room `uidcrc` (native `makekeys` still not golden-diffable: RSA + dates).
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-13/` (outside git). PE `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/uidcrc.exe` (40960). `/usr/bin/wine` wine-10.0. No-arg: usage `uidcrc <uid1> <uid2> <uid3> [ <outputfile> ]` (exit 255; wow64 stderr). With outfile: `wine uidcrc.exe 0x1000007a 0x100039CE 0xe79e4cf9 uidcrc-hex.uid` exit 0; 16-byte file. Lowercase hex same bytes. No outfile: stdout `0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e` (CRLF). File little-endian UIDs + checked UID: `7a 00 00 10 ce 39 00 10 f9 4c 9e e7 4e 19 cf 5d`. Additional stdout goldens (Wine, 0x-hex args):

  - `0 0 0` → checked `0x00000000`
  - `0x1000007a 0 0` → `0x045ac39e`
  - `0xffffffff ×3` → `0x97df97df`
  - `0x1000007a 0x100039ce 0` → `0x98a7d260`
  - `0x12345678 0x9abcdef0 0x11111111` → `0x3a5febb7`

  Clean-room match (derived from those files, not from uidcrc.c): 12-byte LE concatenation of uid1/uid2/uid3; EPOC CRC16 on odd bytes (high 16) and even bytes (low 16). CRC16 byte step: `crc = rotl8(crc) ^ b; crc ^= (crc & 0xff) >> 4; crc ^= crc << 12; crc ^= (crc & 0xff) << 5` (16-bit).

## 14. SIS UID header on experiment-7 `hello.sis` (T2)

- **Requires:** experiment 7 (`hello.sis`) and experiment 13 (`UidCrc`).
- **Skip if:** no `hello.sis` from experiment 7
- **Procedure:** Inspect the frozen experiment-7 SIS (do not re-run `makesis`; creation time would change). Read the first 16 bytes as four little-endian `u32`. Check that `UidCrc::new(uid1, uid2, uid3).checked()` equals the fourth word. Repeat the first 16 bytes of experiment-8 `hello.sisx`. Do not copy MakeSIS C sources. Do not commit `.sis` / `.sisx`.
- **Expected result:** Pinned SIS UID1/UID2/UID3/checked for hello, and the 16 raw bytes.
- **Decision unblocked:** T2 first slice — native SIS UID header via `UidCrc` (full native `makesis` still later).
- **Outcome:** pass
- **Evidence:** 2026-09-17, Ubuntu 26.04.1 LTS x86_64. Files outside git: `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes; experiment 7) and `hello.sisx` (5172 bytes; experiment 8). First 16 bytes of both files are identical. Little-endian words:

  - uid1 `0x10201a7a` (SIS file UID)
  - uid2 `0x00000000` (this SDK `makesis` output; not a wiki default)
  - uid3 `0xe79e4cf9` (hello package UID)
  - checked `0x5db40004` — matches `UidCrc` on those three UIDs

  Raw 16 bytes: `7a 1a 20 10 00 00 00 00 f9 4c 9e e7 04 00 b4 5d`. After the UID block, both files continue with a field whose type word is `0x0000000c` and whose length word is `0x00000f88` (SIS, 3976) or `0x0000141c` (SISX, 5148); `16 + 8 + length` equals file size. Native SIS body / signatures are **out of this experiment**.

## 15. SIS field TLV + 4-byte padding (T2)

- **Requires:** experiment 14 (same frozen `hello.sis` / `hello.sisx`).
- **Skip if:** those files are gone
- **Procedure:** After the 16-byte UID, walk type+length fields (two little-endian `u32`, then `length` payload bytes). Record how the next field is aligned. Do not copy MakeSIS C. Do not commit `.sis` / `.sisx`. Do not re-run `makesis`.
- **Expected result:** Pinned outer-field headers, nested first-field bytes, and the padding rule.
- **Decision unblocked:** T2 `SisField` encode (still not a full native `makesis`).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same files as experiment 14. Outer field immediately after UID: type `12` (`0x0c`), length `3976` (`0x00000f88`) on SIS and `5148` (`0x0000141c`) on SISX. Outer headers:

  - SIS: `0c 00 00 00 88 0f 00 00`
  - SISX: `0c 00 00 00 1c 14 00 00`

  Nested walk of the SIS controller payload (offsets from start of that payload): type `34` length `2` payload `5c 9e` at 0; type `35` length `2` payload `64 03` at 12; type `3` length `295` at 24; type `30` length `3640` at 328. Next field starts at `8 + ((length + 3) & ~3)` from the current field start (2-byte payloads occupy 12 bytes: 8 header + 2 data + 2 zero pad). Type `3` length `295` occupies `8 + 296` (one pad byte) so the following field is at 328. Native compressed controller / file data / signatures remain **out of this experiment**.

## 16. SIS compressed-field prefix (T2)

- **Requires:** experiment 15 (type `3` payload inside the controller).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** On the type-`3` payload, record the leading little-endian `u32` words and where a zlib stream (`78 9c`) starts. Repeat for experiment-8 `hello.sisx`. Decompress only to record uncompressed size, not to commit the body. Do not copy MakeSIS C. Do not add a zlib crate this experiment. Do not commit `.sis` / `.sisx`.
- **Expected result:** Pinned 12-byte prefix: algorithm, uncompressed size, a zero word, then zlib. Field type `3` length equals `12 + zlib_len`.
- **Decision unblocked:** T2 `SisCompressed` encode (inflate / checksums / data still later).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same frozen files. Type-`3` payload starts `01 00 00 00 <uncomp as u32 LE> 00 00 00 00 78 9c …`. Host `zlib.decompress` of bytes from offset 12:

  - SIS: algorithm `1`, uncompressed `548` (`0x00000224`), reserved `0`, zlib starts at 12, payload length `295` (`12 + 283`). Prefix `01 00 00 00 24 02 00 00 00 00 00 00`. Inflated 548 bytes.
  - SISX: algorithm `1`, uncompressed `1868` (`0x0000074c`), reserved `0`, zlib at 12, payload length `1467`. Prefix `01 00 00 00 4c 07 00 00 00 00 00 00`. Inflated 1868 bytes.

  Algorithm `1` is recorded as this SDK’s deflate. The reserved word is `0` on both files; do not invent another meaning. Native inflate, type-34/35 checksums, and type-30 data stay **out of this experiment**.

## 17. SIS UTF-16 strings inside inflated type 13 (T2)

- **Requires:** experiment 16 (host inflate of type-3 payload, crate still does not inflate).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Inflate the type-3 zlib on the host (Python `zlib`, not a repo crate). Walk type-1 fields as UTF-16-LE without a BOM or NUL. Do not copy MakeSIS C. Do not commit inflated bytes or `.sis`.
- **Expected result:** Pinned type `1` payloads for `Vendor` and `hello`, and how odd-length UTF-16 is padded by the existing `SisField` rule.
- **Decision unblocked:** T2 `SisString` encode.
- **Outcome:** pass
-   **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Inflated SIS controller is 548 bytes: one field type `13` length `540`. Inside that, a type-`14` block contains type `1` length `12` payload `56 00 65 00 6e 00 64 00 6f 00 72 00` (`Vendor`) and a nested type `1` length `10` payload `68 00 65 00 6c 00 6c 00 6f 00` (`hello`). No BOM, no terminating NUL. `hello` is 10 bytes so `SisField` adds two zero pad bytes (`… 6f 00 00 00`). Native inflate in-tree, SIS arrays, and version fields stay **out of this experiment**.

## 18. SIS version triple inside inflated type 13 (T2)

- **Requires:** experiment 17 (same host inflate).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** In the type-`14` block, record the type-`4` field as three little-endian `u32`. Compare to experiment-7 `.pkg` `#{"hello"},(0xe79e4cf9),1,0,24,TYPE=SA`. Do not copy MakeSIS C. Do not commit `.sis`.
- **Expected result:** Pinned type `4` payload `1, 0, 24`.
- **Decision unblocked:** T2 `SisVersion` encode.
- **Outcome:** pass
-   **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Type `4` length `12` payload `01 00 00 00 00 00 00 00 18 00 00 00` = major `1`, minor `0`, build `24`. Matches the experiment-7 `.pkg` version triple. Native inflate, arrays, and type-9 UID stay **out of this experiment**.

## 19. SIS package UID inside inflated type 13 (T2)

- **Requires:** experiment 18 (same host inflate).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** In the type-`14` block, record the type-`9` field as one little-endian `u32`. Compare to hello UID3 `0xe79e4cf9`. Do not copy MakeSIS C. Do not commit `.sis`. This is **not** the 16-byte file `SisUid` header.
- **Expected result:** Pinned type `9` payload `f9 4c 9e e7`.
- **Decision unblocked:** T2 `SisPkgUid` encode.
- **Outcome:** pass
-   **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Type `9` length `4` payload `f9 4c 9e e7` = `0xe79e4cf9`. Same UID3 as experiment 7 `.pkg` and `SisUid::new` package word. Native inflate and type-2 arrays stay **out of this experiment**.

## 20. SIS array of fields (type 2) (T2)

- **Requires:** experiments 17–19 (string + field encode).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** In the type-`14` block, record type-`2` payloads as concatenated nested TLV fields (already 4-byte padded). Do not copy MakeSIS C. Do not commit `.sis`.
- **Expected result:** Pinned type `2` wrapping one type-1 `hello` string field.
- **Decision unblocked:** T2 `SisArray` encode.
- **Outcome:** pass
-   **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Type `2` length `20` payload is exactly `SisString::new("hello").field().bytes()` (`01 00 00 00 0a 00 00 00 68 00 65 00 6c 00 6c 00 6f 00 00 00`). A second type `2` length `28` wraps `Vendor-EN` the same way. Payload is concatenated child `SisField::bytes()`, not a count prefix. Native inflate stays **out of this experiment**.

## 21. SIS date/time inside type 8 (T2)

- **Requires:** experiment 20 (same inflated type-14 block).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record the type-`8` payload as nested TLV. Do not invent calendar fields: copy the bytes, then name year/month/day/hour/minute/second only if they match the frozen file’s known stamp. Do not copy MakeSIS C. Do not commit `.sis`. Do not add a time crate.
- **Expected result:** Pinned type-6 date and type-7 time payloads, and type-8 as those two fields concatenated.
- **Decision unblocked:** T2 `SisDate` / `SisTime` / `SisDateTime` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Type `8` length `24` payload:

  - type `6` length `4` payload `ea 07 08 11`: little-endian year `0x07ea` = `2026`, next byte `8`, next byte `0x11` = `17`. Frozen `hello.sis` mtime is 2026-09-17; month byte `8` is 0-based (September).
  - type `7` length `3` payload `0f 12 18` = `15`, `18`, `24`, then one zero pad. Host local mtime 17:18 CEST is 15:18 UTC; seconds `24` as in the file.

  Type `8` is those two child `SisField::bytes()` concatenated (no extra prefix). Do not generate “now” in this slice. Native inflate and type-14 compose stay **out of this experiment**.

## 22. SIS info (type 14) from existing leaves (T2)

- **Requires:** experiments 17–21 (string, array, version, pkg UID, datetime).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record the inflated type-`14` block as concatenated child fields already encoded in T2, plus any leftover bytes inside the unpadded length. Do not copy MakeSIS C. Do not commit `.sis`. Do not generate a clock stamp. Do not inflate in-crate.
- **Expected result:** Pinned type `14` wrapping pkg UID, vendor string, names array, vendor-names array, version, datetime.
- **Decision unblocked:** T2 `SisInfo` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same host inflate of frozen `hello.sis` as experiments 17–21. Type `14` length `150` (`0x96`), field 160 bytes including SisField pad. Children in order (each already a padded `SisField::bytes()`):

  1. type `9` `SisPkgUid::new(0xe79e4cf9)` (12 bytes)
  2. type `1` `SisString::new("Vendor")` (20 bytes)
  3. type `2` `SisArray` of `hello` (28 bytes)
  4. type `2` `SisArray` of `Vendor-EN` (36 bytes)
  5. type `4` `SisVersion::new(1, 0, 24)` (20 bytes)
  6. type `8` `SisDateTime` of the experiment-21 stamp (32 bytes)

  Those six fields are **148** bytes. Two extra `00` bytes sit inside the unpadded length `150` after the datetime field. `SisField` then pads length `150` with two more zeros (`150 % 4 == 2`). Do not treat the inner two zeros as `SisField` padding. Native inflate, type-13 siblings (16/15/17/19/28/40), and `package` wiring stay **out of this experiment**.

## 23. SIS language (types 11 and 15) (T2)

- **Requires:** experiments 20 and 22 (type-2 array encode; same inflated type-13 block).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `15` and its nested type `11` from the inflated controller. Compare the type-11 word to the frozen `hello.pkg` `&EN` line (English only). Do not copy MakeSIS C. Do not commit `.sis` / `.pkg`. Do not invent other language IDs.
- **Expected result:** Pinned type-11 payload `01 00 00 00` and type 15 as a type-2 array of that field.
- **Decision unblocked:** T2 `SisLanguage` / `SisLanguages` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `hello.pkg` has `&EN` and one name `hello`. Inflated type `15` length `20` payload is exactly `SisArray::new(vec![type-11 field]).field().bytes()`:

  - type `11` length `4` payload `01 00 00 00` = `1`
  - type `2` length `12` wraps that type-11 field
  - type `15` wraps that type-2 field (`0f 00 00 00 14 00 00 00` + 20-byte array field)

  Type `16` (payload word `0x21`) is **not** this language id and stays out of this experiment. Native inflate and remaining type-13 siblings stay **out of this experiment**.

## 24. SIS product (types 5 and 18) (T2)

- **Requires:** experiments 18–20 (version, pkg UID, string, array) and the same inflated type-13 block as 22–23.
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `18` and nested type `5` from the inflated controller. Compare to the frozen `hello.pkg` line `[0x102752AE], 0, 0, 0, {"S60ProductID"}`. Do not copy MakeSIS C. Do not commit `.sis` / `.pkg`. Do not invent a from/to second version: the golden has one nested type-4 `0,0,0`.
- **Expected result:** Pinned type-5 wrapping `SisVersion::new(0, 0, 0)`, and type 18 as pkg UID + that wrap + names array `S60ProductID`.
- **Decision unblocked:** T2 `SisProductVersion` / `SisProduct` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `hello.pkg` has `[0x102752AE], 0, 0, 0, {"S60ProductID"}`. Inflated type `18` length `80`:

  - type `9` `SisPkgUid::new(0x102752AE)` payload `ae 52 27 10`
  - type `5` length `20` payload is exactly `SisVersion::new(0, 0, 0).field().bytes()`
  - type `2` wrapping type-1 `S60ProductID`

  Type `17` (array of this product plus a type-2 word `0x12`) stays **out of this experiment**. Native inflate, type 16/19/28/40, and `package` wiring stay out.

## 25. SIS type-2 raw u32 words inside type 16 (T2)

- **Requires:** experiments 16 and 23 (host inflate of type-3; type 11/15 already pinned as the EN language list).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Same host inflate of frozen `hello.sis` as experiments 17–24 (Python `zlib`, not a repo crate). In the type-`13` block, record type `16` and its nested type `2`. Confirm the type-2 payload is concatenated little-endian `u32` words, not concatenated `SisField::bytes()` (`SisArray`). Compare the word to experiment-23 language id `1`. Do not copy MakeSIS C. Do not invent a C / Options / Languages name. Do not commit `.sis`.
- **Expected result:** Pinned type `16` wrapping a type-2 whose payload is one LE u32 `0x21`. That word is not the EN language id.
- **Decision unblocked:** T2 `SisWords` / `SisWords16` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes). Host `zlib.decompress` of type-3 bytes from offset 12 → 548 bytes, one type `13` length `540`. Inside that, type `16` length `12` at offset 160. Field bytes (20):

  `10 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 21 00 00 00`

  Nested type `2` length `4` payload `21 00 00 00` = `0x21`. Not a nested TLV field (a `SisArray` of one child would be longer than 4 bytes). Experiment 23 type-11 word is `1`; `0x21` is not that id. Native inflate, type 17/19/28/40, and `package` wiring stay **out of this experiment**.

## 26. SIS type-2 raw u32 words inside type 19 (T2)

- **Requires:** experiment 25 (same inflated type-13 block; same raw-u32 type-2 rule).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `19` from the same inflated controller. Confirm it wraps type 2 the same way as type 16 (raw u32 concat, not `SisArray`). Do not copy MakeSIS C. Do not invent a C / Wine name. Do not commit `.sis`.
- **Expected result:** Pinned type `19` wrapping a type-2 whose payload is one LE u32 `0x14`.
- **Decision unblocked:** T2 `SisWords19` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same inflate. Type `19` length `12` at type-13 offset 324. Field bytes (20):

  `13 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 14 00 00 00`

  Nested type `2` length `4` payload `14 00 00 00` = `0x14`. Same encoding as experiment 25’s inner type 2. Type 28 and type 17 stay **out of this experiment**. Native inflate and `package` wiring stay out.

## 27. SIS type 40 u32 (T2)

- **Requires:** experiment 25 (same inflated type-13 block).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `40` from the same inflated controller. Confirm it is one little-endian `u32` and is **not** nested in a type-2 wrapper. Do not copy MakeSIS C. Do not invent a C name. Do not commit `.sis`.
- **Expected result:** Pinned type `40` length `4` payload `00 00 00 00`.
- **Decision unblocked:** T2 `SisU32` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same inflate. Type `40` length `4` at type-13 offset 528. Field bytes (12):

  `28 00 00 00 04 00 00 00 00 00 00 00`

  Payload is one LE u32 `0`. No inner type-2 header. Type 28 stays **out of this experiment**. Native inflate, type-13 compose, and `package` wiring stay out.

## 28. SIS products list (type 17) (T2)

- **Requires:** experiment 24 (type-18 product) and the same inflated type-13 block.
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `17` from the inflated controller. It wraps a type-2 array of the experiment-24 type-18 field, then a type-2 field of length 4 whose payload is LE `u32` `0x12`. Do not copy MakeSIS C. Do not commit `.sis` / `.pkg`. Do not invent a second version. Do not add `SisWords`.
- **Expected result:** Pinned type-17 field 116 bytes, payload `n=108`: type-2 array `n=88` of one hello `SisProduct` field, then `02 00 00 00 04 00 00 00 12 00 00 00`.
- **Decision unblocked:** T2 `SisProducts` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same frozen hello type-13 as experiments 22–24. Type `17` length `108` (`0x6c`), field 116 bytes:

  1. type `2` length `88` (`0x58`) whose payload is exactly `SisProduct::new(SisPkgUid::new(0x102752AE), SisProductVersion::new(SisVersion::new(0,0,0)), SisArray::new(vec![SisString::new("S60ProductID").field()])).field().bytes()`
  2. type `2` length `4` payload `12 00 00 00`

  Concatenate those 12 trailing bytes in `payload()`. Do not create `SisWords` here. Types 16/19/40, native inflate, and `package` wiring stay **out of this experiment**.

## 29. SIS file record (type 24) (T2)

- **Requires:** experiments 17 and 25 (string encode; raw-u32 type-2 rule; same inflated type-13 block).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Same host inflate of frozen `hello.sis` as experiments 17–28 (Python `zlib`, not a repo crate). Record type `24` inside type `28`. Walk nested TLVs. Do not copy MakeSIS C. Do not invent C / capability names. Do not commit `.sis` / `.exe`. Confirm the type-25 trailing 20 bytes against SHA-1 of frozen `hello.exe` (experiment 6). Confirm leftover `0x0e04` against that file’s size. Do **not** walk leftover `04 0e 00 00 00 00 00 00` as type `3588` length 0. A previous walk’s “type 29” was KIND `41` (header byte `0x29`).
- **Expected result:** Pinned type-24 field 144 bytes, payload `n=136`: dest `SisString` `!:\sys\bin\hello.exe`, empty `SisString`, type `41` u32 `0x000be000`, type `25` three LE u32 plus 20-byte digest, empty `SisString`, then five LE u32 `3588, 0, 3588, 0, 0`.
- **Decision unblocked:** T2 `SisWord41` / `SisHash` / `SisFile` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same inflate as experiment 25 (548 bytes, type `13` n=540). Type `24` length `136` (`0x88`) inside type `28`’s type-2 array. Field 144 bytes. Children inside the unpadded payload:

  1. type `1` n=40 UTF-16-LE `!:\sys\bin\hello.exe` (matches experiment-7 pkg `"hello.exe"-"!:\sys\bin\hello.exe"`)
  2. type `1` n=0 empty
  3. type `41` n=4 payload `00 e0 0b 00` = `0x000be000`
  4. type `25` n=32 payload `01 00 00 00 25 00 00 00 14 00 00 00` + 20 bytes `3a23e7e7e60ed97354534b2a77e565cd64ea3970`. Host `hashlib.sha1` of `$HOME/src/symdev-experiment-5/hello.exe` (3588 bytes) equals those 20 bytes. Payload is not nested TLVs (`type=1 n=0x25` would overrun).
  5. type `1` n=0 empty
  6. leftover 20 bytes `04 0e 00 00 00 00 00 00 04 0e 00 00 00 00 00 00 00 00 00 00` = five LE u32 `3588, 0, 3588, 0, 0`. `3588` is experiment-6 `hello.exe` size. Not a TLV.

  Native inflate, type 28 wrapper, type 13 compose, type 30 data, and `package` wiring stay **out of this experiment**.

## 30. SIS files list (type 28) (T2)

- **Requires:** experiment 29 (type-24 file) and experiment 25 (`SisWords` raw-u32 type 2).
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Record type `28` from the same inflated controller. Confirm it wraps a type-2 array of the experiment-29 type-24 field, then two type-2 fields of length 4 whose payloads are LE u32 `0x0d` and `0x1a`. Those are raw u32 arrays (`SisWords`), not `SisArray`. Do not copy MakeSIS C. Do not commit `.sis`. Do not pin type 28 as opaque bytes.
- **Expected result:** Pinned type-28 field 184 bytes, payload `n=176`: type-2 array `n=144` of one hello `SisFile` field, then `SisWords::new(vec![0x0d])` and `SisWords::new(vec![0x1a])`.
- **Decision unblocked:** T2 `SisFiles` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Same inflate. Type `28` length `176` (`0xb0`) at type-13 offset 344. Field 184 bytes:

  1. type `2` length `144` (`0x90`) whose payload is exactly the 144-byte type-24 field from experiment 29
  2. type `2` length `4` payload `0d 00 00 00` = `0x0d`
  3. type `2` length `4` payload `1a 00 00 00` = `0x1a`

  Header `1c 00 00 00 b0 00 00 00`. Type 13 compose, type 30 data, checksums 34/35, and `package` wiring stay **out of this experiment**.

## 31. SIS type 13 controller body (T2)

- **Requires:** experiments 22–30 (info, words16, languages, products, words19, files, u32) and the same inflated blob as experiment 16.
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Same host inflate of frozen `hello.sis` type-3 zlib (Python `zlib`, not a repo crate). Confirm the inflated 548 bytes are one type-13 field, unpadded length 540, whose payload is the concatenation of the already-pinned child fields in order, including the experiment-30 type-28 field (not opaque bytes). Do not copy MakeSIS C. Do not commit `.sis`. Do not inflate in-crate. Do not encode type 3 / 30 / 34 / 35 here.
- **Expected result:** Pinned type-13 field 548 bytes, header `0d 00 00 00 1c 02 00 00`, payload `n=540` = `SisInfo` + `SisWords16` + `SisLanguages` + `SisProducts` + `SisWords19` + `SisFiles` + `SisU32(0)`.
- **Decision unblocked:** T2 `SisController` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Inflated type-3 payload is 548 bytes: type `13` length `540` (`0x0000021c`). No leftover after the field (`540 % 4 == 0`). Children occupied sizes at type-13 payload offsets:

  | off | type | n | occupied |
  |-----|------|---|----------|
  | 0 | 14 `SisInfo` | 150 | 160 |
  | 160 | 16 `SisWords16` | 12 | 20 |
  | 180 | 15 `SisLanguages` | 20 | 28 |
  | 208 | 17 `SisProducts` | 108 | 116 |
  | 324 | 19 `SisWords19` | 12 | 20 |
  | 344 | 28 `SisFiles` | 176 | 184 |
  | 528 | 40 `SisU32` | 4 | 12 |

  Sum 540. Type 28 is the experiment-30 field, not a byte stub. Native inflate, type-3 compress, checksums 34/35, type 30 data, and `package` wiring stay **out of this experiment**.

## 32. SIS type 34/35 checksums (T2)

- **Requires:** experiments 15–16 (type-12 children; type-3 zlib prefix) and experiment 13 (EPOC CRC16 byte step). Frozen experiment-7 `hello.sis` and experiment-8 `hello.sisx`.
- **Skip if:** those files are gone
- **Procedure:** Re-dump the frozen files. Walk type-12 children. Try CRC16/CRC32 variants on compressed payload, uncompressed type 13, type-12 payload without checksums, file after UID, zlib-only, exe, and padded type-3 / type-30 field bytes. Do not copy MakeSIS C. Do not commit `.sis` / `.sisx` / `.exe`. Confirm a match on both SIS and SISX.
- **Expected result:** Pinned type-34/35 two-byte payloads and the CRC input that produces them, or a recorded unknown algorithm with the two-byte goldens still usable.
- **Decision unblocked:** T2 `SisChecksum34` / `SisChecksum35` encode (outer type 12 still later).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes) and `hello.sisx` (5172 bytes). After the 16-byte UID, outer type 12. First children of that payload (SIS):

  | off | type | n | occupied | payload |
  |-----|------|---|----------|---------|
  | 0 | 34 | 2 | 12 | `5c 9e` (LE u16 `0x9e5c`) |
  | 12 | 35 | 2 | 12 | `64 03` (LE u16 `0x0364`) |
  | 24 | 3 | 295 | 304 | deflate, uncompressed 548 |
  | 328 | 30 | 3640 | 3648 | file data (alg-0 nested type 3 holds 3588-byte `hello.exe`) |

  SISX type 34 is `01 c4` (`0xc401`); type 35 is the same `64 03`. Type-3 fields differ (SIS occupied 304, SISX occupied 1476, inflated 1868). Type-30 field bytes are identical (3648).

  **Tried (no match for the paired type-34 SIS/SISX targets and type-35 `0x0364`):** CRC-32/IEEE (low/high 16), Adler-32, sum16, CRC-16 IBM / CCITT reflected, CRC-16/T10-DIF, init `0xffff` / `0x1d0f` / xorout `0xffff`, EPOC CRC16 init ≠ 0. Blobs that failed those algos: type-3 payload, zlib-only, type-3 prefix, inflated type 13 / type-13 payload, type-12 payload from type 3 onward, type-12 with checksum payloads zeroed, file after UID, whole file, `hello.exe`, type-3 field without the pad byte (303 / 1475).

  **Match:** EPOC CRC16 init `0` (same byte step as experiment 13: `crc = rotl8(crc) ^ b; crc ^= (crc & 0xff) >> 4; crc ^= crc << 12; crc ^= (crc & 0xff) << 5`). Same values as CRC-16/XMODEM (`poly 0x1021`, init 0, xorout 0, not reflected) on these inputs and on 50 random 32-byte samples. Input is the **padded** `SisField::bytes()` of the inner field (kind, length, payload, pad):

  - type 34 ← CRC16 of the type-3 field (SIS 304 bytes including one pad `00` → `5c 9e`; SISX 1476 bytes → `01 c4`)
  - type 35 ← CRC16 of the type-30 field (3648 bytes, already 4-byte aligned → `64 03` on both files)

  Type 12 compose, type-30 encode, native inflate, and `package` wiring stay **out of this experiment**.

## 33. SIS type 30 data (T2)

- **Requires:** experiment 15 (type 30 after type 3 in the outer type-12 field) and experiment 6 (`hello.exe`). Experiment **32** is reserved for type 34/35 checksums on another branch — do not record 32 here.
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** On frozen `$HOME/src/symdev-experiment-5/hello.sis`, walk type 30 after the compressed type-3 controller (do not re-run `makesis`). Walk nested TLVs. Compare the innermost data bytes to frozen `hello.exe`. Do not copy MakeSIS C. Do not invent C names. Do not commit `.sis` / `.exe`. Do not add a zlib crate. Do not encode types 34/35.
- **Expected result:** Pinned type-30 field 3648 bytes, payload `n=3640`: type-2 array of one type-31, type-2 array of one type-32, type-3 with algorithm `0` / uncompressed `3588` / reserved `0` then raw `hello.exe`.
- **Decision unblocked:** T2 `SisData` / `SisData31` / `SisData32` encode.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes) and `hello.exe` (3588 bytes). After UID, type 12 `n=3976`. Children: type 34 `n=2`, type 35 `n=2`, type 3 `n=295` at offset 24, type 30 `n=3640` at offset 328 occupying 3648 (`3640 % 4 == 0`). Nothing after type 30 (`328 + 3648 = 3976`). Nested walk of the type-30 **field** (header plus payload):

  | off | type | n | occupied |
  |-----|------|---|----------|
  | 0 | 30 | 3640 (`0x0e38`) | 3648 |
  | 8 | 2 | 3632 (`0x0e30`) | 3640 |
  | 16 | 31 | 3624 (`0x0e28`) | 3632 |
  | 24 | 2 | 3616 (`0x0e20`) | 3624 |
  | 32 | 32 | 3608 (`0x0e18`) | 3616 |
  | 40 | 3 | 3600 (`0x0e10`) | 3608 |

  Type-3 payload is the `SisCompressed` 12-byte prefix then 3588 data bytes: algorithm `0`, uncompressed size `3588` (`0x0e04`), reserved `0`, prefix `00 00 00 00 04 0e 00 00 00 00 00 00`. Data starts at field offset 60. Those 3588 bytes **equal** frozen `hello.exe` (E32 head `7a 00 00 10 00 00 00 00 f9 4c 9e e7 b0 08 32 c1`, tail `dc 95 8a 46 a2 45 c4 8c 39 38 35 bb 91 10 7f ff`). Host `hashlib.sha1` of that payload / `hello.exe`: `3a23e7e7e60ed97354534b2a77e565cd64ea3970` (same 20 bytes as experiment 29 type-25 digest). Not zlib (`78 9c` is absent; algorithm is 0, not 1). Inner type-2 payloads start with TLV headers `1f` / `20` (arrays of fields, not raw u32 `SisWords`). Headers:

  - type 30: `1e 00 00 00 38 0e 00 00`
  - type 31: `1f 00 00 00 28 0e 00 00`
  - type 32: `20 00 00 00 18 0e 00 00`
  - type 3: `03 00 00 00 10 0e 00 00`

  Checksums 34/35, outer type 12 compose, native inflate, and `package` wiring stay **out of this experiment**.

## 34. unsigned SIS file = UID + type 12 (T2)

- **Requires:** experiments 14–16 (UID, type-12 walk, type-3 zlib), 31 (type 13), 32 (checksums 34/35), 33 (type 30). Frozen experiment-7 `hello.sis`.
- **Skip if:** experiment-7 `hello.sis` is gone
- **Procedure:** Re-dump frozen `$HOME/src/symdev-experiment-5/hello.sis`. Confirm file = 16-byte UID + type-12 field whose payload is concatenated padded fields type 34, 35, 3, 30 in that order and nothing else. Inflate type-3 zlib and confirm it is the 548-byte type-13 field. Try host `zlib.compress` settings against the frozen 283-byte zlib stream. Do not copy MakeSIS C. Do not commit `.sis` / `.sisx`. Do not spawn Wine.
- **Expected result:** Pinned unsigned layout and whether a stock zlib setting byte-matches the type-3 stream.
- **Decision unblocked:** T2 `SisUnsigned` encode (native `signsis` still later).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes). SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232`. Layout:

  | off | what | n | occupied |
  |-----|------|---|----------|
  | 0 | `SisUid` | 16 | 16 |
  | 16 | type 12 | 3976 (`0x0f88`) | 3984 |
  | 24 | type 34 | 2 | 12 |
  | 36 | type 35 | 2 | 12 |
  | 48 | type 3 | 295 | 304 |
  | 352 | type 30 | 3640 | 3648 |

  Type-12 children occupied 12+12+304+3648 = 3976; `16 + 8 + 3976 = 4000`; nothing after type 30. Type-3 zlib (283 bytes, `78 9c` … `c4 37 2b 01`) inflates to the 548-byte type-13 field. Host Python `zlib.compress(inflated)` (default, level 6, wbits 15, `Z_DEFAULT_STRATEGY`) **equals** that 283-byte stream. Other levels did not: 0–5 and 7–9 differed (level 0/1 header `78 01`, 2–5 `78 5e`, 7–9 `78 da`). wbits `-15` / `31` did not match. Strategies other than default at level 6 were not a match in the sweep; default strategy + memlevel 8 or 9 at level 6 matched.

  In-crate `flate2` 1.1.10 `ZlibEncoder` + `Compression::new(6)`: default `rust_backend` (miniz_oxide) and `features = ["zlib-rs"]` produced different streams (type-34 became `3a 02` / `9b 1f`). `features = ["zlib"]` (system libz / `libz-sys`) byte-equals the frozen 283-byte stream and the 4000-byte file. Native `signsis`, SISX, and `package` wiring stay **out of this experiment**.

## 36. SISX signatures inside type 13 (T2)

- **Requires:** experiments 15–16 (type-12 children; type-3 prefix) and 31 (unsigned type-13 children). Frozen experiment-7 `hello.sis` and experiment-8 `hello.sisx`. Cert/key stay in the experiment dir; never committed.
- **Skip if:** those files are gone
- **Procedure:** Re-dump frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000) and `hello.sisx` (5172). Walk type-12 children. Inflate type-3 zlib with host Python `zlib` (not a repo crate). Walk inflated type-13 children. Compare extra SISX fields to `hello.cer` PEM-decoded DER. Do not copy SignSIS C. Do not invent argv. Do not spawn Wine. Do not commit `.sis` / `.sisx` / `.cer` / `.key`.
- **Expected result:** Where SISX gains bytes relative to SIS, and whether those bytes are TLVs that pin as `SisEncode` value types. If RSA/DSA or cert dates block a regenerating byte-match, record the signature/cert as opaque payloads.
- **Decision unblocked:** T2 `SisSignatures39` encode (type 13 insert / type 12 compose / native DSA still later).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes, SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232`) and `hello.sisx` (5172 bytes, SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea`). UID 16 bytes identical. After UID, both files are one type-12 field (SIS `n=3976`; SISX `n=5148`). Type-12 children on **both**:

  | off SIS | off SISX | type | n SIS / SISX | occupied SIS / SISX |
  |---------|----------|------|--------------|---------------------|
  | 0 | 0 | 34 | 2 / 2 | 12 / 12 |
  | 12 | 12 | 35 | 2 / 2 | 12 / 12 |
  | 24 | 24 | 3 | 295 / 1467 | 304 / 1476 |
  | 328 | 1500 | 30 | 3640 / 3640 | 3648 / 3648 |

  Nothing after type 30. Type 30 field bytes **equal**. Type 34 payload SIS `5c 9e`, SISX `01 c4` (type-3 field changed). Type 35 stays `64 03`. Type-3 prefix SISX `01 00 00 00 4c 07 00 00 00 00 00 00` (alg 1, uncompressed 1868). Extra 1172 bytes of the SISX file are entirely the larger type-3 field (`1476 - 304`).

  Inflated type 3 is one type-13 field: SIS `n=540` (548 bytes), SISX `n=1860` (1868 bytes). Type-13 **payload** prefix 528 bytes is identical (types 14, 16, 15, 17, 19, 28). SIS then has type 40 `n=4` payload `0`. SISX inserts type 39 `n=1312` occupied 1320 at payload off 528, then the same type 40. Extra uncompressed bytes: 1320.

  Nested walk of type 39 (header `27 00 00 00 20 05 00 00`):

  | off | type | n | occupied | notes |
  |-----|------|---|----------|-------|
  | 0 | 39 | 1312 | 1320 | |
  | 8 | 2 | 116 | 124 | array of one type 36 |
  | 16 | 36 | 108 | 116 | |
  | 24 | 38 | 44 | 52 | wraps type 1 UTF-16-LE `1.2.840.10040.4.3` (`n=34`) |
  | 76 | 37 | 48 | 56 | opaque DSA value; DER `30 2c` SEQUENCE of two 20-byte INTEGERs then two payload zeros |
  | 132 | 22 | 1180 | 1188 | wraps one type 37 |
  | 140 | 37 | 1171 | 1180 | one pad `00`; payload **equals** `hello.cer` PEM-decoded DER (1171 bytes; SHA-1 `6698f484c9c64d0ddf44240520f0e6bd629acfd9`) |

  Host `openssl x509 -inform DER` on that extracted type-37 payload (not SignSIS argv): `dsaWithSHA1`; issuer/subject `CN=Joe Bloggs, OU=Development, O=Acme Ltd, C=GB, emailAddress=noone@nowhere.com` (experiment-8 makekeys Example Usage); Not Before `Sep 17 15:21:21 2026 GMT`; Not After `Sep 14 15:21:21 2036 GMT`. Re-running makekeys would change those dates and the DER. `openssl dgst -sha1 -verify` of the DER signature against the unsigned type-13 field / type-13 payload did **not** verify. The signed-bytes rule is not derived here (do not copy SignSIS C). Encode type 37 as an opaque recorded blob.

  Type 13 compose with type 39, outer type 12, native DSA, and `package` wiring stay **out of this experiment**.

## 37. SISX file = signed type 13 inside `SisUnsigned` (T2)

- **Requires:** experiments 34 (`SisUnsigned` zlib type-12 wrap) and 36 (`SisSignatures39` / recorded type-39 blobs). Frozen experiment-8 `hello.sisx`.
- **Skip if:** experiment-8 `hello.sisx` is gone
- **Procedure:** Insert recorded type 39 into `SisController` immediately before type 40. Compress that type-13 field with the same `flate2`/`libz` level 6 as experiment 34. Wrap with existing `SisUnsigned` (live checksums 34/35 + type 3 + type 30). Compare `bytes()` to frozen `$HOME/src/symdev-experiment-5/hello.sisx`. Do not copy SignSIS C. Do not invent DSA. Do not spawn Wine. Do not commit `.sis` / `.sisx` / `.cer` / `.key`. Do not wire `package` / clap.
- **Expected result:** Whether zlib level 6 plus live checksums byte-match the 5172-byte SISX, and that unsigned `hello.sis` still matches when signatures are absent.
- **Decision unblocked:** T2 native SISX compose (native DSA / `package` wiring still later).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sisx` (5172 bytes). SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea`. `SisController::with_signatures` inserts the experiment-36 type-39 field (`n=1312`, occupied 1320) before type 40. Inflated type 13 is 1868 bytes (`n=1860`). Same `SisUnsigned` wrap as experiment 34: UID + type 12 (`n=5148`, header `0c 00 00 00 1c 14 00 00`) of live checksum 34 `01 c4`, checksum 35 `64 03` (unchanged type 30), type 3 (`n=1467`, occupied 1476), type 30 (identical 3648-byte field). `flate2` 1.1.10 `ZlibEncoder` + `Compression::new(6)` + `features = ["zlib"]` (system libz) byte-equals the frozen 5172-byte file. Unsigned `hello.sis` (4000 bytes) still matches with `signatures: None`. Native DSA, `package` wiring, and clap stay **out of this experiment**.

## 38. native unsigned SIS vs Wine makesis (T2 package)

- **Requires:** experiments 7 (Wine `makesis` frozen `hello.sis`) and 34 (`SisUnsigned` encode). Frozen experiment-7 `hello.sis` / `hello.pkg` / `hello.exe`.
- **Skip if:** those files are gone
- **Procedure:** Encode an unsigned SIS from the experiment-7 pkg fields (name `hello`, UID `0xe79e4cf9`, version `1,0,24`, vendor `Vendor` / `Vendor-EN`, platform `0x102752AE`, dest `!:\sys\bin\hello.exe`) plus live SHA-1 and bytes of frozen `hello.exe`, using existing `SisUnsigned` constructors. Compare to frozen Wine `makesis` `hello.sis`. Do not spawn Wine in this experiment. Do not commit `.sis` / `.sisx` / `.cer` / `.key`. Experiment 37 is owned elsewhere (SISX).
- **Expected result:** Native bytes equal the frozen 4000-byte Wine file, or a recorded diff of which derived fields still disagree.
- **Decision unblocked:** `symdev package` can write unsigned `.sis` without Wine `makesis`; Wine `signsis` / `makekeys` stay.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes, SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232`) equals in-crate `testdata/hello_sis.hex`. `encode_unsigned_sis` with the experiment-7 pkg fields, experiment-6 capability set (`LocalServices+NetworkServices+ReadUserData+WriteUserData+UserEnvironment+Location` → type 41 `0x000be000`), recorded TYPE=SA words (`0x21` / `0x14` / `0x0d` / `0x1a`), stamp `2026-08-17 15:18:24`, and live SHA-1 of `hello.exe` (`3a23e7e7e60ed97354534b2a77e565cd64ea3970`) **byte-equals** that Wine file. Re-running Wine `makesis` was not done (datetime would move).   A different name/UID/vendor does **not** emit the hello golden (UID prefix follows the project). Wine `signsis` / `makekeys` stay on `SisPackage`.

## 39. SignSIS signed-bytes (T2 native signsis)

- **Requires:** experiments 36–37 (type 39 / SISX compose) and frozen experiment-8 `hello.sis` / `hello.sisx` / `hello.cer` (public cert only; `.key` password stays local).
- **Skip if:** those files are gone
- **Procedure:** Extract the frozen type-36 DSA blob (48 bytes, DER `30 2c` + two zeros) and cert DER from `hello.sisx`. DSA-SHA1-verify candidate hashes against the cert public key. Do not copy SignSIS C. Do not spawn Wine. Do not commit `.sis` / `.sisx` / `.cer` / `.key`.
- **Expected result:** Which bytes SignSIS signs, recorded failures, and whether a live SHA-1 + DSA signature can replace Wine `signsis` on `SisPackage`.
- **Decision unblocked:** native SISX sign in `package` (Wine `makekeys` may remain).
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `hello.sisx` SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea`. Type-36 blob 48 bytes DSA (`r=0x4de0…aea0`, `s=0x5020…a8c0`). Cert self-signature verifies (pubkey works).

  **Hit:** SHA-1 of uncompressed type-13 **payload without type 39 and without type 40** (hello: 528 bytes = types 14+16+15+17+19+28 field concat). Digest `f3fca5ab077413d247c256bb81824a053b221caa`. Standard DSA-SHA1 (hash integer big-endian). Type 37 payload is DER SEQUENCE of two INTEGERs, padded to 4 bytes.

  **Failed (did not verify):** unsigned type-13 field (548, includes header + type 40); unsigned type-13 payload (540, includes type 40); SISX type-13 field/payload; SISX header+unsigned payload; SISX with type 39 or sig blob zeroed; compressed type 3 field/payload/zlib (unsigned and SISX); type 12 field/payload ± checksums; file after UID; whole `.sis`/`.sisx`; UID; exe; type 30; SHA-1-of-those as the DSA message (double-hash); SHA-256 left-160; little-endian hash integer; every prefix/suffix of those buffers other than the 528-byte payload prefix.

  Live sign: SHA-1 + DSA via RustCrypto (`sha1` 0.10, `dsa` 0.6 RFC 6979 k) over `SisController::signed_bytes()`, cert DER in type 22, OID `1.2.840.10040.4.3`. Frozen type-39 blob still pins `hello.sisx` (recorded). Live sign of the same controller does **not** byte-equal frozen `hello.sisx` (SignSIS random k ≠ RFC 6979). Dates in new makekeys certs also block equality. `SisPackage` writes SISX with `with_signatures` + live key; Wine `signsis` is not spawned. Wine `makekeys` remains when cert/key are absent.

## 40. native makekeys vs frozen hello.cer (T2)

- **Requires:** experiment 8 (Wine `makekeys` frozen `hello.cer` / `hello.key`) and experiment 39 (native `encode_signed_sisx` already reads PEM/DER cert + PKCS#8 / traditional / encrypted-traditional DSA keys). Frozen `$HOME/src/symdev-experiment-5/hello.cer` / `hello.key`.
- **Skip if:** those files are gone
- **Procedure:** Dump hello.cer SPKI, DSA params, subject, validity (openssl; do not assume RSA). Record Wine `makekeys` argv already in tree (`SisTools::makekeys_args` / experiment 8). Generate a self-signed cert+key in Rust (RustCrypto) with injected Not Before `2026-09-17 15:21:21 GMT` and `-expdays 3650`. Compare to frozen hello.cer. Confirm native sign of the hello controller hash verifies with the generated key. Do not copy makekeys C. Do not invent argv. Do not spawn Wine in default tests. Do not commit `.cer` / `.key` / `.sis` / `.sisx`.
- **Expected result:** Byte-match hello.cer if possible; otherwise a verifiable self-signed DSA-SHA1 cert whose subject matches the recorded makekeys DN, plus a key format `encode_signed_sisx` already reads.
- **Decision unblocked:** `SisPackage` can write cert/key without Wine `makekeys`; `package` does not need Wine/`SYMDEV_EPOCROOT`.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Frozen `$HOME/src/symdev-experiment-5/hello.cer` (PEM 1643 bytes, DER 1171, SHA-256 `08818e08330d77a3e53dc7814082a9906ab7711c737d21aa09126f6b319c6f5e`, SHA-1 fingerprint `6698f484c9c64d0ddf44240520f0e6bd629acfd9` matching experiment 36). **Algorithm is DSA, not RSA.** `openssl x509 -text` / `asn1parse`:

  | Field | Frozen hello.cer |
  |-------|------------------|
  | Version | 3 (`INTEGER` 2), **no extensions** |
  | Serial | 1 |
  | Signature | `dsaWithSHA1` (`1.2.840.10040.4.3`), OID only (no NULL parameters) |
  | Issuer = Subject | `CN=Joe Bloggs, OU=Development, O=Acme Ltd, C=GB, emailAddress=noone@nowhere.com` (CN/OU/O/C `PRINTABLESTRING`; email `IA5STRING`) |
  | Validity | UTCTime `260917152121Z` → `360914152121Z` (Not Before 2026-09-17 15:21:21 GMT, Not After 2036-09-14 15:21:21 GMT = `+ 3650` days) |
  | SPKI | `dsaEncryption`; **p 2048-bit**, **q 160-bit** (`INTEGER` 21 bytes `FD5AFC…BD15`), g 2048-bit |

  Frozen `hello.key` (1264 bytes): `BEGIN DSA PRIVATE KEY` / `Proc-Type: 4,ENCRYPTED` / `DEK-Info: DES-EDE3-CBC,0A4E5E4AEE811DAD`. Password stays local.

  Wine argv already in tree (`makekeys_args_match_experiment_8`; same as experiment 8, password redacted):

  ```
  /usr/bin/wine /sdk/epoc32/tools/makekeys.exe \
    -cert -expdays 3650 -password <pw> -len 2048 \
    -dname "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com" \
    hello.key hello.cer
  ```

  Native generate (injected Not Before 2026-09-17 15:21:21 GMT, serial 1, recorded DN, `dsaWithSHA1`): **does not byte-equal** frozen `hello.cer` (new DSA params, RFC 6979 cert `k`, and `dsa` 0.6 has no 2048/160 `KeySize`; debug `DSA_2048_256` keygen ~192s so production uses `DSA_1024_160`). Self-signature verifies. Native `encode_signed_sisx` of the hello controller with the generated PKCS#8 key verifies. Subject is the recorded makekeys Example Usage DN (not the SIS pkg vendor string `Vendor`). Key file is unencrypted PKCS#8 `BEGIN PRIVATE KEY` — Wine `.key` encrypted traditional PEM is **not** re-emitted; native sign already loads both. `SisPackage::package` writes `<name>.cer`/`<name>.key` this way when `signing.cert`/`signing.key` are absent (same overwrite as `std::fs::write` / Wine makekeys). CLI `package` no longer calls `SisTools::from_env`. Default `cargo test` does not spawn Wine.

## 41. Wine `rcomp` goldens + native RSC UID header (T3)

- **Requires:** experiment 9 (Wine `rcomp` usage + `cpp.exe`/`rcomp.exe` argv) and a legal-access FP2 SDK. Experiment **40** is reserved for makekeys (independent branch).
- **Skip if:** no SDK (`rcomp.exe` missing)
- **Procedure:** Dump `rcomp.exe` no-arg usage (do not invent argv). Re-run experiment-9 Wine `cpp.exe` then `rcomp.exe` on the already-copied SDK example `_reg.rss` files (`driveinfo_reg.rss`, `filebrowseapp_reg.rss`) in a workdir outside git. From the usage string, also pass glued `-hHeaderFile`. Pin `.rsc` / `.rsg` bytes. Native slice: first 16 bytes of `.rsc` via existing `UidCrc` (do not copy rcomp C). Do not spawn Wine in `cargo test`. Do not commit SDK / `.rss` / `.rpp` / `.rsc` binaries.
- **Expected result:** Recorded usage; bit-identical `.rsc` vs experiment 9; whether `-h` emits a `.rsg`; whether the 16-byte UID prefix is `UidCrc`.
- **Decision unblocked:** T3 first native encode (`RscUid` / `RcompTool` argv). Full rcomp / RSS parse / `START RESOURCE` wiring stay later.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-41/` (outside git). SDK `rcomp.exe` `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/rcomp.exe` (188416; PE32 i386). `/usr/bin/wine` `wine-10.0 (Ubuntu 10.0~repack-12ubuntu1)`. Log `$HOME/src/symdev-experiment-41/experiment-41.log`.

  **Usage** (no-arg; tool exit 255; stdout empty; stderr after Wine `experimental wow64 mode`):

```
Resource compiler version 8.1 (Build 004) (C) 1997-2005 Symbian Software Ltd.
Usage: rcomp [-vpul] [-force] [-oRSCFile] [-{uid2,uid3}] [-hHeaderFile] [-sSourceFile] [-iBaseInputFileName]
	v	verbose
	p	Parser debugging
	l	Check localisation comments
	force	Emit localisation warnings even if no localisation tags are present
	add-defaults	Amend input rss/rpp file to add missing default localisation options

	u	Generate Unicode resource binary
```

  **(a) `driveinfo_reg`:** same experiment-9 `cpp.exe` then `rcomp.exe` argv (`WINEPATH` = SDK tools so sibling `uidcrc.exe` is found). Exit 0. `driveinfo_reg.rsc` 74 bytes, SHA-256 `10bd8e607b9f166629ac1e9285d6abbad24e88972a885a15062f8680d575b7d8`, **byte-equal** experiment-9. First 16 bytes `6b 4a 1f 10 21 80 1f 10 f4 01 00 a0 b4 0c c8 f0` = `UidCrc` of UID1 `0x101f4a6b`, UID2 `0x101f8021`, UID3 `0xa00001f4`, checked `0xf0c80cb4`.

  **(b) `-h` from usage:** `-hdriveinfo_reg.rsg` after `-o` (glued, like `-o`/`-s`/`-i`). Exit 0. `.rsc` still the same 74-byte file. `.rsg` **0 bytes** (SHA-256 of empty `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`). Registration RSS has no `NAME`; rcomp writes an empty header. Not committed.

  **(c) `filebrowseapp_reg`:** same path. `.rsc` 109 bytes, SHA-256 `437dbc17eef7b26d9650917b408d22922f96e7ed888e2916035f56eb010f0b60`, byte-equal experiment-9. Header `6b 4a 1f 10 21 80 1f 10 a6 00 00 e8 69 64 35 0a` = same UID1/UID2, UID3 `0xe80000a6`, checked `0x0a356469`. `.rsg` also 0 bytes.

  Recorded working argv (flags from usage / experiment 9; `WINEPATH` is Wine lookup for `uidcrc.exe`, not an rcomp flag):

```
WINEPATH=/home/genius/sdk/S60_3rd_FP2/epoc32/tools \
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/gcc/bin/cpp.exe \
  -nostdinc -undef -C -D_UNICODE \
  -I 'Z:\home\genius\sdk\S60_3rd_FP2\epoc32\include' \
  driveinfo_reg.rss -o driveinfo_reg.rpp
```

```
WINEPATH=/home/genius/sdk/S60_3rd_FP2/epoc32/tools \
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/rcomp.exe \
  -u -odriveinfo_reg.rsc -sdriveinfo_reg.rpp -idriveinfo_reg.rss
```

```
WINEPATH=/home/genius/sdk/S60_3rd_FP2/epoc32/tools \
/usr/bin/wine /home/genius/sdk/S60_3rd_FP2/epoc32/tools/rcomp.exe \
  -u -odriveinfo_reg.rsc -hdriveinfo_reg.rsg -sdriveinfo_reg.rpp -idriveinfo_reg.rss
```

  `symdev` still does **not** spawn `rcomp`: MMP keeps `START RESOURCE` inner lines only (rss filename dropped); `GcceBuild` / CLI have no rcomp verb or path. Native body/index after the 16-byte UID, RSS tokens, and `package`/`build` wiring stay **out of this experiment**.

## 42. Unicode RSC body + index after the UID header (T3)

- **Requires:** experiment 41 (frozen `_reg.rsc` goldens + `RscUid`) and a legal-access FP2 SDK.
- **Skip if:** experiment-41 `.rsc` files are gone
- **Procedure:** Hex-dump the bytes after the 16-byte UID on frozen `driveinfo_reg.rsc` (74) and `filebrowseapp_reg.rsc` (109). Run recorded experiment-9/41 Wine `rcomp.exe -u -v` on the existing `.rpp` (verbose is in the usage string). From SDK `epoccnf.pl` / `epocaif.pl`, the observed `:-dump_prefix` token dumps each resource uncompressed and unpadded; use that only to record layout, do not add it to `RcompTool`. Reconstruct `APP_REGISTRATION_INFO` (`LONG`/`LLINK`/`LText16`/`BYTE`/`LEN WORD STRUCT[]`) and the packed form that byte-equals the goldens. Do not copy rcomp C. Do not spawn Wine in `cargo test`. Do not commit `.rsc` / `.rss` / `.rpp` binaries.
- **Expected result:** Pinned body layout after the UID: 4-byte size/flags header, packed resource, trailing `u16` index. Native encode of both goldens.
- **Decision unblocked:** T3 native RSC body+index encode (`Rsc` / `RscAppRegistration` / `RscLtext16`). RSS parse, non-empty `.rsg`, and `START RESOURCE` wiring stay later.
- **Outcome:** pass
- **Evidence:** 2026-09-18, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-42/` (outside git). Same Wine `rcomp.exe` 8.1 as experiment 41. Frozen `.rsc` SHA-256 unchanged from experiment 41.

  **Verbose** (`-v` from usage) prints `LText16` (not `LTEXT`) and `IndexTable` / `IndexTableItem 14` then `IndexTable 46` for driveinfo (file offsets 20 and 70). Filebrowse index end is 105 (`0x69`).

  **Uncompressed dump** (`:-_dump_of_resource_` from SDK perl, not `RcompTool`): driveinfo 46 bytes 8-bit `LText16` (`0c` + `DriveInfoApp`); filebrowse 79 bytes. Unicode `-u` pads each non-empty `LText16` to `len`, `0x00`, UTF-16LE (`DriveInfoApp` → 59 bytes uncompressed; filebrowse loc path → 126). Those sizes are byte 17 of the `.rsc` (`0x3b` / `0x7e`). Empty `LText16` stays one `0x00`. `LLINK` is 4 bytes. `LEN WORD STRUCT[]` omitted members are `u16` 0.

  **File layout** (little-endian index; first resource always at offset 20):

  | Region | Driveinfo | Filebrowse |
  |---|---|---|
  | UID (`RscUid`) | 16 bytes | 16 bytes |
  | Header | `00 3b 00 01` | `00 7e 00 01` |
  | Packed resource | offset 20..70 (50 bytes) | 20..105 (85 bytes) |
  | Index (`u16` start, end) | `14 00 46 00` | `14 00 69 00` |

  Header byte 1 is the largest uncompressed resource size as `u8` (59 / 126). Bytes `00 01` are a constant on these two single-resource Unicode files (could be flags `0x0100` LE or count 1; not distinguished here). Packed form starts with `0x00`, then runs of `(u8 count, literal bytes)` with `LText16` Latin-1 stored as duplicate length plus 8-bit chars (`0c 0c DriveInfoApp`). Length byte sits in the preceding literal run; the UTF-16 pad `0x00` is omitted in the packed form.

  Native `Rsc::bytes()` of `RscAppRegistration` **byte-equals** both goldens. `.rsg` / RSS parse / `START RESOURCE` stay **out of this experiment**. `:-` is not on `RcompTool`.



## 43. Two-file SIS: hello EXE + `_reg.rsc` (T2/T3)

- **Requires:** experiments 7 (hello `.pkg` + Wine `makesis`) and 42 (native `Rsc` byte-equal to Wine `rcomp`).
- **Skip if:** experiment-5 `hello.exe` or the FP2 SDK is gone
- **Procedure:** Copy experiment-9 `driveinfo_reg.rss`, substitute only `UID3 0xE79E4CF9` and `app_file="hello"`. Run the recorded experiment-9 Wine `cpp.exe` + `rcomp.exe -u` argv. Append to the experiment-7 `.pkg` one `_reg.rsc` line in the SDK example's form (`locationsatviewrefapp_armv5.pkg`), dest `!:\private\10003a3f\import\apps\hello_reg.rsc`. Run recorded experiment-7 `makesis.exe -v hello.pkg hello.sis`. Compare against native `Rsc::registration` and `SisUnsigned::encode` with the SIS datetime read back from the Wine controller.
- **Expected result:** Recorded bytes for a SIS carrying a non-EXE file; tells whether native encode needs per-file rules.
- **Decision unblocked:** `symdev package` shipping `_reg.rsc`; `Makesis` reading the pkg's `_reg.rsc` line.
- **Outcome:** pass
- **Evidence:** 2026-09-19, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-43/` (outside git). `hello_reg.rsc` 67 bytes (SHA-256 `c9d15ec1…4367`); rcomp warns only about unused `datatype_list` / `file_ownership_list` / `service_list`. `makesis` exit 0; `hello.sis` 4172 bytes (SHA-256 `442abbf2…947e`). Controller datetime `2026-(8)-19 09:02:53`.

  Native `Rsc::registration(0xe79e4cf9, "hello")` **byte-equals** the Wine `_reg.rsc` (same shape as `driveinfo_reg`: empty `localisable_resource_file`, so no `_loc` / caption `.rsc` is needed).

  Wine `makesis` differs from a naive second `SisFile` in two recorded ways:

  | Field | EXE | `_reg.rsc` |
  |---|---|---|
  | Type-41 capabilities field | present (`0x000be000`) | **absent** |
  | Data blob | stored, algorithm 0 (zlib 3599 ≥ 3588) | **zlib**, algorithm 1 (50 < 67) |
  | Descriptor length / uncompressed | 3588 / 3588 | **50** / 67 |

  Rule: each file is zlib-compressed (level 6) and kept compressed only when smaller (`SisCompressed::smallest`); capabilities only on the EXE. With those, native `SisUnsigned::encode` **byte-equals** the Wine `hello.sis` (goldens `hello_reg.rsc.hex`, `hello_reg_sis.hex`). The multi-element `SisArray` (element type once) is pinned by this golden too.

## 44. Uncompressed E32 from the experiment-6 argv (T4)

- **Requires:** experiment 6 (`hello.elf`, Linux `elf2e32_next`, FP2 `--libpath`).
- **Skip if:** experiment-5 `hello.elf` or `elf2e32_next` is gone
- **Procedure:** Rerun the recorded experiment-6 argv verbatim (output `hello.exe`), then once more adding only `--uncompressed` (observed in `elf2e32 --help`: "Don't compress output e32image"; output `hello_u.exe`). Compare both with the frozen experiment-6 `hello.exe`. Decode the import section and code relocations of `hello_u.exe`.
- **Expected result:** An uncompressed golden of the same image, so post-header sections can be pinned without writing a Symbian inflater.
- **Decision unblocked:** T4 import section, `iDllRefTableCount`, `iCodeRelocOffset`; next: code relocations and code words.
- **Outcome:** pass
- **Evidence:** 2026-09-19, Ubuntu 26.04.1 LTS x86_64. Workdir `$HOME/src/symdev-experiment-44/` (outside git). Both runs exit 0.

  Rerun `hello.exe` is 3588 bytes and differs from the frozen one only at `iHeaderCrc` (0x14) and `iTimeLo`/`iTimeHi` (0x24–0x28): `elf2e32_next` is deterministic apart from time. `hello_u.exe` is 5652 bytes (`0x9c + J.iUncompressedSize 0x1578`) and differs from the compressed header only at `iHeaderCrc` and `iCompressionType` (0). `hello_u.exe` SHA-256 `8f4f91db…d5d0` (fixture `hello_uncompressed.exe.hex`).

  **Import section** (`0x14e8..0x15b8`, `KImageImpFmt_ELF`): `u32 size 0xd0`; per DLL `u32 name offset, u32 count, count × u32 code offset`; names NUL-terminated, padded to 4. Two DLLs, `drtaeabi{000a0000}.dll` (14 slots) then `euser{000a0000}[100039e5].dll` (19). Entries are code offsets of import slots, **not ordinals** (ordinals live in the code words). They are exactly the `DT_REL`/`DT_JMPREL` relocations against undefined `.dynsym` symbols, grouped by the `.gnu.version_r` name of the symbol's version, in `.gnu.version_r` order. `DT_RELSZ` (520) spans `.rel.dyn` + `.rel.plt` + `.rel.other`; each entry counts once. Six `DT_NEEDED` DSOs, but only these two are versioned imports → `iDllRefTableCount = 2`.

  **Code relocations** (`0x15b8` to end of file): `u32 size 0x54` (blocks only), `u32 count 32` (real entries), then per 4 KiB page `u32 page, u32 block size` and `u16` entries `kind << 12 | offset`, padded to 4 with a zero entry. Pages `0x0` (23) and `0x1000` (9), sorted by offset. They are exactly the local dynamic relocations (`R_ARM_RELATIVE` ×31, `R_ARM_ABS32` ×1) against defined symbols; kind 1 (text) when the symbol lies in the code segment, 2 (data) for the one into `.bss`. Native `E32RelocSection::code_from_elf` **byte-equals** it.

  **Code section** (`0x9c..0x14e8`): the ELF executable segment with 34 words rewritten. 33 import slots become `addend << 16 | ordinal` (32 with addend 0; one `R_ARM_ABS32` import of `_ZTVN10__cxxabiv117__class_type_infoE` with addend 8 → `0x8007b`). The one local `R_ARM_ABS32` (0x1298) becomes `S + A` (0x9280). `R_ARM_RELATIVE` words stay as linked. Ordinal = the word a DSO `.dynsym` symbol's value points at in `ER_RO` (equals `value / 4 + 1` on all 33). The 33 used ordinals are recorded in `hello_ordinals.txt`; `Elf2E32::ordinals` reads them from `--libpath` DSOs and matches that table (test runs only when `SYMDEV_EPOCROOT` is set). Native `E32CodeSection::from_elf` **byte-equals** the code section.

  Native `E32ImportSection::from_elf` **byte-equals** the import section; the golden header builds `iDllRefTableCount` and `iCodeRelocOffset` from it.

## 45. Native `elf2e32 --uncompressed` hello in EKA2L1 (T4)

- **Requires:** experiments 43 (native two-file SIS), 44 (uncompressed golden), patched EKA2L1 with the RM-469 firmware (see [eka2l1-bringup.md](eka2l1-bringup.md)).
- **Skip if:** no FP2 `--libpath` DSOs or no EKA2L1
- **Procedure:** Run the native `symdev-elf2e32` bin with the experiment-44 argv (experiment-6 argv + `--uncompressed`, real `--libpath`). Diff against `elf2e32_next`'s `hello_u.exe`. Package the result with the native `makesis` bin on the experiment-43 `.pkg` (EXE + `_reg.rsc`), `eka2l1_qt --install`, then `eka2l1_qt --run hello`.
- **Expected result:** A hello built without `elf2e32_next` installs and runs.
- **Decision unblocked:** native elf2e32 is usable for EXE output; only the default deflate compression is still missing.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19. Native `native_u.exe` 5652 bytes; differs from `elf2e32_next` `hello_u.exe` only at `iHeaderCrc` and `iTimeLo` (live time). Workdir `$HOME/src/symdev-experiment-45/`: native `hello.sis` 4108 bytes. EKA2L1 log `Installation done!`; installed `E:\sys\bin\hello.exe` is byte-identical to the native image; applist `Found app: hello, uid: 0xE79E4CF9`; launch loads `econs`; screen shows `Hello, world!` / `[press any key]` (pixel check, no grid).

## 46. Native compressed E32 (clean-room deflate) in EKA2L1 (T4)

- **Requires:** experiment 45; `docs/research/e32-deflate-spec.md`.
- **Skip if:** no FP2 `--libpath` DSOs or no EKA2L1
- **Procedure:** Clean-room in two roles: a separate agent read elf2e32_next (EPL-1.0) and wrote only a behavioural spec (`e32-deflate-spec.md`, verified by its own throwaway decoder/encoder written from the spec). `E32Deflate` was then implemented from that document alone, without reading the source. Run the native `elf2e32` bin with the verbatim experiment-6 argv (compressed default), diff against the frozen experiment-6 `hello.exe`, package with native `makesis` (EXE + `_reg.rsc`), install and run in EKA2L1.
- **Expected result:** Native default output equals elf2e32_next apart from CRC/time, and runs.
- **Decision unblocked:** `symdev build` can drop `SYMDEV_ELF2E32` for EXEs.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19. `E32Deflate::compress` of the 5496-byte body byte-equals the 3432-byte experiment-6 stream; with the frozen time, native `encode_elf` reproduces the whole 3588-byte frozen `hello.exe`. Live run: 3588 bytes, differs only at `iHeaderCrc` and `iTimeLo`/`iTimeHi` (0x24–0x28). Workdir `$HOME/src/symdev-experiment-46/`: native `hello.sis` 4172 bytes; EKA2L1 `Installation done!`; installed `E:\sys\bin\hello.exe` byte-identical; `--run 0xe79e4cf9` shows `Hello, world!` / `[press any key]`.

## 47. `symdev build` with native post-link (examples/hello)

- **Requires:** experiment 46.
- **Procedure:** Copy `examples/hello`; `symdev build` with the M1 toolchain env **without** `SYMDEV_ELF2E32`, then `symdev package`, `eka2l1_qt --install build/hello.sisx`, `eka2l1_qt --run 0xef9f2cab`. Compare `build/hello.exe` with the same project built with `SYMDEV_ELF2E32=elf2e32_next`.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19. Native and external `hello.exe` are both 3588 bytes and differ only at `iHeaderCrc` (0x14–0x17) and `iTimeLo` (0x24–0x27). Installed `E:\sys\bin\hello.exe` byte-identical to the native build; screen shows `Hello, world!` / `[press any key]`.

## 48. `symdev run`: one EKA2L1 invocation installs and launches

- **Requires:** experiment 47; patched EKA2L1 ([eka2l1-bringup.md](eka2l1-bringup.md)).
- **Procedure:** Read EKA2L1's own CLI option table (`--install, -i` "Install a SIS."; `--run, -a, --app` "Run an app with given name or UID"). `eka2l1_qt --install <sisx> --run 0x<uid3>` in one process. Then `symdev run` in `examples/hello` with `SYMDEV_EKA2L1` pointing at a host wrapper that sets the software-GL environment.
- **Outcome:** pass (emulator only)
- **Evidence:** 2026-09-19. Upstream EKA2L1 `--install` reported `Installation of SIS failed` right after `Installation done!` and quit: the CLI handler stored the `installation_result` enum in a `bool`, and success is `0`. Patched to compare with `installation_result_success` (in `~/src/EKA2L1-econs-heap.patch`). With that, one invocation installs to E: and launches; `symdev run` starts it in its own process group, logs to `build/eka2l1.log`, exits 0; log shows `Installation done!`, `Found app: hello, uid: 0xEF9F2CAB`, `Loaded library: econs`; screen shows `Hello, world!`.

  **Correction (2026-09-19, later the same day):** the apps used above (`hello` 0xE79E4CF9, `examples/hello` 0xEF9F2CAB) had already been installed by earlier separate runs, and `E:\resource\apps\hello.rsc` (a caption resource an earlier session installed) was still on the virtual E:. A **brand-new** app failed: `App with UID ... doesn't exist`, even on a cold boot. Two more upstream EKA2L1 bugs, fixed on `symdev-fixes`: (1) the CLI `--install` never rescans app registrations; (2) `applist_server::load_registry` returned before committing a registration whose localisable resource file is missing, so any `_reg.rsc` with an empty `localisable_resource_file` (the SDK `driveinfo_reg` form symdev writes) was dropped. With both, a new project (`fresh2`, 0xE42E4CA7) installs and launches in one `symdev run`. Also: screenshots are now bound to the emulator PID (`_NET_WM_PID`); earlier, largest-window selection could have captured another open EKA2L1 window.

## 49. E32 data section: `counter` with writable static data (T4)

- **Requires:** experiment 47.
- **Procedure:** `symdev new counter`, then add three globals with external linkage: `TInt gCounter = 41;`, `const TText16* gGreeting = L"counter";`, `TInt* gCounterPtr = &gCounter;` and print them after `++*gCounterPtr`. `symdev build` with `SYMDEV_ELF2E32=elf2e32_next`; run `elf2e32_next` with the same argv to get `counter.exe` and `--uncompressed` `counter_u.exe`. Compare native `symdev-elf2e32`; then build natively, package, `symdev run`.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19. Workdir `$HOME/src/symdev-experiment-49/`. ELF: RW `PT_LOAD` 0x400000, filesz 0xc, memsz 0x10; data relocations `R_ARM_ABS32` at 0x400000 (→ `gCounter` 0x400004) and `R_ARM_RELATIVE` at 0x400008 (→ `.text`). E32: `iDataOffset` = `0x9c + iCodeSize` (0x153c), data stored after code and before imports, BSS 4 not stored; data words: `R_ARM_ABS32` becomes `S + A` (0 → 0x400004), `R_ARM_RELATIVE` unchanged; data relocation section follows the code relocations (`iDataRelocOffset` 0x1678), same format, offsets relative to the data base: `0x2000` (data kind, offset 0) and `0x1008` (text kind, offset 8), count 2. Native uncompressed (5772 bytes) and compressed (3687) differ from `elf2e32_next` only at `iHeaderCrc` and time; with the golden time both byte-equal (fixtures `counter*.hex`). Natively built and run: screen shows `Hello, world!` / `counter=42` / `[press any key]` (PID-bound screenshot).


## 50. Capability bits for every name (E32 `iCaps` and SIS type 41)

- **Procedure:** For each of the 20 capability names, run `elf2e32_next` with the experiment-6 argv, `--capability=<name>`, `--uncompressed`, and read `iCaps` (V header, u64 at 0x88). Then build an EXE with `--capability=TCB+AllFiles+ReadDeviceData+UserEnvironment`, run Wine `makesis -v` on the experiment-7 `.pkg`, and read the SIS type-41 word (makesis takes capabilities from the E32).
- **Outcome:** pass
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-50/caps.txt`. Bits: TCB 0, CommDD 1, PowerMgmt 2, MultimediaDD 3, ReadDeviceData 4, WriteDeviceData 5, DRM 6, TrustedUI 7, ProtServ 8, DiskAdmin 9, NetworkControl 10, AllFiles 11, SwEvent 12, NetworkServices 13, LocalServices 14, ReadUserData 15, WriteUserData 16, Location 17, SurroundingsDD 18, UserEnvironment 19. The mixed set gives `iCaps` and SIS type 41 both `0x80811`. `symdev_core::Capabilities` now maps all 20; the manifest still rejects non-user-grantable names for self-signed packages.

## 51. Avkon GUI app end to end (resources, LIBRARY, native elf2e32, three-file SIS)

- **Requires:** experiments 9, 44–50.
- **Procedure:** Author a minimal Avkon app (own code, not an SDK example) with `gui.rss` (`EIK_APP_INFO`, `LOCALISABLE_APP_INFO`) and `gui_reg.rss`. Compile resources with the recorded experiment-9 `cpp.exe` + `rcomp.exe` (`-h` for the `.rsg`). Compile C++ with the GcceBuild flags, link with `euser apparc cone eikcore avkon gdi` DSOs, post-link with native and `elf2e32_next` (compressed and `--uncompressed`), package three files with Wine `makesis`; then the same through `symdev new --template gui`, `build`, `package`, `run`.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-51/`.
  - **Host headers:** the FP2 SDK includes headers with the wrong case (`fbs.h` → `FbsMessage.h`; 260 such names). symdev builds a symlink overlay (`build/sdk-include-casefold`, `SdkIncludeCaseFold`) searched after `epoc32/include`. GCC 12 rejects SDK headers (extra member qualification in `openfont.h`/`w32std.h`, narrowing UID initialisers): compile adds `-fpermissive -Wno-narrowing`; `examples/hello` E32 is unchanged by this (differs only in CRC/time).
  - **elf2e32 (observed on `gui.elf`, 8064-byte uncompressed image):** `R_ARM_GLOB_DAT` against a defined symbol becomes `S` plus a text relocation; import DLL blocks are sorted by DLL name (not `.gnu.version_r` order; hello's happened to be sorted); an `R_ARM_ABS32` to undefined `__cxa_pure_virtual` is **not** imported — the word stays and gets an inferred (kind 3) code relocation. Every other undefined ABS32 (vtables, typeinfo, methods) is imported. Native output then equals elf2e32_next apart from CRC/time, compressed (4990) and uncompressed.
  - **SIS:** Wine makesis keeps `.pkg` file order (exe, `\resource\apps\gui.rsc`, `import\apps\gui_reg.rsc`) and writes **no** type-41 field for an EXE without capabilities. Native `SisUnsigned::encode` with a file list byte-equals the 5736-byte `gui.sis`.
  - **End to end:** `symdev new notes --template gui` → `build` → `package` (three-file `.pkg`) → `run`: EKA2L1 `Found app: notes`; screen (PID-bound capture) shows title `notes`, centered `Hello from symdev`, softkey `Exit`.


## 52. DLL: E32 image, `.def` and `.dso` (T4)

- **Requires:** experiment 51. SDK GCCE recipe read from `epoc32/tools/cl_bpabi.pm` (entry `_E32Dll`, `edll.lib`, `--targettype=DLL`, `--dso`, `--defoutput`, `--definput` when a `.def` exists else `--ignorenoncallable`).
- **Procedure:** Build `mathlib` (three `EXPORT_C` functions) with `-D__DLL__`, link with `edll.lib`, `--entry _E32Dll`, soname `mathlib{000a0000}[e5d1b001].dll`; run `elf2e32_next --sid --uid1=0x10000079 --uid2=0x1000008d --uid3 --targettype=DLL --ignorenoncallable --dso --defoutput` (compressed and `--uncompressed`). Also DLLs with 1, 4, 5, 6, 8, 10, 13, 17, 25, 40 exports for the `.dso` hash.
- **Outcome:** pass
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-52/`.
  - **Image:** export directory appended to the code: `u32 count`, then each export's link address (Thumb bit kept); `iExportDirOffset` points at the first address; `iCodeSize`/`iTextSize` include it; every address slot gets a text relocation. Flags `0x1200002b` (EXE flags + DLL bit), `iEntryPoint` = `_E32Dll`, V export description type 0. Ordinals follow **symbol-name order** (`_Z7MathAbsi` is ordinal 1 despite the highest address). `--ignorenoncallable` ("Generate exports for functions only") skips linker symbols such as `_edata`/`__bss_start`. *Correction (experiment 54): elf2e32_next skips those without the flag too; the flag changed nothing observed.*
  - **`.def`:** `EXPORTS`, `; NEW:`, `\t<name> @ <ordinal> NONAME` lines, trailing blank line.
  - **`.dso`:** ELF32 ARM `ET_DYN`, flags `0x04000004`; sections `ER_RO` (ordinal words 1..n + a zero word), `.dynamic` (SONAME = the `--dso` file name, SYMTAB, SYMENT, STRTAB, STRSZ, VERSYM, VERDEF, VERDEFNUM=2, HASH, NULL), `.hash` (`nbucket = N/3 + N%3`, N = n+1; clean-room spec `dso-hash-spec.md`), `.version_d` (base = soname, 2 = linkas), `.version`, `.strtab` (exports, soname, linkas; zero-padded to 4), `.dynsym` (value 4·i, size 4, GLOBAL FUNC, section 1), `.shstrtab`; sections 4-aligned after the headers; program headers last (LOAD flags `0x80000001`, DYNAMIC).
  - Native `symdev-elf2e32` output equals elf2e32_next for the DLL (apart from CRC/time) and **byte-equals** every `.def` and `.dso` (11 DSOs).

## 53. `symdev build` with a project DLL (EXE + own DLL)

- **Requires:** experiment 52.
- **Procedure:** `symdev new calc`; add `group/mathlib.mmp` (`TARGETTYPE DLL`, `UID 0x1000008d 0xe5d1b001`) before `calc.mmp` in `bld.inf`; `calc.mmp` gets `LIBRARY mathlib.lib`; `hello.cpp` prints `MathTwice(21)` and `MathAbs(-5)`. `symdev build` (native post-link), `symdev package`, `symdev run`. Then set `capabilities = ["ReadUserData"]`, rebuild, and compare the SIS with Wine `makesis` on the generated `.pkg`.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-19. Build writes `mathlib.dll`, `mathlib.dso`, `mathlib.def` (DLL recipe from `cl_bpabi.pm`: `-D__DLL__`, `edll.lib`, `_E32Dll`, `--libpath=<sdk>;build`), then links `calc.exe` against `build/mathlib.dso`. `.pkg`: `calc.exe`, `mathlib.dll` → `!:\sys\bin\`, `calc_reg.rsc`. EKA2L1 shows `MathTwice(21)=42 MathAbs(-5)=5` (PID-bound screenshot), so ordinals from our `.dso` resolve at runtime. Wine makesis writes type 41 for **each** E32 file from its own header (the DLL too); with that, the native controller equals Wine's once date/time are masked, same file size (4716).

## 54. Frozen exports: `--definput`, `ABSENT`, data exports, `--dlldata` (T4)

- **Requires:** experiment 52. SDK rules read from `epoc32/tools/mmp.pm` (default `.def`: `<mmp dir>/../eabi/<base>u.def`; `DEFFILE` overrides base/dir, `~` means `eabi`; `u` dropped by `NOSTRICTDEF`) and `cl_bpabi.pm` (`--definput` when the frozen `.def` exists, else `--ignorenoncallable`; `--dlldata` for `EPOCALLOWDLLDATA`).
- **Procedure:** Run `elf2e32_next` on the experiment-52 `mathlib.elf` with hand-written `.def` inputs: same order, reversed order, one symbol missing from the `.def`, one extra `.def` symbol absent from the ELF (with and without `ABSENT`), ordinal gaps and reordering. Build `shape` (a `CBase` class with virtual members), `data` (exported `TInt gCounter = 7` and `const TInt KAnswer`), `bss` (uninitialised global), `zts` (asm-named `_ZTS…`/`_ZTI…`/`_ZTT…` objects), and sweep `ABSENT` positions over the 4…40-export DLLs from experiment 52. Compare the native encoder (DLL compressed and `--uncompressed`, `.dso`, `.def`) with every image elf2e32_next writes.
- **Outcome:** pass
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-54/`.
  - **Ordinals:** the `.def` order is kept; ELF symbols it lacks follow, sorted by name, under `; NEW:` in `--defoutput` (warning "New Symbol … not yet Frozen"); a frozen `.def` without new symbols is echoed without the marker. `.def` ordinals must be 1, 2, 3, … in file order ("Ordinal number is not in sequence"). A `.def` symbol missing from the ELF is an error (rc 217, `.def` written with `ABSENT`, no image) unless the input already says `ABSENT`.
  - **`ABSENT`:** the export slot holds the entry-point address; `.dso` names it `_._.absent_export_<n>` (FUNC, size 4); `.def` keeps the original name plus ` ABSENT`. The V header describes holes with a presence bitmap (bit per ordinal, LSB first, padding bits set): type 1 = whole bitmap when shorter (≤8 ordinals: 1 byte), type 2 = sparse (bitmap of non-`0xff` bytes, then those bytes) when shorter (≥17 ordinals); the header grows and `iCodeOffset` is rounded up to 4 (3-byte description → `0xa0`). With 9–16 ordinals both are 2 bytes and elf2e32_next fails its own check ("gaps between export description and code sections"), so there is no golden: symdev reports a TODO.
  - **What is exported:** defined global FUNC and OBJECT symbols outside `SHN_ABS`, except names starting `_ZTS` (typeinfo names). `NOTYPE` linker markers never are. `--ignorenoncallable` made no difference in any case. OBJECT exports are `DATA <size>` in `.def` and `STT_OBJECT` of that size in `.dso`; a frozen entry's kind comes from the `.def` (a `_ZTI` frozen without `DATA` stays FUNC size 4).
  - **Comments:** kept as the text from `;`, trailing blanks trimmed, written back after ` ; ` — so `; x` becomes `; ; x` and grows each round trip (elf2e32_next quirk; `symdev freeze` therefore never rewrites frozen lines).
  - **Writable data:** any `.data` or BSS in a DLL is refused without `--dlldata` ("contains initialized/uninitialized writable data", rc 244); with it, data exports' slots get data relocations and everything matches.
  - Native output byte-equals elf2e32_next (DLL apart from time/CRC, `.dso`, `.def`) in all 60+ cases that produced an image; every case it refused, symdev refuses too.
  - **Freeze:** SDK `efreeze.pl` does not run on Linux (Windows paths), so `symdev freeze` uses its own rule: keep the frozen text, append the `; NEW:` lines. E2E in EKA2L1: `calc.exe` built against frozen `mathlib`, then `MathAab` (sorts first) added and only the DLL rebuilt: frozen → `MathTwice(21)=42 MathAbs(-5)=5`; unfrozen → `MathTwice(21)=268435477 MathAbs(-5)=995` (old ordinals hit the wrong functions). Emulator only; not E52 support.

## 55. App icon: SVG → MIF with Wine `mifconv` (GUI)

- **Requires:** experiment 51 (GUI app), Wine.
- **Procedure:** SDK example icon makefiles (`cpp_examples/*/group/icons_scalable.mk`) run `mifconv <App>_aif.mif /h<App>.mbg /c32,8 <icon>.svg` and the app's `LOCALISABLE_APP_INFO` names `icon_file = "\\resource\\apps\\<App>_aif.mif"`, `number_of_icons = 1`. Run SDK `mifconv.exe` (version 1.11 build 49) under Wine on a hand-written SVG Tiny; install the MIF with the GUI app and look for it in EKA2L1.
- **Outcome:** pass (emulator only)
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-55/`.
  - Bare `mifconv` fails: "Binary converter 'SVGTBINENCODE.exe' not found" (needs `/S<epoc32\tools>`), then "Changing temporary working directory failed! \epoc32\BUILD\s60\icons\temp\" (needs `/T<dir>`). With both it writes the MIF and the `.mbg` enum (`EMbm<App>…`, 16384/16385).
  - MIF layout: `B##4`, version 2, entry table offset 16, entry count 2 (icon and mask entries point at the same icon); each icon: `C##4`, version 1, header size 0x20, data length, type 1 (SVG), depth 0xb, animated 0, mask depth 4, then the `svgtbinencode` output.
  - **Path trap:** the input path is mangled into the temporary `.svgb` name (`Z__home_…_gui.svgb`); when that path gets long (between ~85 and ~150 characters it stopped working) `svgtbinencode` writes nothing and `mifconv` still exits 0 with a 64-byte MIF whose icons have zero length. symdev runs it in `build/mifconv-temp` on `<app>.svg` with relative paths (worked from a 168-character directory) and rejects a MIF with an empty icon.
  - The S60 context pane and Menu grid in EKA2L1 show a placeholder for every app (ROM Menu too), so they cannot confirm icons; EKA2L1's own app list shows the `gui` icon from our MIF (other apps: EKA2L1's default).
  - Native MIF writing needs a clean-room SVG → SVGB encoder; not done.

## 56. Native resource compiler: SDK example corpus (T3)

- **Requires:** experiments 9, 41–42 (Wine `cpp.exe` + `rcomp.exe` argv, RSC header), a legal-access FP2 SDK with the examples unpacked.
- **Procedure:** Compile every `.rss` under the SDK examples with the experiment-9 Wine `cpp.exe` + `rcomp.exe -u -h` argv (include path: the `.rss` directory, `../inc`, `../include`, `../../inc`, `../data`, the `.rsg` files generated so far, the casefold overlay, `epoc32/include`, `variant`, `oem`; non-`_reg` files first) into golden `.rpp`/`.rsc`/`.rsg`. Write hand-made probe `.rss` files for single features and compile them with Wine `rcomp.exe` alone. Build symdev's preprocessor and compiler; compare (a) native `rcomp` on the golden `.rpp`, (b) native preprocessor + native `rcomp` on the original `.rss`.
- **Outcome:** pass (one open case, see below)
- **Evidence:** 2026-09-19, `$HOME/src/symdev-experiment-56/` (corpus script, goldens, probes; outside git).
  - Corpus: 165 `.rss`; Wine compiled 143 (21 stop in `cpp` on includes the examples do not ship, 1 in `rcomp`). Native (a) and (b): **143 of 143 byte-equal** (`.rsc` and `.rsg`).
  - **Compress only when shorter.** A text becomes a compressed run only when that is fewer bytes than leaving it in the raw run; ties stay raw. Compressing costs the run header (1 byte, 2 from 0x80) plus the encoded bytes, plus one byte to reopen a raw run when anything follows and one to close an open compressed run; staying raw costs the alignment pad plus two bytes per character, plus one byte when a raw run has to be opened. Text that stays raw is written with fill byte `0xab` instead of the `0x00` pad. `imopenapiexample` (an array of nine one-character `LTEXT`s) shows both sides: the first eight are a tie and stay raw, the last one has nothing after it and is compressed. Pinned by the probe `exp56_compress_only_when_shorter`.
  - **File layout (`-u`):** UID block (UID1 `0x101f4a6b`); a flags byte (`0x01` when there is no `UID3` statement: UID3 is then the `NAME` value, UID2 0); `u16` size of the largest resource when uncompressed; one bit per resource set when it is stored packed; resources; `u16` start offsets plus the index offset.
  - **Packed resources:** runs alternate compressed-Unicode / raw starting with a compressed run (possibly empty); a run length is one byte, or two (`0x80 | high`, low) from 0x80; alignment pads before 16-bit text are dropped. Compressed Unicode is SCSU: Latin-1 passes through, control characters are quoted with SQ0 (`"\f"` → `01 0c`), Cyrillic selects window 2 (`12 …`) or quotes one character (`03 b1`).
  - **Ids and names:** `NAME` is base 27 (A/a = 1 … Z/z = 26, digits 0): `TEST` → `0x6120e`, `L10N` → `0x39ab2`; resource id = `NAME << 12 | index` (1-based). `.rsg`: `#define <NAME in upper case>` padded to column 50 with at least one space, the id as `0x%x` (decimal when there is no `NAME`), CRLF.
  - **Values:** 16-bit text starts at an even offset within the resource (a `0x00` pad after an `LTEXT` length byte at an odd offset); empty texts have no pad; arrays default to a `WORD` count, `LEN BYTE` to a byte count; `STRUCT` members may be given as `{ S { … } }`; an undefined name where text is expected is stored as its own spelling (the SDK example compiles with missing `.rls` names that way); `"\f"` is U+000C.
  - **Preprocessor:** the SDK resource headers need `#define` (object- and function-like, one `##`), `#include` with `\` and wrong-case names (resolved case-insensitively, so resources no longer need the casefold overlay), `#if`/`#ifdef`/`#ifndef`/`#elif`/`#else`/`#endif` with `defined` and undefined names as 0.
  - `symdev build` compiles `START RESOURCE` natively; `examples/gui` `gui.rsc`, `gui.rsg`, `gui_reg.rsc` are byte-equal to the Wine tools' output.
  - Wine `rcomp` crashes (page fault, `winedbg`) on some probe inputs (a `WORD` initialised from an unnamed `enum`, forward `LINK`, `TEXT`); none of these occur in the corpus.

## 57. Native icon encoder: SVG Tiny → SVGB → MIF (T5)

- **Requires:** experiment 55 (Wine `mifconv` goldens) and the clean-room [svgb-mif-spec.md](svgb-mif-spec.md).
- **Procedure:** Implement `symdev-mif` (SVG parser, SVGB encoder, MIF container, `.mbg` header) from the spec, then compare its output with Wine `mifconv.exe` for the shipped icon template and for a hand-written `<circle>` document; build and install `examples/gui` and look for the icon in EKA2L1's app list.
- **Outcome:** pass (emulator only; **not** E52 support)
- **Evidence:** 2026-09-20. `gui.svg` → 193-byte `.svgb`, 257-byte `.mif`, 164-byte `.mbg`, all three **byte-equal** to `mifconv.exe` under Wine; the `<circle cx cy r fill>` document likewise (the spec's §8 snippet for it drops one byte of the `fill` record — the real tool writes the flag byte, and so do we). Pinned in the crate's tests: the file header and tree markers, the whole template, the circle, number truncation toward zero, the `|v| > 32765` drop, the `#rgb` expansion bug, `none`/`rgb()`/`url()` paint, the version-1 float and byte-reversed colour, the container and the `.mbg` naming rules.
  With this, `symdev build` no longer runs Wine at all: resources (experiment 56) and icons are native, and `SYMDEV_WINE` is gone from the toolchain. Elements beyond `svg`, `g`, `rect` and `circle`, and attributes outside the icon subset, are refused with a `TODO: … (not observed)` error rather than silently dropped as the SDK tool does.

## 58. Native `bmconv`: BMP → `.mbm` and `.mbg` (T5)

- **Requires:** the clean-room [bmconv-spec.md](bmconv-spec.md) and Wine for the goldens.
- **Procedure:** Implement `symdev-mbm` (BMP reader, depth conversion with the two built-in palettes, the four RLE encoders, the file store and the `/h` header) from the spec. Generate 32 BMPs (sizes 1×1 … 32×8, solid / noise / bands / two-tone) and compile each at all nine depths with Wine `bmconv.exe`, once with `/n` and once with compression, comparing every byte.
- **Outcome:** pass
- **Evidence:** 2026-09-20, `$HOME/src/symdev-experiment-58/` (generator, goldens, comparison script; outside git). **288 of 288 uncompressed and 288 of 288 compressed files byte-equal**, covering the bytewise, 12-bit, 16-bit and 24-bit encoders, the `0xFF` row padding, the odd 12- and 24-bpp strides, the twips formula and the colour substitutions. `bmconv /h` output reproduced byte for byte.
  - The 512- and 4096-entry lookup tables are **generated** from the two palettes by city-block nearest with the first index on a tie, exactly as the spec describes; no table is embedded.
  - Depth options attach to the file name (`/8icon.bmp`), not as separate arguments.
  - Pinned in the crate: eight golden `.mbm` files across all depths, the recorded bytewise streams of spec §7.2, the grey and colour conversion table of §4.1/§4.3 and the `.mbg` text of §9.2.
  - Not implemented yet: the ROM stores (`/r`, `/s`), `/u` decompile, `/v`, `/p` custom palettes, `/m` and command files; a `.mbm` still has to be wired into `symdev build` (a project needs a way to declare bitmap sources).


## 59. Compiler dialect: `.c` through the C front end, and `gcce.h` varargs on AAPCS GCC

- **Requires:** experiments 1–2 (SDK, toolchain). Closes gaps 7 and 8 of [third-party-app-puzzles.md](third-party-app-puzzles.md).
- **Procedure:** Build a throwaway project (`/tmp/claude-1000/c-dialect-work/proj`, outside git) with one `.cpp` that calls `TDes::AppendFormatList` through `VA_LIST`/`VA_START` and one `.c` with constructs that only compile as C (anonymous `enum` inside a struct, C99 designated initialisers, a member named `class`, implicit `void*` conversion) plus its own `va_list` function. Drive it through `symdev build`. Read the SDK's own compiler config and `gcce.h`; measure `__builtin_va_list` on this GCC; check what the SDK libraries actually export before changing anything.
- **Outcome:** pass (compile and link only; no device, no emulator run in this experiment)
- **Evidence:** 2026-09-20, GCC 12.1.0 `arm-none-symbianelf`, SDK S60 3rd FP2.
  - **Both gaps reproduced first.** `.c` as C++: `error: expected identifier before ';' token`, `expected unqualified-id before 'class'`. Varargs: `error: invalid initialization of reference of type '__va_list&' from expression of type 'void*'` in C++, `error: first argument to 'va_arg' not of type 'va_list'` in C.
  - **The SDK uses one compiler for both dialects.** `epoc32/tools/compilation_config/gcce.mk`: `CC=arm-none-symbianelf-g++`, `CPP_LANG_OPTION=-x c++`, `C_LANG_OPTION=-x c`, `CIA_LANG_OPTION=-x c++ -S -Wa,-adln`, `PREINCLUDE_OPTION=-include $(EPOCROOT)EPOC32/INCLUDE/GCCE/GCCE.h`. So symdev routes `.c` through `SYMDEV_GXX -x c` rather than a second `…-gcc` binary; `.cpp` keeps the recorded experiment-5 argv unchanged (no `-x c++`). Anything that is not `.cpp` or `.c` — `.s`, `.S`, `.cia` — is now refused with a `TODO: … (not observed)` error.
  - **Which flags the C front end rejects.** Only `-fpermissive`: `cc1: warning: command-line option '-fpermissive' is valid for C++/ObjC++ but not for C` (exit 0, but diagnosed). `-Wno-narrowing` is accepted silently yet names a C++-only conversion rule, so it goes too. `-fexceptions`, `-mapcs`, `-mthumb-interwork`, `-msoft-float`, `-march=armv5t` and every `-D` are accepted unchanged. `g++ -x c` leaves `__cplusplus` undefined (so `gcce.h` takes its C branch: `typedef unsigned short __TText`, no `namespace std`) and defaults to `__STDC_VERSION__ 201710L`.
  - **The varargs cause.** `gcce.h:96` declares `typedef struct __va_list { void *__ap; } va_list;` and then `#define va_start(ap, parmN) __builtin_va_start(ap.__ap, parmN)` — it unwraps its own struct and hands the bare `void *` member to the builtin. That is correct only for a compiler whose `__builtin_va_list` *is* `void *`, which is what the SDK's GCCE 3.4.3 had. On this GCC, DWARF for `__builtin_va_list probe;` gives `DW_TAG_structure_type __va_list`, `DW_AT_byte_size 4`, one member `__ap` at offset 0 of a 4-byte pointer — i.e. **the same layout as the SDK's struct**, but a distinct type, so the builtins want a `__va_list` lvalue.
  - **The `VA_LIST` ABI is unchanged by the fix.** `euser.dso` exports `_ZN6TDes1610FormatListERK7TDesC16St9__va_list` and `_ZN6TDes1616AppendFormatListERK7TDesC16St9__va_listP14TDes16Overflow`; `St9__va_list` is what both `std::va_list` (the `gcce.h` typedef) and `__builtin_va_list` mangle to on this compiler, checked by compiling a declaration of each. symdev therefore leaves the typedef alone and redefines only `va_start`/`va_arg`/`va_end` (`VA_START`/`VA_ARG`/`VA_END` expand to them at the point of use) to `__builtin_va_*(*(__builtin_va_list *)&(ap), …)`, in a generated header force-included right after `gcce.h`. At `-O2 -fstrict-aliasing` this is **byte-identical assembly** to the same function written with the compiler's native `__builtin_va_list` — verified in C, and in C++ through the real `gcce.h` plus the generated header for a function that both consumes its varargs and passes the `VA_LIST` on by value to a callee (`bl _Z6calleeSt9__va_list` in both). The repro's object file carries exactly the `AppendFormatList` symbol `euser.dso` exports, and `ld --no-undefined` resolved it.
  - Not a stopgap: the generated header states the cause in a comment and is pinned by tests (`driver/tests/gcce_compat.rs`, `driver/tests/language.rs`) that do not need the SDK. `examples/hello` and `examples/gui` still build byte-for-byte through the unchanged C++ path.
  - Still open: `.cia` and assembly sources have no pipeline; the fix was not exercised on hardware or in EKA2L1 in this experiment (the earlier Puzzles run used the same `*(__builtin_va_list *)&ap` re-point by hand and the app played in the emulator).

## 60. Icon encoder for real app icons: the SVG Tiny subset Illustrator emits (T5)

- **Requires:** experiment 57 (the byte-equal `symdev-mif`), the clean-room [svgb-mif-spec.md](svgb-mif-spec.md) and Wine for the goldens.
- **Procedure:** Widen `symdev-mif` from the four-element template subset to what a real S60 application icon uses: `<path d>` with the full command set, the `<svg>` attributes Adobe Illustrator's "SVG Tiny 1.1" export writes (`version`, `id`, `x`, `y`, `xml:space`, `width`/`height` in `px`, the namespace declarations), `id` everywhere, the remaining shapes (`polygon`, `polyline`, `line`, `ellipse`), `defs`/`use`/`title`/`desc`/`text`/`switch`/`a`/`image`/gradients/`solidColor`, and the presentation attributes (`stroke*`, the three opacities, `fill-rule`, `color`, `visibility`, `display`, the `font-*` set, `transform`, `style`). Write a corpus of 103 SVGs — one feature per file, plus six full Illustrator-style app icons — and compare every one of them byte for byte against `svgtbinencode.exe` at all four encoding versions; run the whole pipeline once through `mifconv.exe` as well.
- **Outcome:** pass
- **Evidence:** 2026-09-20, `/tmp/claude-1000/svg-subset-work/` (corpus, harness; outside git). **103 of 103 files byte-equal at `-v 3`, and 103 of 103 at `-v 1`, `-v 2` and `-v 4` — 412 of 412 comparisons.** The realistic icon also produced a `.mif` and a `.mbg` byte-equal to `mifconv.exe /Ho.mbg /c32,8`.
  - **Everything in the file is 16.16 fixed point.** A literal is parsed as `float`, multiplied by 65536 and truncated toward zero, and *all* later arithmetic is integer arithmetic on that. Versions 1 and 4 convert the fixed value back to a `float` when writing, so `x="0.1"` is the float `6553/65536`, not `0.1`. That conversion truncates the bits a `float` cannot hold instead of rounding them: the saturated `0x7FFFFFFF` comes out as 32767.998046875.
  - **Relative path coordinates are therefore integer sums.** `M84 0 l-8.059 0` gives 4976870, one more than truncating the exact 75.941 (4976869) — the tool truncates the delta (−528154) and adds. Reproducing this is what took the `path-decimals` and Illustrator icons from "one bit off in three places" to byte-equal.
  - **One reflection point, shared by `S` and `T`.** It starts at the origin and is updated by every command except `M` and `Z`: a curve leaves `2·end − last control`, a plain `L`/`H`/`V` leaves `2·new − old`. So `S` reflects a quadratic control point, `T` reflects a cubic one, an `L` between two curves changes the reflection, and a `Z` leaves it alone.
  - **An implicit repeat after `M`/`m` stays a move**, where SVG says it becomes a line: `d="M 1 2 3 4"` is two move commands.
  - **`<use>` writes a second string** after `xlink:href`, not a second copy of the record: the fragment with the `#` stripped when an element carrying that `id` has already been written, otherwise the reference again. (Spec §4.9 read the no-target case as a duplicate.)
  - **`matrix()` takes its kind word from its values** (identity → 0; otherwise bit 2, plus bit 1 for `e`/`f` and bit 4 for `b`/`c`); the other functions contribute a fixed kind — `translate` 1, `scale` 2, `rotate(a)` 6, `rotate(a,cx,cy)` 7, `skewX`/`skewY` 4 — and a list OR-s them. Spec §4.11's "matrix() always sets all three" is wrong.
  - **The colour keyword table is the SVG 1.1 list minus five `grey` spellings** (`grey`, `dimgrey`, `darkslategrey`, `lightslategrey`, `slategrey` encode as black; `darkgrey` and `lightgrey` are present). Matching is case-insensitive. `rgb()` percentages are scaled by the `float` constant 2.55, so 100 % is **254**, not 255.
  - **`style` is a second attribute syntax** with its own property list: the paint, stroke, font, opacity, `display`, `visibility` and `fill-rule` properties are turned into the ordinary records at the position `style` occupies, an unknown property carries no bytes, `stop-color`/`stop-opacity` are dropped or mis-valued, `transform` yields an identity matrix with kind 0, and `style="d:…"` **crashes the tool**.
  - **`preserveAspectRatio` is per element:** two bytes (`00 02`) on `<svg>` and only for `none`; an ordinary string on `<image>`, any value; on `<rect>` the tool leaves a truncated file behind.
  - **`xml:space="preserve"` on `<text>` itself** stops the whitespace collapsing (every whitespace character becomes a space and all of them are kept); it is not inherited from an ancestor. `<title>` and `<desc>` are written as bare tokens — their text never reaches the file.
  - Small parser traps confirmed: a leading `+` is accepted in path data but not in an attribute (`x="+5"` is dropped, `stroke-width="+2"` becomes 0); an opacity written `.25` is read as 1 (`stop-opacity` as 0); `viewBox` with commas is dropped, `stroke-dasharray` with commas is not; `enable-background`, `overflow`, `class` and `pathLength` carry no bytes; `<metadata>` and its children are dropped.
  - **Refused rather than reproduced** (the tool's behaviour is data loss or a crash): elliptical arcs and unknown path commands (it empties the whole path), `1e1` and `0.5.5` in path data, `rotate`/`skewX`/`skewY` (its sine differs from a double-precision one by up to 2/65536 and could not be derived), a leading `+`, `.25` opacities, comma-separated `viewBox`, `preserveAspectRatio` off `<svg>`/`<image>`, `d`/`points`/`transform` inside `style`, colour keywords outside its table, strings over 127 characters, and every element and attribute outside the pinned set.

## 61. `bld.inf` / `.mmp` front end, from the clean-room spec (T5)

- **Requires:** the clean-room [mmp-frontend-spec.md](mmp-frontend-spec.md). Closes gaps 1–5 and 13/14's reachability of [third-party-app-puzzles.md](third-party-app-puzzles.md).
- **Procedure:** Implement preprocessing (§1), lexing (§2), the `bld.inf` grammar (§4), the `.mmp` directive split (§5), `START RESOURCE` (§6) and `START BITMAP` (§7) from the spec alone. Drive a throwaway project (`/tmp/claude-1000/mmp-frontend-impl/e2e`, outside git) carrying every construct through `symdev build`, and compare the bitmap output with the SDK's own `bmconv` under Wine.
- **Outcome:** pass (build, package and byte comparison; no device, no emulator run in this experiment)
- **Evidence:** 2026-09-20.
  - **Preprocessing.** `ProjectCpp` reuses the native preprocessor written for `.rss` (experiment 56). A Carbide block-comment header, `#ifdef GCCE` in `bld.inf`, `#ifdef MARM_ARMV5` and `#if GCCE` in the `.mmp` all behave as §1.3/§14.3 say: the `MARM_ARMV5` branch compiled and linked (`_Z19SymdevArmOnlyMarkerv` in the ELF), the `#if GCCE` branch did not. The 79 macros of the force-included variant header are visible to `#ifdef`.
  - **Spec correction found.** §1.4 says to absorb one space after a restored `_____NAME`. That is right for the SDK's GCC 2.x preprocessor, which adds one space after an expansion; symdev's adds one on *each* side, so absorbing would glue `OPTION GCCE -O3` into `OPTION GCCE-O3`. `ProjectCpp::restore` therefore restores the text only. The difference is invisible to the tokeniser except for a platform macro spelled inside a larger word, which both implementations break, differently.
  - **`MACRO` reaches the compiler**: a source guarded by `#ifndef COMBINED #error` compiled. `OPTION GCCE -O3` lands at §8.4 position 12; `OPTION ARMCC …` stays inert.
  - **`START RESOURCE TARGET`**: `TARGET gui_0xe7351c20` renamed both the `.rsc` and the `.rsg`, the sources `#include <gui_0xe7351c20.rsg>` and compiled, and `LANG SC 01` produced `gui_0xe7351c20.rsc` and `gui_0xe7351c20.r01` (§6.3).
  - **`bld.inf`**: `DEFAULT -EDG`, `TIDY`, `IGNORE` and a `PRJ_EXPORTS` line all honoured (the exported header was staged into `build/` and included); `gnumakefile Icons_scalable_dc.mk` and `START EXTENSION s60/mifconv` refused by name.
  - **`START BITMAP`**: three BMPs at `c8,c24` gave a 779-byte `games.mbm` and a 190-byte `games.mbg` **byte-identical** to `bmconv.exe /q /hgames.mbg games.mbm /c8cube.bmp /c24net.bmp /c8loopy.bmp` under Wine. `#include <games.mbg>` compiled and the `.mbm` reached the SIS at `!:\resource\apps\games.mbm`.
  - `examples/hello` and `examples/gui` rebuild byte-for-byte (every output; the E32 image differs only in its timestamp and checksum, which is true of any two runs).
  - **Not implemented, refused by name instead:** `VENDORID` other than zero (`symdev-elf2e32` has no `--vid`), `UID` inside `START RESOURCE` (`symdev-rcomp` has no `-uid2`/`-uid3`), every `TARGETTYPE` but `EXE` and `DLL`, `AIF`, `SYSTEMRESOURCE`, the flat `RESOURCE` form, `RAMTARGET`/`ROMTARGET`, `FEATUREVARIANT`, the ASSP directives, external makefiles and extension templates.
  - **Recorded and warned about, not honoured:** `EPOCSTACKSIZE`, `EPOCHEAPSIZE`, `EPOCPROCESSPRIORITY`, `VERSION`, `LINKAS`, the compression and paging directives, and the rest of §5.5. Each needs a post-linker option `symdev-elf2e32` does not take yet; half-wiring one would ship the wrong image silently.

## 62. Thumb and interworking on GCCE: whose flags are right? (not run)

- **Why:** [mmp-frontend-spec.md](mmp-frontend-spec.md) §8.4 records that this SDK's `epoc32/tools/compilation_config/gcce.mk` passes **neither** `-mthumb` nor `-mthumb-interwork` — both the instruction-set and the interworking settings are empty in the GCCE configuration — and defines neither `__MARM_THUMB__` nor `__MARM_INTERWORK__`. symdev's `driver/compile.rs` passes all four, from a public write-up rather than from the SDK. The spec also has `-Wall -Wno-ctor-dtor-privacy -Wno-unknown-pragmas -pipe -fno-unit-at-a-time` where symdev has `-fpermissive -Wno-narrowing`.
- **Why it was not settled here:** it is a build experiment, not a spec question (§15 item 1). symdev's current flags produce an app that installs and plays in EKA2L1; changing them needs a device or emulator run to compare, which the front-end work neither needed nor provided.
- **Procedure when it is run:** build `examples/gui` and the Puzzles port both ways, diff the E32 images and the disassembly around every interworking call site, install both on EKA2L1 and, for the answer that matters, on a stock E52.

## 63. Running the SDK's own build-file generator on Linux, and the makefile it produces (T5)

- **Why:** [mmp-frontend-spec.md](mmp-frontend-spec.md) was written by reading the SDK's Perl. Nothing in it above §6 had been checked against a run of the generator, so the compile command line, its flag order and the generated rule set were unverified — including the Thumb question of experiment 62.
- **Procedure:** make the SDK's `bldmake` and `makmake` run under Linux perl 5.40.1, then generate the GCCE makefile for four projects and read the command lines out of it. Recipe: `docs/research/sdk-generator.sh` (with `sdk-generator-winpath.c`, `sdk-generator-cpp.py`, `sdk-generator-File-Path.pm`).
- **Outcome:** pass. The generator runs; four makefiles captured; one SDK module patched, in a way that cannot reach the output.
- **Evidence:** 2026-09-20. Captures under `/tmp/claude-1000/sdk-generator-work/out/` (outside git, SDK tree verified unchanged by `find ~/sdk -newer <timestamp>` after every run).

### The shim

Nothing in the SDK is mutated; it is mirrored into a scratch directory whose `epoc32/build` is the only writable part. `EPOCROOT` is set to that mirror in the shape the generator wants — drive-less, backslash-separated, with a trailing separator — which is what defeats both of the errors an obvious attempt hits (`EPOCROOT must end with a backslash` / `must be capitalised`).

| Shim | What it stands in for | Why it cannot change the generated output |
|---|---|---|
| `winpath.so`, preloaded | the kernel's view of a path | Rewrites backslashes to slashes and resolves spelling case-insensitively inside libc, so the strings perl builds and prints are untouched. Also turns cmd.exe quoting into sh quoting for `/bin/sh -c` command lines (in cmd a backslash inside quotes is an ordinary character), and forces mode `0700` on `mkdir` (Windows ignores the mode; one module asks for mode 2 and then cannot enter the directory it made). |
| `bin/set` | the `cmd.exe` `set NAME` builtin | The environment module checks that `EPOCROOT` is spelled in capitals by reading `set EPOCROOT`. The stand-in prints matching environment entries; perl execs it directly because the command has no shell metacharacters. |
| `bin/make` | the SDK's Windows GNU make | Runs the real make with `SHELL=/bin/bash`. The generator reads the tool-chain settings out of the `.mk` configuration by running `echo VAR=$(VAR)`; under `/bin/sh` the backslashes of every path in those values are eaten. |
| `gccbin/gcc/bin/cpp.exe` | the SDK's Cygwin `cpp.exe` (GNU cpp 2.9x) | Runs the host `g++ -E -x c++`. Three differences are papered over: `-+` is spelled `-x c++`; backslash separators are turned into slashes, because gcc composes a search-path prefix itself before it ever calls `open` so the preload cannot reach it; and the space GNU cpp 2.x inserted after a macro expansion is put back, because the generator's un-expansion step eats one space and would otherwise glue `OPTION GCCE -O3` into one token. It also joins backslash-continued lines of a `.mmp`/`bld.inf` and pads with blank lines, which is what cpp 2.x emitted and what the line-oriented project-file parser expects; the temporary file this needs is substituted back out of the line markers. |
| `perl-overlay/Win32.pm` | the core `Win32` module | The source checker calls only `GetLongPathName`, which expands an 8.3 short name. Linux names have no short form, so the stand-in is the identity. |
| `perl-overlay/File/Path.pm` | the core `File::Path` | On Windows it splits on `\`; on Linux it does not, so an SDK path arrives as one component and `mkpath` degenerates to a single `mkdir`. The stand-in normalises the separator and does the same job. |
| `perl-overlay/SdkWin.pm` | — | Loaded through `PERL5OPT` so it also reaches spawned perls. Puts `File::Basename` into its Windows mode; without it a resource basename comes back as the whole absolute path. |

**The one patched SDK module** is `e32plat.pm`: two occurrences of `defined` applied to a hash, removed from perl in 5.22. Both guard a duplicate-definition warning for `.bsf`/`.assp` platform specifications, and this SDK ships no `.bsf` that survives validation, so dropping `defined` leaves a plain truth test on the same hash and cannot reach the makefile.

### What was captured

`examples/gui` (EXE, two `START RESOURCE`), the SDK's own `npbitmap` (PLUGIN, `START BITMAP` with `HEADER`, `START RESOURCE`, four `SYSTEMINCLUDE`, ten capabilities), the SDK's own `consoleapp` (EXE, two `.c` sources, `STATICLIBRARY`), and a purpose-built `.mmp` carrying `MACRO`, `OPTION GCCE`, `OPTION ARMCC` and `ALWAYS_BUILD_AS_ARM`. The makefile was not run; the flag variables it leaves to the `.mk` configuration were expanded with a three-line makefile that includes that configuration and prints them.

### Findings

- **Thumb, settled as far as a capture can settle it.** The generated makefile's only instruction-set reference is `$(THUMB_INSTRUCTION_SET)`, and with `ALWAYS_BUILD_AS_ARM` it becomes `$(ARM_INSTRUCTION_SET)`; the GCCE configuration defines both as empty, along with the Thumb and interworking define settings. The expanded UREL compile line therefore carries **no `-mthumb`, no `-mthumb-interwork`, no `__MARM_THUMB__`, no `__MARM_INTERWORK__`** — and `ALWAYS_BUILD_AS_ARM` changes nothing at all on GCCE. Experiment 62's question is unchanged: this says what the SDK does, not which binary an E52 accepts. No Rust was touched.
- §1.2, §1.3 and §1.6 hold exactly, now from a run rather than a reading. `bld.inf` really is preprocessed once with no `-D` at all and then once per platform; this SDK's platform list is `WINSCW`, `GCCXML`, `ARMV5`, `GCCE`, and the `ARMV5` pass defines `GCC32`, not `ARMCC`.
- §8.2, §8.3, §8.4 and §8.6 hold in full, including `__GCCE__` and `__MARM_ARMV5__` appearing twice, `MACRO` landing between `-D__EXE__` and `-D__SUPPORT_CPP_EXCEPTIONS__`, `OPTION GCCE` landing at position 12, and `OPTION ARMCC` staying inert.
- **The SDK include directory really is not implicit.** `examples/gui` has no `SYSTEMINCLUDE`, and its generated compile line has no `-I …/epoc32/include`: the generator warns that it cannot find `aknapp.h`, `eikenv.h` and the rest. symdev builds that same project, so symdev adds an include path the SDK would not.
- §6.4 and §7.6 describe the wrapper calls correctly; the captures add the wrappers' own argument lists, and §7.6's `bmconv` line remains a reading, since the makefile was not run.
- §12 was extended: the post-link's `--elfinput`, `--linkas`, `--libpath` and `--sysdef` were missing, `--ignorenoncallable` sits between `--output` and `--dso`, and the map file is switched by the configuration variable rather than by a compiler-version probe.
- **A gap in the SDK, not in the shim:** the tool-chain install path is derived by splitting the compiler's `libgcc` path on `/../`. GCC 12.1.0 prints a path with no `/../` in it, so the split keeps the trailing newline and the generated makefile breaks across two lines. The CSL toolchain the SDK was written for prints `…/bin/../lib/…`.

### Known limits of the shim

Two things a modern preprocessor does differently are papered over rather than reproduced (the expansion space and the continued-line layout); both are listed above with what they do. The tool-chain include directory and the linker's two `-L` paths reflect whichever GCCE toolchain is on `PATH`, and are empty without one. Dependency lists in the captured makefiles are as complete as the project's own `SYSTEMINCLUDE` lines allow, which for `examples/gui` is not very.

## 64. Icon containers from `symdev.toml`: what `mifconv` does with `.bmp` sources, and the makefile hand-off

- **Why:** [third-party-app-puzzles.md](third-party-app-puzzles.md) gap 13: Carbide projects build their icons from `gnumakefile` lines in `bld.inf`, each makefile one `mifconv` call, and [svgb-mif-spec.md](svgb-mif-spec.md) §9 left "everything about how `mifconv` drives `bmconv` for `.bmp` sources" unspecified. symdev runs no makefile, so the call has to be declared in the manifest and reproduced natively.
- **Procedure:** run the SDK `mifconv.exe` 1.11 (build 49) and `bmconv.exe` 112 under Wine from a short working directory with relative source names (§3.1 trap 3), capturing the command file it hands `bmconv` with `strace -f -s 3000 -e trace=write,execve`; then implement `[[icons]]` and compare every file symdev writes for the Puzzles project (`gfx/*.bmp` ×34, `gfx/app.svg`) against the tool's.
- **Outcome:** pass — 4/4 byte-equal (`games.mif` 560 B, `games.mbm` 209 743 B, `puzzles_0xa000ef77.mbg` 1489 B, `puzzles.mif` 5658 B). The acceptance loop (pristine clone + one `symdev.toml`) compiles and links every source and stops in the post-link at `TODO: --sid other than --uid3 (not observed)`, which is the project's protected-range `SECUREID`, not an icon gap.
- **Evidence:** 2026-09-20, scratch under `/tmp/claude-1000/icon-sets-work/mif/` (goldens in `golden/`, outside git).
  - **`mifconv` needs `/S<dir>` even for bitmap-only input** (`ERROR: Binary converter 'SVGTBINENCODE.exe' not found`), and finds `bmconv` only through `/B<dir>`; with both, the whole bitmap chain runs under Wine. The "broken bmconv call" of the first Puzzles run was the missing `/B`.
  - **The `bmconv` hand-off.** `mifconv` copies nothing: it writes `<out>.mbm_###_bmconv_tmp_cmd_file` containing `/q <out>.mbm /c24blackbox.bmp  /c24bridges.bmp  /c24cube.bmp  ` and runs `bmconv <cmdfile>` — no `/h`, so the `.mbg` is `mifconv`'s own. The `.mbm` is byte-equal to `bmconv /q out.mbm /c24a.bmp …` run directly, which `symdev-mbm` already reproduces.
  - **The bitmap `.mif` is a stub:** `B##4`, v2, table at 16, 2×N entries, and a bitmap's two entries are `(−index as u32, 0)` — its index in the sibling `.mbm`. Depth-independent (`/c8 /c24 /1` gives the same bytes as `/c24` ×3); 1 bitmap → 32 B, 3 → 64 B, 34 → 560 B.
  - **A bitmap mask (`/c24,8`)** goes to `bmconv` as `/8<stem>_mask_soft.bmp` right after its icon and takes its own index: icon `(0,0)`, mask `(−1,0)`, next bitmap `(−2,0)` twice. Missing file: `ERROR: EGray256 Mask not found! blackbox_mask_soft.bmp`. With both `_mask.bmp` and `_mask_soft.bmp` present, `/c24,1` *also* used `/8<stem>_mask_soft.bmp`, so the `/1` mask form was never seen: symdev refuses mask depth 1 on a bitmap by name, and `/A` on a bitmap likewise.
  - **The `.mbg` steps by two per icon whatever the mask**, and the `_mask` line appears only when a mask depth was given: two SVGs at `/c32` give `A = 16384`, `B = 16386`. Bitmaps follow the same rule.
  - **The enum is named after the header, not the `.mif`:** `games.mif /Hpuzzles_0xa000ef77.mbg` gives `enum TMifPuzzles_0xa000ef77` and `EMbmPuzzles_0xa000ef77Blackbox`. §7 of the spec said "mif stem" because every example there had the two stems equal; corrected.
  - **The output extension is forced.** `mifconv x.mbm … app.svg` writes `x.mif` (Puzzles' `Icons_scalable_dc.mk` names `puzzles.mbm`; its `.rss` and `.pkg` use `puzzles.mif`); `mifconv g.mbm … a.bmp` writes `g.mif` (32 B) and `g.mbm`. The manifest therefore requires a `.mif` `dest` and derives the `.mbm` beside it.
  - **Mixed `/c24 a.bmp /c32,8 x.svg /c24 c.bmp`:** entries `(0,0)×2`, the SVG block at 0x40 (= 16 + 8×6) twice, `(−1,0)×2`; the `.mbm` equals direct `bmconv` over the bitmaps in order; the `.mbg` is interleaved in source order (`Blackbox = 16384`, `A = 16386`, `A_mask = 16387`, `Cube = 16388`).
  - **Manifest shape chosen:** `[[icons]]` = one `mifconv` call: `dest` (the `.mif`'s install path, normalised like `[[install]].dest`), optional `header` (bare `.mbg` name, written to `build/`), optional `depth` (the default for every source), `sources` = `"path"` or `{ file, depth, animated }`. The 34-bitmap set is 34 strings under one `depth = "c24"`. `[symbian] icon` is unchanged. A `gnumakefile`/`makefile`/`nmakefile` line in `bld.inf` is now a warning naming the file and line, not an error.
  - **Dead end:** `/B` without `/S` never reaches the bitmap stage; the usage text is printed after `Choosing...` and the `ERROR:` line only shows with the head of the output.

## 65a. A `no_std` Rust `E32Main` through the native pipeline into EKA2L1 (T5, Rust SDK)

- **Requires:** the recorded link line (exp 5), native `symdev-elf2e32` (exp 45–47), native SIS (T2), `symdev run`; `rustup target add armv5te-unknown-linux-gnueabi` (stand-in with a prebuilt `core`) — the real `arm-symbian-e32` target is experiment 65. Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md).
- **Procedure:** `hello.rs` (`#![no_std] #![no_main]`, `#[export_name = "_Z7E32Mainv"]`, calls `User::InfoPrint` with a `_LIT`-shaped static, `User::After(5 s)`, returns 0; panic → `User::Exit(-1)`), compiled by stable rustc 1.98.1 to a `staticlib` with `-C panic=abort -C relocation-model=static -C opt-level=s -C force-unwind-tables=no`; linked with the recorded argv; post-linked by the native `elf2e32` with the recorded EXE argv; packaged by `symdev package` from a `bld.inf`-less project; run by `symdev run`. Control: the same three calls in C++ through the normal `symdev build`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/65a-rusthello/` (source, probe, the 755-byte E32).
  - **Descriptor layout observed, not assumed:** the type enum is not in the public headers (all `TPtrC16` constructors are `IMPORT_C`), but `_LIT(KHello,"Hi")` compiled by the observed GCCE argv at `-O0` is `02000000 48006900 0000` — a length word with type nibble 0, then UTF-16 — so a `#[repr(C)] { u32, [u16; N] }` static is a valid `const TDesC16&`.
  - **euser is callable from Rust without a shim:** static member functions are plain EABI; `_ZN4User9InfoPrintERK7TDesC16`, `_ZN4User5AfterE27TTimeIntervalMicroSeconds32`, `_ZN4User4ExitEi` (from `nm -D euser.dso`) bound to the versioned DSO exports through `--default-symver` exactly as C++ objects do. `TTimeIntervalMicroSeconds32` by value = one register (the 5-second wait was honoured).
  - **Link hypothesis 3 (spec §4) settled:** a Rust static library placed in the object position is *not* pulled in — the reference to `_Z7E32Mainv` comes from `usrt2_2.lib` inside the `-( … -)` group after it, not from `eexe.lib`. `-u _Z7E32Mainv` before the archive makes the linker pull the one member (the map shows only `hello_rs…cgu.0.rcgu.o`; no `core`/`compiler_builtins` member was needed). Hypothesis 4 (`compiler_builtins` vs `-lgcc` collisions) did not arise because nothing from it was pulled — still open for real programs.
  - ELF 15380 bytes with the same six `NEEDED` DSOs as a C++ EXE; native `elf2e32` produced a 755-byte E32 (`file`: Psion Series 5 executable); `symdev package` added the generated `_reg.rsc` and signed it.
  - **Ran on the emulated OS:** EKA2L1 log `[Service.Notifier]: Trying to display: Hello from Rust SDK`, then `Corrupted graphics command list! Emulation halt.` The C++ control (`ctrl/`, `_LIT` + the same three calls) produced **the identical two lines**, so the halt is EKA2L1's `User::InfoPrint` note rendering, not the Rust image. **Correction 2026-09-20:** that line is not a failure. EKA2L1's OpenGL driver aborts its command queue *before* setting its stop flag, so the render thread's `pop()` returns nothing while the flag is still false and it logs the error on every clean shutdown. Both runs exited normally; the app had already done its work. Worth a one-line fix upstream (check the stop flag before logging), tracked separately from PRs #724–#728.
  - What this does not show: `alloc`, anything leaving, panics, Thumb interworking (rustc emitted ARM code, as the SDK does), a device.

## 65. `symdev new --language rust` → `build` → `package` → `run`: the Rust target as a product path (T5, Rust SDK)

- **Requires:** experiment 65a (the hand-built Rust E32); nightly `rustc`/`cargo` with `rust-src` (`-Zbuild-std`); the recorded link line, native `symdev-elf2e32`, native SIS, `symdev run`. Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §3–§5, §9.
- **Procedure:** (1) `symbian-rs/` — a second Cargo workspace at the repo root (`exclude`d from the host one), `rust-toolchain.toml` pinning `nightly-2026-09-19` (+ `rust-src`), `targets/arm-symbian-e32.json` derived with `rustc +nightly -Z unstable-options --print target-spec-json --target armv5te-none-eabi` and overridden field by field; (2) `symbian-sys` (euser declarations, mangled names from `nm -D euser.dso`, the `_LIT16` layout of 65a), `symbian-runtime` (`entry!` macro exporting `_Z7E32Mainv`, panic → `User::Exit(-1)`), `examples/hello`; (3) `cargo build --release` on the custom target, then the 65a hand path (recorded link + `-u _Z7E32Mainv`, native elf2e32, `symdev package`/`run`); (4) host side: `language.name = "rust"`, `RustBuild: BuildBackend` (cargo, then `GcceBuild::link_args` + `-u _Z7E32Mainv`, then `GcceBuild::elf2e32_args`, unchanged), `symdev new <name> --language rust`; (5) the one-liner from an empty directory with only `symdev-env.sh` and `SYMDEV_SIGN_PASSWORD` set.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/65-hello/` (the 752-byte E32 `symdev build` wrote; source is `symbian-rs/examples/hello`).
  - **Acceptance:** `symdev new hello --language rust && cd hello && symdev build && symdev package && symdev run` from an empty directory: `build/hello.exe` 752 bytes (65a: 755, stand-in target), `build/hello.sisx`, then `build/eka2l1.log` line 379 `[Service.Notifier]: Trying to display: Hello from Rust SDK` and line 380 the known `Corrupted graphics command list! Emulation halt.` (EKA2L1's `User::InfoPrint` rendering, same for the C++ control in 65a); the emulator process exited on its own. No `bld.inf`, no `.mmp`, no hand step.
  - **Target JSON, overrides on the derived `armv5te-none-eabi` spec:** `os = "symbian"`, `env = "e32"` (naming only; nothing in `core` keys on them); `c-enum-min-bits = 32` (derived says 8 — AAPCS short enums — but a probe compiled with the observed GCCE argv gives `sizeof(enum {EA,EB}) == sizeof(TCapability) == 4`, so GCCE does not short-enum; spec §10 item 8 settled); `eh-frame-header = false`, `default-uwtable = false`, `has-thread-local = false`, `executables = false` (explicit; rustc prints none of the defaults back). Kept as derived although the spec said otherwise: `linker`/`linker-flavor` (`rust-lld`/`gnu-lld` — never run for a `staticlib`; `executables = false` already makes cargo refuse a `bin`, which is why a `src/main.rs` library needs `autobins = false`); `features = "+soft-float,+strict-align"` (`+v5te` is implied by `llvm-target`). `panic-strategy = abort`, `relocation-model = static`, `max-atomic-width = 0`, `atomic-cas = false`, `emit-debug-gdb-scripts = false` are the derived values already. `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"` are `[profile.release]` settings, not target fields.
  - **Toolchain facts:** this nightly's cargo refuses a `.json` target without `-Zjson-target-spec` (config `[unstable] json-target-spec = true` or the flag); `-Zbuild-std=core,alloc` compiles `core`, `alloc` and `compiler_builtins` (which pulls `cfg-if`, `gimli`, `object`, … from crates.io into the workspace `Cargo.lock`). `rust-toolchain.toml` with `channel = "nightly-2026-09-19"` made rustup auto-install that dated build (1.100.0-nightly 420ed2a0c 2026-09-18); the host's floating `nightly` is one day newer. `RustBuild` removes `RUSTUP_TOOLCHAIN` from cargo's environment: a symdev started through a rustup proxy would otherwise force the host toolchain on the project.
  - **The archive:** `libhello.a` 902900 bytes, two members — the LTO'd `hello…cgu.0.rcgu.o` (`.text._Z7E32Mainv` 40 bytes, undefined only `_ZN4User5After…`, `_ZN4User9InfoPrint…`) and `compiler_builtins…rcgu.o` (libm, `__aeabi_*`, `mem*`, all weak `W`). Spec §4 hypothesis 1: rustc *does* emit `.ARM.exidx` (one `CANTUNWIND` entry per function) despite `panic = abort` and `default-uwtable = false`; the linked ELF carries `.ARM.exidx`/`.ARM.extab` exactly as a C++ EXE does (eexe/usrt objects) and the native post-linker accepts it. Hypothesis 4: the `compiler_builtins` member is not pulled for hello (the map lists the hello member only), so no collision arose; `memcpy`/`memset`/`memmove`/`memclr` are strong versioned exports of `euser.dso` and `__aeabi_mem*` of `drtaeabi.dso`, `memcmp` is exported by nothing on the line — a program that references any of them will pull the weak `compiler_builtins` member first (archive precedes the DSOs); to be observed in 66.
  - **Link:** `GcceBuild::link_args` unchanged plus `-u _Z7E32Mainv` inserted after `-u _E32Startup` (`RustBuild::link_args`, unit-tested against the C++ argv); ELF 15228 bytes, the same six `NEEDED`. Post-link: `GcceBuild::elf2e32_args`, unchanged.
  - **Integration decisions (stopgaps, labelled in code):** the scaffold writes absolute paths — `symbian-runtime = { path = "<sdk>/crates/symbian-runtime" }` in `Cargo.toml` and `build.target = "<sdk>/targets/arm-symbian-e32.json"` in `.cargo/config.toml` — where `<sdk>` is `SYMDEV_RUST_SDK` or the `symbian-rs/` of the checkout symdev was built from (`RustSdk`); `rust-toolchain.toml` and `src/main.rs` are verbatim copies of the SDK's (`include_str!`, so they cannot drift). cargo is `SYMDEV_CARGO` or `cargo` on `PATH`; the project's `rust-toolchain.toml` picks the nightly. `--target-dir build/cargo` keeps everything under `build/`. `--template gui` with `--language rust` is a `TODO` error. `symdev new --target` now defaults to `nokia-e52` and `--language` is an alias of `--lang`.
  - What this does not show: `alloc` (experiment 66: `User::Alloc`/`Free`/`ReAlloc` are declared in `symbian-sys` with their `nm` names but unused; cell alignment unobserved), the C++ shim path, panics reaching `User::Exit` (the handler is linked but never ran), Thumb, a device.

## 66. `SECUREID` other than UID3, and overriding it from the manifest (T5)

- **Requires:** the native post-linker (exp 45–47). Reference: `elf2e32_next` at `~/src/elf2e32_next/bin/Release/elf2e32` (EPL-1.0, the crate's recorded reference).
- **Procedure:** run the reference over one ELF twice, with `--sid` equal to and different from `--uid3`, and diff. Then thread the secure id through `E32Image` and add a manifest field that overrides an MMP `SECUREID`.
- **Outcome:** pass
- **Evidence:** 2026-09-20, `/tmp/claude-1000/sid-exp/` (outside git).
  - The reference accepts a differing `--sid` and changes **exactly two fields**: the `E32ImageHeaderV` secure id at **0x80** and the header CRC at **0x14**. UID3 at 0x08, the vendor id at 0x84 and every other byte are untouched. So `SECUREID` is an independent identity, not a second spelling of UID3 — the earlier `TODO: --sid other than --uid3 (not observed)` was an unimplemented path, not a format constraint.
  - Our encoder is byte-equal to the reference for a differing SID: golden `crates/symdev-elf2e32/src/testdata/exp66_counter_sid.exe.hex` (the experiment-49 `counter.elf`, `--uid3=0xecacde23 --sid=0xa000ef77`), pinned by `experiment_66_secure_id_other_than_uid3_matches_elf2e32_next`.
  - **Precedence:** `[symbian] secure_id` beats an MMP `SECUREID`, which beats the post-linker's default of UID3. The manifest wins because it is the project's identity in symdev and a third-party MMP often names a secure id the operator cannot sign for; an override that differs from the MMP's is reported as a warning naming both. `secure_id` is range-checked exactly like `uid3` (the recorded self-sign ranges), so a protected-range secure id is refused at manifest level rather than at install time.
  - End to end on the Puzzles port with no project edit: `uid3 = "0xA000EF77"`, `secure_id = "0xE000EF77"` produced an E32 with `uid3 @0x08 = 0xa000ef77` and `sid @0x80 = 0xe000ef77`, packaged and installed.

## 67. A third-party S60 app through symdev with no edit inside the project (T5)

- **Requires:** experiments 56–61, 63, 64, 66 and the gap work of [third-party-app-puzzles.md](third-party-app-puzzles.md).
- **Procedure:** `git clone https://github.com/tdionizio/puzzless60`, add exactly one `symdev.toml` beside it, run `symdev build && symdev package && symdev run`. Loop: `/tmp/claude-1000/acceptance/run.sh` (outside git); it copies a pristine clone each time and `diff -rq` against the clone afterwards proves nothing in the project changed.
- **Outcome:** pass (emulator; not a device)
- **Evidence:** 2026-09-20. 65 objects (55 `.c`, 11 `.cpp`) compile, 26 libraries link, the 389-line `.rss` and its `_reg` compile, `games.mbm` (209 743 B), `games.mif`, `puzzles.mif` and `puzzles_0xa000ef77.mbg` are produced natively, the SIS self-signs and installs, EKA2L1 logs `Found app: Puzzles, uid: 0xA000EF77`, and the **Cube** puzzle draws its board with the Options/Exit softkeys. `diff -rq` between the pristine clone and the built tree reports no difference outside `build/` and the added `symdev.toml`.
  - **The last supposed blocker was a misreading.** The earlier note said the project's `0xA000EF77` is a protected UID that cannot be self-signed; the recorded policy ([uids-capabilities-signing.md](uids-capabilities-signing.md)) is that `0xA0000000–0xAFFFFFFF` **is** a self-sign range and only `< 0x80000000` is protected. With the project's own UID the run needs no UID change at all. Experiment 66's `secure_id` override remains useful for a project whose `SECUREID` really is unsignable, but Puzzles does not need it.
  - Four `gnumakefile` lines warn and are skipped (exp 64); what they produce is declared in the one `symdev.toml` as `[[icons]]` and `[[install]]`.

## 68. `alloc`: a `GlobalAlloc` over `User::Alloc`/`Free`/`ReAlloc` (T5, Rust SDK)

- **Requires:** experiment 65 (`symdev new --language rust` through `build`/`package`/`run`). Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6, §11 step 68.
- **Procedure:** (1) two scratch probe applications built by `symdev build` and run in EKA2L1, each allocating 16 cells and `InfoPrint`ing the low bits of the addresses, `User::AllocLen` of each cell and the strides — the first over sizes 1…64, the second over 36…257; (2) `symbian-rs/crates/symbian-alloc` with the `GlobalAlloc`, the `#[global_allocator]` and `#[alloc_error_handler]` in `symbian-runtime`; (3) `symbian-rs/examples/alloc`, which grows a `Vec<u16>` and a `String`, builds an `HBuf16`, allocates a 32-aligned `Box`, drops everything and prints numbers derived from the data; (4) the `-Map` file read for what the `mem*` symbols resolved to.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/68-alloc/` (the 11 499-byte E32); source is `symbian-rs/examples/alloc`.
  - **Heap cell alignment is 8, measured, not assumed.** Across 32 cells in two runs every payload address had its low three bits clear (`addr & 0x1f` cycled 00/08/10/18 in run 1 and 00/08/10/18/08/18 in run 2, never 04 or 0c). `User::AllocLen` returned 36, 44, 68, 100, 132, 260 — always `4 (mod 8)` — and the strides between consecutive cells were 0x28, 0x30, 0x48, 0x68, 0x88, always multiples of 8: a 4-byte cell header sits in front of an 8-aligned payload and the cell is rounded to 8 bytes. The minimum payload on this ROM is **36 bytes** (`User::Alloc(1)` and `User::Alloc(36)` both give `AllocLen == 36`, stride 40). The public headers of this SDK do not declare `RHeap` at all — only the inlines survive in `e32cmn.inl`, and `RHeap::Align` reads a per-heap `iAlign` — so the alignment is a property of this thread's heap, not an ABI promise; that is why the allocator trusts 8 and no more.
  - **Over-alignment is padded, deliberately, never silently trusted.** `MAX_TRUSTED_ALIGN = 8`; a larger `Layout::align` allocates `size + align`, places the payload on the first aligned address above the cell and writes the cell's own address in the four bytes below it (there are always at least 8 spare, since the cell is 8-aligned and `align >= 16`). `dealloc` and `realloc` branch on `layout.align()` and hand euser back exactly the cell it returned. Observed: `Box<#[repr(align(32))] [u8; 32]>` came back with `addr % 32 == 0` and its bytes intact.
  - **Failure never unwinds.** `alloc` returns euser's null, so `try_reserve` and the other fallible `alloc` APIs still work; the infallible path goes through `#[alloc_error_handler]` to `User::Exit(KErrNoMemory)` = `-4`. A Rust panic (which would report `-1` and mean nothing to Symbian) is never reached. `alloc_zeroed` uses `User::AllocZ` (`_ZN4User6AllocZEi`) instead of allocating and clearing.
  - **`realloc` is `User::ReAlloc(cell, size, 0)`** for normally aligned blocks. Mode 0 keeping the contents is *observed*, not assumed: the example grows a `Vec<u16>` from empty to 64 elements — several reallocations — and the sum it prints, 85 344, is exactly `sum(i^2, i = 0..63)`. The over-aligned path re-places by hand, because `ReAlloc` may move the cell and promises no more than the heap's own alignment.
  - **Which heap.** `User::Alloc` uses the *calling thread's* heap. A cell may therefore only be freed on the thread that allocated it; `HBuf16` holds a `NonNull`, so it is neither `Send` nor `Sync` and the compiler enforces that. A second thread (`RThread::Create`) gets its own heap unless told to share one; nothing here creates a thread and the allocator is revisited at step 72, not before.
  - **`mem*` resolution observed (spec §4 item 4 settled).** The Rust object references `__aeabi_memclr4` as soon as a program has a local array; the `-Map` file shows that pulling `libaprobe.a(compiler_builtins-….rcgu.o)`, and `memcpy`, `memset` and the whole `__aeabi_mem*` family then resolve **to `compiler_builtins`**, never to euser's strong `memcpy`/`memset`/`memmove`/`memclr` or drtaeabi's `__aeabi_mem*`: the Rust archive sits before the DSOs on the recorded link line. There is no duplicate-definition conflict — the DSO copies are simply not used.
  - **But the pull is all-or-nothing, and that is what `--gc-sections` fixes.** `compiler_builtins` is built as one codegen unit, so the single `__aeabi_memclr4` reference dragged 0x2b338 bytes of `.text` in: the alloc example was **104 560 bytes**. rustc gives every function its own `.text.<symbol>` section, so adding `--gc-sections` to the *Rust* link line (`RustBuild::link_args`; the recorded C++ line is untouched and unit-tested to stay so) brings it to **11 499 bytes** with the same behaviour. `examples/hello-raw` — the experiment-65 program, which pulls no `compiler_builtins` member at all — is still **752 bytes**, byte-for-byte the recorded size, so the flag costs a minimal program nothing.
  - **Ran on the emulated OS:** `[Service.Notifier]: Trying to display: heap 0,1,4,9,16,` (an `HBuf16` used as a `const TDesC16&`) and `[Service.Notifier]: Trying to display: alloc sum=85344 cap=64 a8=0 a32=0 byte=a5 heap=16`, then the known benign `Corrupted graphics command list! Emulation halt.`
  - What this does not show: a second thread or a shared heap, `__UHEAP_MARK`-style leak checking, out-of-memory actually happening (the handler is linked but never ran), a device.
  - Left open for step 70 and later: the residual ~7 kB of a Rust E32 is `compiler_builtins`' globally visible helpers, which `--gc-sections` cannot drop because every dynamic symbol is a collection root in a `-shared` link; `--exclude-libs` is the next thing to try.

## 69. `symbian-core`: system errors and the descriptor family, and a hello with no `unsafe` (T5, Rust SDK)

- **Requires:** experiment 68 (`HBuf16` needs the heap). Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §7, §11 step 69.
- **Procedure:** (1) a C++ probe (`symdev new` console template, the ordinary GCCE path) that `InfoPrint`s the raw header word, `sizeof` and data offset of each 16-bit descriptor class, run in EKA2L1 so the values come from the real euser; (2) `symbian-rs/crates/symbian-core` with `SymbianError`/`ErrorKind` from `e32err.h`, the descriptor family and the safe non-leaving `User::` wrappers; (3) `symbian-rs/examples/hello` rewritten onto them, with the raw version kept as `symbian-rs/examples/hello-raw`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/69-hello/` (the 10 375-byte safe E32 and the 752-byte raw one).
  - **The descriptor type nibbles are observed, not recalled.** `e32des16.h` gives the header word (`KShiftDesType16 = 28`, `KMaskDesLength16 = 0xfffffff`) but not the type enum — every `TPtrC16` constructor is `IMPORT_C`. The probe printed, on the device's own euser: `_LIT16("abc")` `0x00000003`, `TBufC16<8>("ab")` `0x00000002`, `TPtrC16("abcd")` `0x10000004`, `TPtr16(p,3,8)` `0x20000003`, `TBuf16<8>("abc")` `0x30000003`, `HBufC16::New(8)` `0x00000000`. So **`EBufC = 0`, `EPtrC = 1`, `EPtr = 2`, `EBuf = 3`**; 65a's literal nibble 0 is the `EBufC` case.
  - **Sizes and data offsets from the same run:** `sizeof` `TDesC16` 4, `TPtrC16` 8, `TDes16` 8, `TPtr16` 12, `TBufC16<8>` 20, `TBuf16<8>` 24, `TBufC16<7>` 20, `TBuf16<7>` 24, `HBufC16` 8. Data at `+4` for `TBufC16` and `HBufC16`, `+8` for `TBuf16`. `TPtrC16` word 1 is the data pointer; `TPtr16` word 1 is `iMaxLength` and word 2 the pointer; `TBuf16` word 1 is `iMaxLength`. `User::AllocLen(HBufC16::New(8))` is 36 — the heap minimum of experiment 68 again.
  - **The Rust types and what each one is:** `Buf16<N>` is `TBuf16<N>` (`EBuf`, `iMaxLength = N`, units at +8; `size_of::<Buf16<8>>() == 24` is a `const` assertion in the crate). C++ rounds an odd `N` up one unit (`__Align16`), which changes only `sizeof`; the Rust array is exactly `N` units and `iMaxLength` is `N`, so every euser write stays inside it. `PtrC16<'a>` is `TPtrC16` over a borrowed `[u16]` (`EPtrC`, pointer in word 1), with the borrowed slice kept *after* the two C-visible words so the type needs no `from_raw_parts`. `HBuf16` is `HBufC16`: one heap cell, `EBufC` header word then the units at +4, grown through the allocator's `realloc`. `Des16<'a>` is `&'a dyn DesC16` — the Rust half of `const TDesC16&`. `EBufCPtr` (the `RBuf16` variant) was **not observed and has no type**.
  - **Errors:** `SymbianError` keeps the raw `TInt`; `ErrorKind::of` names all 49 codes of `e32err.h` (`KErrNone` 0 down to `KErrCommsBreak` -48, contiguous) and `Unknown(TInt)` carries a component-specific code unchanged. `check(code)` turns a negative `TInt` into `Err`; a positive one is a result, not an error.
  - **`&str` ↔ UTF-16 without allocating:** `encode_utf16_into`/`decode_utf16_into` write into a caller-provided buffer and report `KErrOverflow` (too small) or `KErrArgument` (unpaired surrogate). `core::fmt::Write` on `Buf16` and `HBuf16` makes `write!` work; an overflow is `fmt::Error` there and `KErrOverflow` through `push_str`.
  - **The example writes no `unsafe`.** `examples/hello` builds its note with `write!(note, "{GREETING} ({} chars)", GREETING.len())` into a `Buf16<64>` and calls `symbian_core::user::info_print`; the file contains no `unsafe` block and names no C function. In EKA2L1: `[Service.Notifier]: Trying to display: Hello from Rust SDK (19 chars)`. `examples/hello-raw` still prints `Hello from Rust SDK`. All the `unsafe` in the SDK is now in `symbian-sys` (declarations), `symbian-alloc`, `symbian-runtime`, `symbian-core/src/user.rs` (the three euser calls) and `symbian-core/src/des/hbuf16.rs` (the heap cell), each block with a `// SAFETY:` note.
  - **E32 sizes** next to experiment 65's 752 bytes: `hello-raw` **752** (unchanged), `hello` **10 375**, and **8 086** for the same program with `push_str` instead of `write!` — so `core::fmt` costs about 2.3 kB and everything else (the heap, the descriptors, `compiler_builtins`' surviving helpers) about 7.3 kB. The `alloc` example is 11 499.
  - The acceptance path still holds: `symdev new hello --language rust && symdev build && symdev package && symdev run` from an empty directory produces the same 10 375-byte E32 and the same notifier line (the scaffold now writes a `symbian-core` dependency beside `symbian-runtime`).
  - What this does not show: anything that leaves (that is step 70, and every leaving API is deliberately absent rather than guessed), `TDes16&` out-parameters filled by euser, `RBuf16`, the 8-bit descriptor family, a device.

## 72. Concurrency primitives on Symbian 9.3 / ARMv5TE: atomics, locks, threads (T5, Rust SDK)

- **Requires:** the SDK on this host, the recorded GCCE compile/link argv (exp 5, 51, 59), the pinned nightly and `symbian-rs/targets/arm-symbian-e32.json` (exp 65), EKA2L1 via `symdev run`. Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §4, §8, §10 item 2, §11 step 72. Full survey: [eka2-concurrency.md](eka2-concurrency.md).
- **Procedure:** (1) `nm -D euser.dso` and a sweep of `epoc32/include` for the `__e32_atomic_*` family, `User::LockedInc/Dec`, `User::SafeInc/Dec` and the `R*` lock classes; (2) GCCE probes for `__atomic_load_n` / `fetch_add` / `compare_exchange_n` / `exchange_n` at 8/16/32/64 bits and every ordering, at `-march=armv5t|armv5te` × Thumb|ARM, then `nm` over every archive and DSO of the recorded link line, then an actual link; (3) the same in Rust on a scratch target JSON with `max-atomic-width: 32` / `atomic-cas: true`; (4) C++ probes run in EKA2L1 for the four euser atomics' return values and predicate, their atomicity against a second `RThread`, the five lock classes' creation/recursion/timeout behaviour and their uncontended cost; (5) a `__atomic_*` shim over one `RFastLock`, exercised from both C++ and the Rust archive.
- **Outcome:** pass (survey; no target-JSON change, and no device)
- **Evidence:** 2026-09-20, scratch `/tmp/claude-1000/atomics-work/` (outside git). Details and the UNKNOWN list: [eka2-concurrency.md](eka2-concurrency.md).
  - **9.3 has no atomics header.** No `e32atomics.h` in this SDK (the `__e32_atomic_*` family is 9.4+); the only `*atomic*` file is glib's `stdapis/glib-2.0/glib/gatomic.h`, whose `g_atomic_*` are `IMPORT_C` DLL calls. `nm -D euser.dso | grep -cE '__sync|__atomic|__e32_'` = 0.
  - **The whole atomic surface is four `TInt` counters**, declared in `e32std.h:4518` under `// Atomic operations` and exported as `_ZN4User9LockedIncERi`, `_ZN4User9LockedDecERi`, `_ZN4User7SafeIncERi`, `_ZN4User7SafeDecERi`. Observed: all four return the **old** value; `Locked*` is an unconditional ±1, `Safe*` does the ±1 **only when the old value is > 0** (0, −1 and −5 are left alone) — the reference-count semantics the SDK's own `f32fsys.h:231` relies on. The headers state **no memory ordering** for any of them (UNKNOWN).
  - **`LockedInc` really is atomic** in the emulated OS: two threads × 20000, each iteration doing `LockedInc(g)` beside a hand-rolled `x=p; User::After(0) every 64; p=x+1`, gave `LockedInc=40000 plain=20000`. Without the forced yield EKA2L1 never preempts the loop and both come out exact, so the yield is what makes the test mean anything.
  - **GCCE 12.1:** relaxed 8/16/32-bit load/store is inline; acquire/release/seq_cst adds `bl __sync_synchronize`; 64-bit load/store and **every** RMW at every width and ordering are libcalls; no `SWP` is ever emitted; `-march=armv5t` vs `armv5te` and Thumb vs ARM make no difference.
  - **Nothing on the recorded link line defines any of it.** `nm` over `libgcc.a`, `libsupc++.a`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, `drtaeabi.dso`, `scppnwdl.dso`, `drtrvct2_2.dso`: zero `__atomic_*`/`__sync_*`. Linking an `E32Main` that uses them fails with `undefined reference to '__atomic_fetch_add_4' / '__atomic_compare_exchange_4' / '__sync_synchronize'` ×2 — so today even an *acquire load* is a link error.
  - **LLVM is stricter than GCC:** on this target it lowers **every** atomic op to a libcall, including a relaxed load (`probe_load_relaxed` is `bl __atomic_load_4`, `r1 = 0`). `compiler_builtins` defines none of them. At `max-atomic-width: 0` the types do not exist at all (`no AtomicU32 in sync::atomic`, `cannot find sync in alloc` — no `Arc`); at 32/true they exist and the link fails with seven undefined `__atomic_*`.
  - **Libcall ABI, read off the emitted code and identical for GCC and LLVM:** `load_N(ptr, mo)`, `store_N(ptr, val, mo)`, `exchange_N(ptr, val, mo)`, `fetch_*_N(ptr, val, mo)`, `compare_exchange_N(ptr, expected_ptr, desired, success_mo, failure_mo)` — **five** arguments, the builtin's `weak` flag is not passed. mo: 0 relaxed, 2 acquire, 3 release, 4 acq_rel, 5 seq_cst.
  - **A lock-backed shim closes the gap, for Rust too.** ~110 lines over one process-wide `RFastLock` (each definition needs an `__asm__("__atomic_…")` label, or GCC reports `ambiguates built-in declaration`). C++: `add8 7→10 r=7`, `xchg16 r=7 v=9`, `add64 7→107 r=7`, a failing `cas8`, and across two threads `fetch_add` = 40000 exact / CAS loop = 400 wins exact while the plain RMW beside it lost half. Rust: the `max-atomic-width: 32` archive linked through `symdev build` (MMP `LIBRARY librprobe.a`) into a 5423-byte EXE, and `AtomicU32::fetch_add` from two threads gave **exactly 40000**.
  - **Locks.** All non-leaving, all kernel handles. `RFastLock`: `CreateLocal` only, process-local, `sizeof` 8, **not recursive** — a second `Wait` from the owner blocks forever (observed). `RMutex`: local or global by name, **recursive** — `Wait` twice from the owner returns, `IsHeld()` 0→1 (observed). `RSemaphore`: local or global, and the only timed wait — `Wait(50000)` on an empty semaphore returns **−33 `KErrTimedOut`**, 0 with a token. `RCriticalSection`: `CreateLocal` only, `IsBlocked()` 1 inside. `RCondVar`: `CreateLocal` returns `KErrNone` but `Handle()` is 0 (suspected EKA2L1 gap, UNKNOWN). Uncontended cost per 100 000 iterations at 32768 Hz, **emulator ratios only**: plain 7, `LockedInc` 238, `RMutex` pair 349, `RFastLock` pair 523, shim `fetch_add` 641 — note `RMutex` measuring *cheaper* than `RFastLock` contradicts the header and is almost certainly an emulator artefact.
  - **Decision: the target JSON is left at `max-atomic-width: 0` / `atomic-cas: false`.** Raising it is correct *only* in the change that also puts the shim on the link line; on its own it converts a compile error into a link error. Recommended then: 32 (not 64) with `atomic-cas: true`, `Mutex` on `RFastLock` (`try_lock` on `RSemaphore`'s timeout, since neither `RFastLock` nor `RMutex` has one), `Once` on a shim `compare_exchange` plus its own `RFastLock`, threads on `RThread::Create` + `Logon`. `Arc` and `core::sync::atomic` do not exist until the shim lands, and once it does every operation on them takes a process-global lock (~90× a plain increment in the emulator), so `Rc`/`RefCell` remain the right tools for single-threaded code.
  - **Emulator notes:** after a worker `RThread` exits, the main thread takes an `Access violation reading address 0x8000A4` on every threaded probe (single-threaded probes never fault) — cause unidentified. EKA2L1 also refuses to install a package whose executable is already on drive E (logs `Installation done!`, then the front end prints `Installation of SIS failed` and never runs it), so each probe needs a fresh app name **and** UID3.
  - What this does not show: anything on a device; the ordering guarantees of the four euser atomics; whether `__sync_synchronize` must be more than a compiler barrier; `RMutex` priority inheritance; whether `RCondVar` works.

## 76. The Avkon shim ABI: a Thumb C++ shim forwarding to ARM Rust (T5, Rust SDK)

Numbers 68–75 are the Rust SDK steps of the [design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §11 and are claimed by other work; this record takes **76** so nothing collides. It is the probe series behind [avkon-rust-spec.md](avkon-rust-spec.md), the design for step 75.

- **Requires:** experiment 65 (the `arm-symbian-e32` target and `symbian-rs/`), the recorded GCCE compile and link argv, the native post-linker, `symdev package`/`run`, `examples/gui` as the C++ control.
- **Procedure:** three throwaway builds under `/tmp/claude-1000/ui-spec-work/` (outside git), each a Rust `staticlib` on `arm-symbian-e32` (nightly-2026-09-19, `-Zbuild-std=core`) hand-linked beside a C++ object that symdev's own `build` produced with the observed GCCE argv, post-linked by the recorded `elf2e32` argv, then packaged and run by symdev.
  - **A+** `probe76/`: a console EXE whose C++ `E32Main` calls Rust through a direct call and whose Rust code calls back through a `#[repr(C)]` table of `extern "C"` function pointers; the leaving call it reaches is wrapped in `TRAPD` **inside** the shim.
  - **A−** `probe76/raw/`: the same image with `MACRO PROBE_RAW`, so the shim calls `User::Leave(-6)` with a Rust frame on the stack.
  - **B** `uiprobe/` + `uirust/`: a full Avkon application — a 203-line C++ shim subclassing `CAknApplication`, `CAknDocument`, `CAknAppUi` and `CCoeControl` and forwarding `create/destroy/construct/draw/offer_key/command/size_changed` to a Rust vtable, with a 174-line `#![no_std]` Rust half that draws a bar chart; resources and the icon built by `symdev build` from a `.mmp`, the link done by hand because symdev links exactly one object today.
- **Outcome:** pass for the ABI and the leave rule; **key input could not be tested** (emulator only; no device).
- **Evidence:** 2026-09-20.
  - **A+ passed in EKA2L1.** `build/eka2l1.log`: `Trying to display: Shim: calling Rust`, `Rust: entered from Thumb shim`, `Rust: trapped leave returned -6`, `Shim: back from Rust`. So a `TRAPD` inside the shim turns `User::Leave(-6)` into the return value -6 and the process continues.
  - **Interworking settled for this boundary** (experiment 62 remains unrun for the general case). `readelf -A shim.o`: `Tag_THUMB_ISA_use: Thumb-1` — symdev compiles C++ as Thumb (`-mthumb -mthumb-interwork`), while rustc emits ARM. The linker rewrites the shim's `bl probe76_run` as `blx <probe76_run@plt>` with an ARM PLT stub (`ldr pc, [pc, #-4]`), and rustc's indirect calls are `blx r1`. No hand veneer, no flag change. New warning on every such link: `uses 4-byte wchar_t yet the output is to use 2-byte wchar_t` (`Tag_ABI_PCS_wchar_t` 4 vs 2); nothing in the ABI crosses as `wchar_t`.
  - **A− is a silent death.** The log stops at `Rust: about to let a leave cross this frame` and the process disappears — no panic, no `KERN-EXEC`, no line after it. `readelf --unwind rawprobe.elf` shows 20 `.ARM.exidx` entries of which one, `0x813c … 0x331d4`, is `0x1 [cantunwind]` and covers the whole Rust text region, while every shim function carries `__gxx_personality_v0`. `e32cmn.h` line 5933 with `__LEAVE_EQUALS_THROW__` (`variant/symbian_os_v9.3.hrh` line 651) makes a leave a real C++ exception, so this is the unwinder meeting a `cantunwind` frame. **The rule "no Rust frame on the stack when a leave flies" is enforced by nothing and fails without a diagnostic.**
  - **B renders.** Screenshot `/tmp/claude-1000/ui-spec-work/uiprobe-1.png`: the S60 title pane reads `uiprobe`, the Exit softkey is drawn, and the client area shows the three coloured bars, the baseline and the axis that the **Rust** `draw` callback painted through the host function-pointer table. `Found app: uiprobe, uid: 0xE7351C79` in the log; the app stayed alive until killed.
  - **B's key half is unproven.** No key reached the emulated device: XTest with the emulator window activated (`_NET_ACTIVE_WINDOW` sent, `XGetInputFocus` confirming) and the pointer over the screen changed nothing, and the stock Exit softkey (F2 → `EStdKeyDevice1` per `~/.local/share/EKA2L1/bindings/default.yml`) did nothing either. A `User::InfoPrint` at the top of the shim's `OfferKeyEventL` never fired. A way to drive keys into EKA2L1 is now a prerequisite for step 75's acceptance.
  - **Cost, measured.** `gui.o` (the four-class C++ example) has 182 undefined symbols; the forwarding shim has 185 — subclassing cost is fixed and forwarding is nearly free. `shim.o` 30 996 B. **But the E32 is 107 028 B** against 752 B for the Rust `examples/hello` and 3591 B for the same shim with a C++ stand-in: one C++ object next to the Rust archive pulls the whole `compiler_builtins` member, a single compilation unit carrying libm and every `__aeabi_*`. No symbol collision arose (design spec §4 hypothesis 4); the problem is granularity, and it needs its own experiment before step 75 ships.
  - **Key codes and event types**, read by compiling `e32keys.h` rather than recalled: `EKeyDevice0` 0xf842 / `EKeyDevice1` 0xf843 (softkeys), `EKeyDevice3` 0xf845 (selection), arrows 0xf807–0xf80a, `EKeyYes` 0xf862, `EKeyNo` 0xf863, `EKeyMenu` 0xf836; scan codes 0xa4/0xa5/0xa7, 0x0e–0x11, 0xc4, 0xc5, 0x94. `TKeyEvent` is four words (`w32std.h` 974), `TEventCode` `EEventKey = 1` (line 266), `TKeyResponse` `EKeyWasConsumed = 1` (`coedef.h` 24).
  - **Import census of a minimal Avkon app** (`gui.elf`, by `NEEDED` DLL): cone 67, eikcore 58, avkon 38, drtaeabi 19, euser 11, apparc 6, scppnwdl 1, gdi 1. Drawing through the `CWindowGc&` costs **no** library — every `CGraphicsContext` drawing entry point is a pure virtual, and `ws32` is absent from both the `LIBRARY` line and `NEEDED`; `gdi` is imported for `CFont::AscentInPixels` alone.
  - **Not diagnosed:** the first `symdev run` of probe A logged `Installation done!` and then `Installation of SIS failed`; an identical second run installed and ran.

## 77. What a Rust binary costs against the same program in C++ (T5, Rust SDK)

- **Requires:** experiments 65, 68, 69, 76.
- **Procedure:** build the *same* program twice, once in each language, through `symdev build`, and compare the E32. Two pairs: (a) three euser calls and nothing else, (b) the same plus a formatted string. Then read where the bytes went (`arm-none-symbianelf-size -A`, `nm --size-sort -S`) and try the obvious levers on the link.
- **Outcome:** pass (measurement; no device)
- **Evidence:** 2026-09-20.

  | Program | C++ | Rust | Ratio |
  |---|---|---|---|
  | `User::InfoPrint` + `User::After` + `return 0` | **746 B** | **752 B** | 1.01× |
  | the same, text built with `TBuf::Format` / `write!` into a `Buf16` | **803 B** | **10 375 B** | 12.9× |
  | the second one with `push_str` instead of `write!` | — | 8 086 B | — |
  | a `Vec`/`String` exercise (`examples/alloc`) | — | 11 499 B | — |

  - **With no formatting the two languages are the same size.** A `no_std` Rust EXE that calls euser directly is six bytes larger than its C++ twin, so the target, the entry point, the link and the post-link cost nothing. The whole difference is library code Rust brings and C++ takes from the ROM.
  - **The jump is `core::fmt` plus `compiler_builtins`.** `size -A` on the 10 375-byte image: `.text` 15 076 (pre-compression), `.rodata` 268, `.plt` 96. By symbol, the largest are `compiler_builtins`' soft-float and 64-bit division helpers — `__divdf3` 0x420, `__adddf3` 0x38c, `__muldf3` 0x344, `__divsf3` 0x24c, `__addsf3` 0x210, `u64_div_rem` 0x27c — then `memmove` 0x3dc and `memcpy` 0x1b0, then `core::fmt::write` and the `Display` impls. **A program that does no floating-point arithmetic still carries the soft-float routines**, because `compiler_builtins` is one codegen unit and the first `__aeabi_mem*` reference pulls the object.
  - `--gc-sections` on the Rust link (experiment 68) already took this case from 104 560 B to 10 375 B. Two further levers were tried and **neither works**: `--exclude-libs ALL` changes nothing (ld 2.29.1 applies it to archives named with `-l`, and the Rust archive is a positional path), and an anonymous `--version-script` is refused outright — `anonymous version tag cannot be combined with other version tags`, and `--default-symver` is part of the recorded link line.
  - **The worst case is mixed C++ and Rust**: experiment 76's UI probe, one C++ object beside the Rust archive, came to 107 KB. Same cause at a coarser granularity, and it is the case every shim-using program will be in, so it needs solving before step 70 ships.
  - **Solved, by reading the map instead of guessing at the link.** `Archive member included to satisfy reference by file (symbol)` names the culprit exactly: `libhello.a(compiler_builtins…rcgu.o)` is pulled by `libhello.a(hello…rcgu.o) (__aeabi_memclr4)`. But the phone already has that routine: `euser.dso` exports `memcpy`/`memset`/`memmove`/`memclr` and `drtaeabi.dso` the whole `__aeabi_mem*` family, both already on the link line — **after** the archive, which is the only reason the linker reached into it. Naming those two DSOs *before* the archive resolves the helpers from ROM and the `compiler_builtins` member is never pulled (`grep -c compiler_builtins` on the map: 3155 → 0).

  | Program | before | after | C++ twin |
  |---|---|---|---|
  | `hello` (safe, `write!`) | 10 375 B | **3 187 B** | 803 B |
  | `examples/alloc` (`Vec`, `String`) | 11 499 B | **4 324 B** | — |

  Both still run: `Hello from Rust SDK (19 chars)`, and `heap 0,1,4,9,16,` / `alloc sum=85344 cap=64 …` unchanged. The remaining gap to C++ is `core::fmt` and the allocator, which is code the program actually uses.
  - `-Zbuild-std-features=` (dropping `compiler-builtins-mem`) changes **nothing** — measured, the archive is byte-for-byte the same size and the member is still pulled, because the reference is `__aeabi_memclr4` from our own object, not from `mem`.
  - `--exclude-libs ALL` changes nothing (ld 2.29.1 applies it to archives named with `-l`, ours is a positional path), and an anonymous `--version-script` is refused: `anonymous version tag cannot be combined with other version tags`, and `--default-symver` is part of the recorded line.
  - This also disposes of the 107 KB mixed C++/Rust case of experiment 76: same cause, same fix, and it is now fixed before step 70 puts every program in that configuration.
  - **Where the remaining 3 187 bytes go, measured by removing the one suspect.** The same `hello` with `push_str` instead of `write!`, so nothing pulls `core::fmt`, is **809 bytes** — six bytes from the C++ twin's 803, exactly the gap the no-formatting pair showed. So the whole difference is `core::fmt`: **2 378 bytes**, and nothing else.
  - By symbol (`nm --size-sort -S` on the ELF, `.text` 3 524 before the E32's deflate): `<usize as Display>::fmt` 908, `<&str as Display>::fmt` 784, `core::fmt::write` 536, `Formatter::padding` 192, `pad_integral::write_prefix` 100 — plus `symbian-core`'s `fmt::Write` adapter for `Buf16` (`write_str` 444, `write_char` 192). The Symbian startup stub (`_E32Startup`, `CallThrdProcEntry`, `__cpp_initialize__aeabi_`, ~310 bytes) and our own `E32Main` (172) are in the C++ binary too.
  - **C++ pays nothing for formatting because it does not carry a formatter**: `TDes16::Format` and `AppendNum` are euser exports, i.e. ROM. That is the lever if a program needs to be small: a shim over `TDes16::Num`/`AppendNum`/`Append` (step 70) would let `symbian-core` offer number and string appending at no binary cost, leaving `core::fmt` for programs that ask for it by using `write!`.

## 78. The C++ shim: which calls need one, and a leave that comes back as a value (T5, Rust SDK)

- **Requires:** experiments 65, 68, 69, 76, 77. Design: [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §7, §11 step 70.
- **Procedure:** (1) C++ probes compiled with the recorded GCCE argv and disassembled, to settle the non-static member ABI and the sizes of `RFs`/`RFile`; (2) `symbian-rs/shims/common/` with the rule in its header and two `TRAP` wrappers, compiled by `RustBuild` with `GcceBuild`'s own C++ argv and archived beside the Rust archive; (3) `symbian-sys` declarations for the euser `TDes16` members, the `RFs` members and the shim entry points; (4) `symbian-core`'s `FileServer` and the capacity-checked descriptor operations; (5) `symbian-rs/examples/shim` through `symdev build`/`package`/`run`; (6) the same image with the `TRAP` removed, as a throwaway outside git.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/78-shim/` (the 4 475-byte E32); source is `symbian-rs/examples/shim`.

  - **The rule, as it is now written** (in full in `symbian-rs/shims/common/symrs_shim.h`, and in the design spec §7). A call needs a C++ wrapper when any of three things is true, and otherwise `symbian-sys` declares it and Rust calls it directly:
    1. **it can leave** — confirmed against the SDK header, not from the trailing `L`, and a header that says nothing counts as leaving;
    2. **its signature is not a C signature** — a class returned by value with a non-trivial copy constructor (sret), a `TRefByValue` varargs function such as `TDes16::Format`, a virtual call;
    3. **it is a virtual member, or a member of a class with multiple or virtual inheritance**, where `this` may need adjusting — not observed here, so not attempted.
  - **Being a non-static member function is *not* a reason, and that is observed, not guessed.** `probe_append(TDes16* d, const TDesC16* s) { d->Append(*s); }` compiled with the recorded GCCE argv is a bare `bl _ZN6TDes166AppendERK7TDesC16` with **no register shuffle at all**; `d->AppendNum((TInt64)n)` emits `movs r2,r1; asrs r3,r1,#31` — the 64-bit argument in r2:r3, r1 skipped. So `this` is argument 0 under the ordinary AAPCS assignment and the rest follow. `d->Find(*s)` likewise, result in r0. `MaxLength()`/`Length()` are inline and make no call. The **one exception, also observed**: `d->Left(n)` returning `TPtrC16` by value is sret — `mov r0,sp; movs r1,r0; movs r2,n` — the hidden return slot is argument 0 and `this` moves to argument 1. Probes: `scratchpad/shim70/memberabi{,2}.cpp`.
  - **A panic is not a leave and no `TRAP` catches it.** `e32panic.h` line 131: `ETDes16Overflow = 11`, category USER, documented for "any of the copying, appending or formatting member functions". So `symbian-core`'s `append_num`/`append_des`/`copy_des` check the room first — from the length word and `N` they already hold, with no call — and return `KErrOverflow`. A Rust caller cannot reach the panic.
  - **Build integration.** `symbian-rs/shims/common/*.cpp` is compiled by `RustBuild` with `GcceBuild::compile_args` — the identical C++ argv a project source gets, so the shim sees `gcce.h`, the GCC-12 varargs repair and every define — into `build/shims/*.o`, archived as `build/shims/libsymrs.a` and placed immediately after the Rust archive. The application names none of it. `ar` is **derived from `SYMDEV_LD`** (`…-ld` → `…-ar`, `SYMDEV_AR` overrides); no new required environment variable.
  - **An archive and not loose objects, for a measured reason.** With the objects on the link line, `hello` went 3 187 → 3 219 bytes and gained `bafl{000a0000}.dso` as a `DT_NEEDED` although it calls nothing in the shim. Hidden visibility (`SYMRS_EXPORT`) plus `--gc-sections` did remove the unused wrapper's code — `nm` finds no `symrs_*` in the ELF — but ld 2.29.1 decides `--as-needed` during symbol resolution, **before** garbage collection, so the dependency outlived the code that justified it. From an archive the member is never pulled: `hello` is **3 187 bytes again** with the same six `NEEDED` entries as before. The two extra DSOs the SDK may need (`efsrv.dso`, `bafl.dso`) go on under `--as-needed`/`--no-as-needed`; `shimdemo`, which uses both, has eight.
  - **The `-l:euser.dso -l:drtaeabi.dso`-before-the-archive ordering of experiment 77 is untouched** and still unit-tested; the 107 KB mixed C++/Rust case did not come back. Sizes after this step: `hello` **3 187** (unchanged), `hello-raw` **752** (unchanged), `examples/alloc` **4 320** (4 below the 4 324 of experiment 77 — `symbian-core` gained two modules and the crate is built with LTO and one codegen unit, so the archive's layout moved; not diagnosed further), `examples/shim` **4 475**.
  - **The leave, end to end.** `[Service.Notifier]: Trying to display: shim70 mkdirall=0 trapped=-12 bad=0 ensured=0 sign=-42 alive`. `trapped=-12` is `User::LeaveIfError(-12)` raising a real C++ exception inside the shim, the `TRAP` catching it, and `KErrNotFound` arriving in Rust as an `Err` — and everything printed after it is the proof that the process carried on.
  - **The counter-proof, in a throwaway outside git** (`scratchpad/shim70/rawleave`, the same SDK with the `TRAP` deleted): the log stops at `rawleave: about to leave untrapped` and there is nothing after it — no `still alive`, no panic, no `KERN-EXEC`. Experiment 76's silent death, reproduced with today's toolchain, which is what the shim is buying.
  - **EKA2L1's file server accepts paths a phone would refuse.** A throwaway probe put `Z:\…`, `Y:\…`, `Q:\…` and a `*` inside a component through `BaflUtils::EnsurePathExistsL`; every one returned `KErrNone`, and `RFs::MkDirAll` on `E:` did too. That is why the deterministic leave in the example is `User::LeaveIfError` and not a bad path. Whether the file call leaves on a device is **untested**.
  - **Measured for step 71:** `sizeof(RFs) == 4` and `sizeof(RFile) == 8` (`return sizeof(RFs);` compiled with the recorded argv gives `movs r0,#4`); `RFs::Close()` compiles to `_ZN11RHandleBase5CloseEv`; `RFs::Connect()`'s default argument is `-1` (`movs r1,#1; negs r1,r1`); `KMaxFileName` is `0x100` (`e32const.h` line 390). `f32file.h` declares **no** leaving member on `RFs` at all, so the whole file API of step 71 is shim-free.
  - **euser's number formatting works and is free.** `sign=-42` shows `TDes16::AppendNum(TInt64)` renders the minus sign, so the wrapper's `decimal_len` bound holds. `e32des16.h` has no `AppendNum(TInt)`: the integer overloads are `AppendNum(TInt64)` and `AppendNum(TUint64, TRadix)`.
  - What this does not show: a device; a leave from the real file API; `TDes16::Format` (varargs, shim-only, not written); the virtual and multiple-inheritance cases of rule 3; `RFile`.

## 79. Files as `symbian_std::fs`, and a test harness an example reports through (T5, Rust SDK)

- **Requires:** experiments 68, 69, 77, 78. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a: the developer-facing API is `std`-shaped, the Symbian-shaped one is the escape hatch.
- **Procedure:** read `f32file.h` and `efsrv.dso` for what the file API is and whether any of it leaves; measure the 8-bit descriptor and `TEntry` layouts with a compile probe; build `symbian-std` (`io::{Read, Write, Seek, Error, ErrorKind}`, `fs::{File, OpenOptions, Metadata, …}`, `prelude`); add the `E:\symdev\results\<uid3>.json` protocol and `symdev test --emulator`; run `examples/files` in EKA2L1 and check the bytes from the host.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20.
  - **The whole file API is shim-free.** `f32file.h` declares **no leaving member** on `RFs`, `RFile`, `RDir` or `TEntry`; every `IMPORT_C … L(` in that header is on `CDir`, `CDirScan`, `CFileBase`, `CFileMan` or `TOpenFileScan`. The step-70 mechanism is there for when that stops being true, and was not needed here.
  - **The 8-bit descriptor type nibbles were never observed** (experiment 69 observed only the 16-bit family), so nothing about the header word is assumed: `PtrC8`/`Ptr8` are built by euser's own exported constructors, `_ZN6TPtrC8C1EPKhi` and `_ZN5TPtr8C1EPhii`, called as ordinary members with `this` as argument 0 (experiment 78's ABI).
  - Layouts measured with a compile probe on the recorded GCCE argv: `sizeof` `TDesC8` 4, `TDes8` 8, `TPtrC8` 8, `TPtr8` 12, `RFile` 8, `TEntry` 552 with alignment 8 and `iAtt`/`iSize`/`iModified`/`iType`/`iName` at 0/4/8/16/28.
  - **`memcmp` is defined nowhere on the recorded link line** — `nm -D` over euser, drtaeabi, dfpaeabi, scppnwdl and drtrvct2_2 finds `memcpy`/`memset`/`memmove`/`memclr` and the `__aeabi_mem*` family but no `memcmp` or `bcmp`, and LLVM emits one for `a == b` on two `[u8]`, so comparing byte slices did not link at all. Two fixes measured: `-Zbuild-std-features=compiler-builtins-mem` works but costs **4.6 kB** (experiment 77's one-codegen-unit problem again, and `--gc-sections` cannot drop weak *global* symbols, which are collection roots in a `-shared` link); a five-line `shims/common/symrs_cstring.cpp` instead took `files` from 15 595 to **10 423** bytes and the map's `compiler_builtins` count from 3 162 to **0**, leaving `hello` 3 187, `hello-raw` 752, `alloc` 4 320 and `shim` 4 475 untouched.
  - **The harness reports failure, which is the part worth proving.** All passing: `filesdemo: 16 passed`, exit 0. With two deliberate failures added: `FAIL deliberately wrong`, `FAIL open a file that is not there: NotFound (KErrNotFound (-1))`, `filesdemo: 2 failed, 16 passed`, exit **1**. A report that never arrives is also exit 1 (`the application never wrote one` after the timeout).
  - **Checked from outside the emulator:** `~/.local/share/EKA2L1/data/drives/e/symdev/files71/kept.bin` is 46 bytes and byte-identical to what the example wrote, em-dash (`e2 80 94`) included.
  - **A duplication that cost a confused run, and is now gone.** The example named its own UID3 in the source as well as in `symdev.toml`; changing one and not the other made `symdev test` wait for a file nobody was writing. `symdev build` now puts the manifest's value in cargo's environment as `SYMDEV_UID3` and `Report::new(app)` reads it at compile time, so the UID is written once. `Report::with_uid3` remains for an application that wants to name its own.
  - A host test regression found while finishing this: `current_dir(project())` dropped the `TempDir` — and deleted the directory — before the command was spawned. Bound to a variable, with the reason in a comment.
  - Caveat recorded: EKA2L1's file server accepts paths a phone would refuse (experiment 78), so nothing here says how a device behaves.

## 83. Why does no key press reach the guest in EKA2L1, and how does a test send one? (M5, UI acceptance)

- **Why:** [avkon-rust-spec.md](avkon-rust-spec.md) §9 closed with "no key of any kind could be delivered to the emulated device from this session" — not the arrows, and not the **stock** Exit softkey — which blocks every UI acceptance test and step 75's accept. The previous session had ruled out the application (the stock softkey failed too) but not the host or the emulator.
- **Procedure:** establish, in order, whether it is how we drive the emulator, whether EKA2L1 has a supported way in (`--help`, the scripting interface), or whether our fork needs an option. Instrument the Qt and window-server input path if needed, then drive a real application and observe the effect in pixels.
- **Outcome:** pass — **it is how we drive it.** No emulator change; nothing was added to our fork.
- **Evidence:** 2026-09-20, [eka2l1-input.md](eka2l1-input.md); scratch driver in the session scratchpad.
  - **XTEST is silently dropped on this host.** The session is GNOME on Wayland (`Xwayland` + `mutter-x11-frames`); EKA2L1 is an X11 client under Xwayland, and a synthetic XTEST event is routed by the compositor to the Wayland-focused surface, not to the X window that owns the X input focus. `XTestQueryExtension` says 2.2 is present, `XGetInputFocus` names the emulator before and after every key, Qt paints a caret in its own Search box — and three `XTestFakeKeyEvent` letters leave that box empty. That is the whole of the earlier finding.
  - **`XSendEvent` to the toplevel works.** A `KeyPress`/`KeyRelease` pair addressed to EKA2L1's toplevel X window bypasses the compositor, and Qt's xcb plugin dispatches a `send_event` key like a real one. Send to the toplevel, never to one of the eight or nine `Qt::WA_NativeWindow` children — keys addressed to those went nowhere.
  - **The guest path was never broken.** Temporary probes (reverted; `~/src/EKA2L1` clean) trace one F1 through `display_widget::keyPressEvent` (`key=16777264`), `main_window::deliver_key_event` (touch overlay declines), `window_server::handle_input_from_driver` (`mapped to scancode 164`) and `window_key_shipper::start_shipping` (`1 events, focus group id 2`). `init_key_mappings` loaded 22 keybinds from `bindings/default.yml`; §5.2's binding table is correct.
  - **A symdev-built application observes keys.** `examples/gui` copied to scratch with an `OfferKeyEventL` that counts presses and draws them, built and installed with `symdev build/package/run` on a clean emulator build: `Down Left Left` renders **`keys 3 code f807 scan 0e`** (`EKeyLeftArrow`/`EStdKeyLeftArrow`), and `Down Down Up Return` renders `keys 4 code f845 scan a7` (`EKeyDevice3`). ROM Notes (`0x10005907`) opens its Options menu on F1 and moves the highlight on `Down`, 3/3 cold launches.
  - **Softkeys remain dead in our applications.** Scancodes 164/165 are shipped to the focus group, but `examples/gui` does not exit on its `R_AVKON_SOFTKEYS_EXIT` softkey and the counting application never sees F2 in `OfferKeyEventL`, while the same scancodes drive Notes' softkeys. **Unknown — requires experiment.** Write acceptance tests against arrows, selection and digits until it is understood.
  - **A latent fragility, not patched.** `display_widget` never calls `setFocusPolicy`, so its policy is `Qt::NoFocus`, and `switch_to_game_display_mode()` calls `displayer_->setFocus()` while there is no active window (`hasFocus=false focusWidget=(null) activeWindow=(null)`); the widget only really holds focus after the toplevel is activated once. Activating the window before sending keys covers it, and no failure was ever reproduced from it, so no fix was invented.

## 82. Can LLVM's `lld` replace GNU ld 2.29.1 on the Symbian link? (T5, Rust SDK)

- **Why:** the toolchain still needs a hand-built GNU ld **2.29.1** — 2.35 dies on `euser.dso`'s `.gnu.version_d` — which is the heaviest setup burden left for anyone adopting symdev. rustup already ships `rust-lld`, so if it could link an E32 the binutils dependency would go. The Rust SDK has no other use for binutils except `ar`.
- **Procedure:** run the recorded link line (experiments 5, 77) with `rust-lld` (LLD 23.1.1) instead of `arm-none-symbianelf-ld`, on `examples/hello`'s archive, and keep going until it either links or the next blocker is understood.
- **Outcome:** **no** — keep GNU ld 2.29.1. Four blockers, each deeper than the last; the first two are cheap to fix and the last two are not.
- **Evidence:** 2026-09-20, `/tmp/claude-1000/lldfix/` (outside git).
  1. **`--default-symver` is not an lld option.** Cosmetic; the link gets further without it.
  2. **428 of the 570 SDK import libraries have a malformed `.dynstr`.** lld: `SHT_STRTAB string table section [index 6] is non-null terminated`. Confirmed by hand: `euser.dso` section 6 ends `2e 64 6c 6c 00 20 20 20`, i.e. `.dll\0` and then three padding **spaces**, so the last byte is `0x20`, not the NUL the ELF spec requires. GNU ld tolerates it. The trailing bytes are padding that no string references, so overwriting the last one with `0x00` in a cached copy fixes it without moving a single offset — done for all 570 files and it cleared the error. The overlay pattern already used for SDK headers (`SdkIncludeCaseFold`) would fit.
  3. **`R_ARM_ABS32 cannot be used against local symbol; recompile with -fPIC`.** The E32 model *is* a `-shared` link full of absolute relocations that the post-linker rewrites; Symbian never uses `-fPIC` (design spec §4). `-z notext` gets past it, but it is telling lld to permit exactly what it is designed to reject, in a configuration nothing has ever validated.
  4. **The SDK's own runtime objects want RVCT linker-defined symbols.** `eexe.lib` and `usrt2_2.lib` reference `.ARM.exidx$$Base`/`$$Limit`, `ER_RO$$Base`/`$$Limit` and `INIT_ARRAY$$Base`/`$$Limit` — the ARM toolchain's section symbols, which GNU ld supplies and lld does not (`undefined hidden symbol: .ARM.exidx$$Base`). Six names, so a linker script could define them, but their values are the real section bounds and getting one subtly wrong would miscompile the startup path.
- **Conclusion.** Not a dead end in principle: a DSO overlay, `-z notext` and a linker script supplying six symbols would probably link something. It is a dead end *for now*, because every layer is a place for a defect that would only show on a device we have not tested on, and the prize is one build dependency. Revisit if binutils 2.29.1 becomes hard to build, or after hardware M0 gives a way to tell a good image from a bad one.

## 81. `#[symbian_std::main]`: an attribute entry point, and whether a proc macro works on this build (T5, Rust SDK)

- **Requires:** experiments 65, 65a, 69, 77, 79. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a: an application developer should almost never touch a Symbian-specific API, and an entry macro at the bottom of the file is one.
- **Procedure:** (1) prove a proc-macro crate compiles at all in a workspace whose `.cargo/config.toml` forces `build.target = arm-symbian-e32.json` with `-Zbuild-std=core,alloc`; (2) `symbian-rs/crates/symbian-macros` with the `main` attribute, re-exported as `symbian_std::main`; (3) `IntoExitCode` for `Result`, `SymbianError` and `io::Error`; (4) migrate all five examples and the scaffold; (5) `symdev new`/`build`/`package`/`run` from an empty directory, `symdev test --emulator` on `examples/files`, and the E32 sizes of all five.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20.
  - **A proc macro needs no change to this build.** cargo compiles a `proc-macro = true` crate for the *host* even when `build.target` is the phone's JSON and `-Zbuild-std=core,alloc` is in force — `core`/`alloc` are built for the target, the macro against the host's own prebuilt std, and nothing in the target JSON, the link line or `RustBuild` was touched. The fallback (a better `macro_rules!`) was not needed.
  - **Its logic can be unit-tested, but not through `proc_macro`.** `cargo test -p symbian-macros` runs in that workspace (the test harness is a host binary), yet any use of the `proc_macro` API inside a test panics with `procedural macro API is used outside of a procedural macro`, so a test can never build a `TokenStream`. The macro is therefore split: a pure `&str -> Result<String, String>` reader with 14 `#[test]`s, and a four-line shell that stringifies `item` for it. `trybuild` is not in the offline registry cache; `syn`/`quote`/`proc-macro2` are, and were deliberately not used — the grammar is one function header, and the crate stays dependency-free.
  - **`TokenStream::to_string()` returns the original source, comments included.** Not `#[doc = "…"]`: a doc comment arrives as `///`, and a `/* */` inside a parameter list arrives as itself. Found by a build failure on `examples/files`, whose `main` carries a doc comment; the header reader skips line, doc and nested block comments, and `fn main(/* nothing */)` counts as taking no arguments.
  - **`#![no_main]` is gone from every example and from the scaffold.** The crate is compiled as a `staticlib` (`[lib] path = "src/main.rs"`, `autobins = false`), so rustc never looks for a `main` of its own; the attribute is left as the only thing an entry point needs. `#![no_std]` stays — there is no `std` for this target.
  - **Where it lives:** `#[symbian_std::main]`, re-exported from the implementation-detail crate `symbian-macros`, and not a new umbrella `symbian` crate. An application's first lines are then one crate name for everything it is meant to touch — `use symbian_std::prelude::*;` and `#[symbian_std::main]` — with `symbian-runtime` (the panic handler, the heap, `entry!`) arriving underneath without being named. The scaffold's `Cargo.toml` names `symbian-std` and `symbian-core`, one for the road and one for the descriptors the escape hatch still needs.
  - **One conversion, not two.** `IntoExitCode` stays in `symbian-runtime` and gains `Result<T, E>` (generic in both), `SymbianError` (a new `symbian-core` dependency there — the runtime is the top of the stack, not the bottom) and, in `symbian-std`, `io::Error`. `ExitCode`/`IntoExitCode` are re-exported at the `symbian_std` root so an application can implement it for its own error type, and so a diagnostic names a path a user can read. `entry!` still works and still ends in `ExitCode::from_main`.
  - **`symbian_std::io::Result` gained a defaulted error parameter** (`Result<T, E = Error>`), which is what lets the prelude export the name: `fn main() -> Result<()>` reads as it should and `Result<T, E>` still means what it always did in the rest of the file.
  - **The wrong shapes, as rustc prints them.** Every one points at the user's `#[symbian_std::main]` line, because that is the attribute's call site; the function itself is re-emitted as the token stream it arrived as, so nothing about the body moves.

    | Written | Said |
    |---|---|
    | `#[symbian_std::main] struct Main;` | ``error: `symbian_std::main` can only be applied to a function: expected `fn`, found `struct` `` |
    | `fn main(argc: i32)` | ``error: `fn main` must take no arguments: `E32Main()` is called with none, and a Symbian process reads its command line with `RProcess::CommandLine` instead (found `argc: i32`)`` |
    | `fn run()` | ``error: `#[symbian_std::main]` names the program's entry point, so the function must be called `main` (found `run`)`` |
    | `fn main() -> &'static str` | ``error[E0277]: the trait bound `&str: IntoExitCode` is not satisfied`` at the attribute, with `()`, `Result<T, E>`, `i32`, `SymbianError` and `symbian_std::io::Error` listed as the types that do |
    | `#[symbian_std::main(gui)]` | ``error: `#[symbian_std::main(gui)]` is not implemented yet: an Avkon application's entry point must not create a `CActiveScheduler`, because CONE creates and runs `CCoeScheduler` itself and `CCoeEnv` is a `CActive` on it (step 75)`` |
    | `async fn main()` | ``error: `#[symbian_std::main]` cannot be applied to a `async fn`: an `async fn main` needs an executor, and the active-scheduler bridge is step 73`` |
  - **Room for the second shape, and no code for it.** `gui` is in the grammar (`Shape::Gui`) and refused by the generator, so step 75 changes what it does rather than what it is; the `wrapper` match has the arm where an `EikStart::RunApplication` hand-off will go. Nothing about the GUI entry was implemented or guessed.
  - **Acceptance:** `symdev new hello --language rust && symdev build && symdev package && symdev run` from an empty directory: `build/hello.exe` **3 187** bytes, `build/hello.sisx` 4 528, and `build/eka2l1.log` `[Service.Notifier]: Trying to display: Hello from Rust SDK (19 chars)`; the emulator exited on its own. `examples/files` through `symdev test --emulator`: `filesdemo: 16 passed`, exit 0.
  - **Sizes, against experiments 77 and 79.** `hello` **3 187** (=), `hello-raw` **752** (=), `alloc` **4 320** (=), `files` **10 423** (=), `shim` **4 474** against 4 475 — one byte less. Measured both ways from the same tree: the shim ELF's `.text` went 6 596 → 6 604, eight bytes *more*, because the `Result` is now converted inside the exported `E32Main` instead of in the example's own `fn main() -> i32`; the E32 body is deflate-compressed, so eight bytes of ARM text came out one byte shorter. An entry-point change moving nothing else is what the other four confirm.
  - What this does not show: the GUI entry (step 75), an `async` one (step 73), `trybuild`-style compile-fail tests in CI (the messages above were produced by hand, by building a deliberately broken `examples/hello`), a device.

## 80. Atomics, `Arc`, `Mutex`, `Once` and threads made real (T5, Rust SDK)

- **Requires:** experiments 65, 68, 69, 72, 77, 78, 79. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §4, §6a, §8, §11 step 72. Implements the survey [eka2-concurrency.md](eka2-concurrency.md).
- **Procedure:** (1) raise `max-atomic-width` to 32 and `atomic-cas` to true and read the undefined symbols off the resulting archive with `nm -u`; (2) define exactly that set over one process-wide `RFastLock`, first in C++ and then in Rust, and link it as its own archive; (3) settle the access violation the survey recorded after a worker thread exits, in Rust and then in pure C++; (4) build `symbian_std::sync` (`Mutex`, `MutexGuard`, `Once`, `Arc`) and `symbian_std::thread` (`spawn`, `JoinHandle::join`, `sleep`, `yield_now`); (5) `examples/atomics` through `symdev test --emulator`, with a deliberately racy counter beside the atomic one as the control.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/80-atomics/` (the 11 582-byte E32); source is `symbian-rs/examples/atomics`.

  - **The exact libcall set is 31 symbols, measured and not guessed.** `nm -u` over the static library of a probe that touches every operation `core::sync::atomic` offers at every width the target allows gives `__atomic_{load,store,exchange,fetch_add,fetch_sub,fetch_and,fetch_or,fetch_xor,fetch_nand,compare_exchange}_{1,2,4}` plus `__sync_synchronize`. **No `_8` at any point**, which is the evidence that `max-atomic-width: 32` really does keep 64-bit atomics out of dependency code. `fetch_max`, `fetch_min` and `compare_exchange_weak` emit no libcall of their own: they lower to a `__atomic_compare_exchange_N` loop.
  - **The target JSON now says `max-atomic-width: 32` and `atomic-cas: true`**, in the same commit as the shim, as experiment 72 required. What that makes exist: `AtomicBool`/`AtomicU8`/`U16`/`U32`/`Usize`/`Isize`/`AtomicPtr`, `core::sync::atomic::fence`, and **`alloc::sync`, so `Arc` and `Weak`** — none of which compiled at all before.
  - **The shim is Rust, not C++, and the two `TRAP`s are the only C++ left.** The rule in `shims/common/symrs_shim.h` is now explicit: a file belongs in the C++ shim when it needs `TRAP` (or, from step 75, a C++ subclass), and nothing else. `symrs_atomic.cpp` and `symrs_cstring.cpp` are gone; `crates/symbian-libcalls` has both, `symrs_leave.cpp` and `symrs_f32.cpp` remain.
  - **Three traps in the Rust port, two of which bit.**
    1. *Self-recursion.* `core::sync::atomic::compiler_fence` inside `__sync_synchronize` compiled to `bl __sync_synchronize` — a call to itself — because LLVM lowers even a single-threaded fence to a libcall on this target. `objdump -dr` over the object found it; an empty `asm!("")` with neither `nomem` nor `readonly` is the barrier that emits no instruction. Every other body uses `read_volatile`/`write_volatile`, and the check that they do is `objdump -dr` over the archive: the only call relocations are 30 to `AtomicLock::acquire`, 31 to `RFastLock::Signal`, and one each to `Wait`, `CreateLocal`, `User::LockedInc`, `User::Panic` and `User::After`.
    2. *Signatures.* rustc's own `suspicious_runtime_symbol_definitions` lint prints the expected signature of `memcmp`/`bcmp` for this target (`*const c_void`, not `*const u8`), so the ABI came from the compiler rather than from a reading of the C standard.
    3. *Bootstrapping the lock.* Solved twice, and the second way is better — see below.
  - **A C++ static constructor is the wrong way to create the lock, and the reason is worth keeping.** The first version used `__attribute__((constructor))`. GCC emits that as its **own** `.init_array` slot, ahead of the slot holding the translation unit's C++ static initialiser, so the lock was created first and `RFastLock`'s own inline constructor then ran over it and zeroed `iHandle` again. Observed: `rc=0 h=196610` inside the creating function, `h=0` by the time `E32Main` read it, the same address (0x400008) both times, and a plain `.bss` sentinel written beside it surviving intact; the first atomic then hit the guard and the process died silently. A static object *defined after* the lock fixes it, because C++ orders a translation unit's statics by definition. Confirmed on the way: the linker script does `PROVIDE (SHT$$INIT_ARRAY$$Base/Limit)` around `.init_array`, `--gc-sections` does not drop it, and `usrt2_2.lib`'s `__cpp_initialize__aeabi_` walks exactly that range before `E32Main` (traced with `User::InfoPrint` from inside the constructor).
  - **The Rust version needs no constructor at all: `User::LockedInc` is a once.** `if LockedInc(&claim) == 0 { create the lock; publish } else { spin on the published flag with User::After(0) }` — built from the one primitive the platform really has, which needs no lock and no initialisation of its own. Observed from inside the program: `no atomic lock before the first atomic` (status 0), then after one `store`, status 1 and handle 196610. If the lock cannot be created the shim panics with category `symrs-atomic` rather than returning a value that is quietly not atomic.
  - **Size: the port costs nothing, but only because the crate is its own archive.** A `#[unsafe(no_mangle)]` symbol is a global in a `-shared` link and therefore a `--gc-sections` root, so as an ordinary dependency of `symbian-runtime` the 32 entry points survived into every program: `hello` went **3 187 → 3 943**. `-Zdefault-visibility=hidden` does not help — `no_mangle` items stay `GLOBAL DEFAULT`. Built separately and placed after the application archive, the member is pulled only by a program that really uses it, and `hello` is 3 187 again. One more step was needed: at the workspace's `codegen-units = 1` the whole crate was one object, so `examples/files`, which uses `memcmp` and no atomic, grew **10 423 → 11 137**; `codegen-units = 16` for that profile gives one member per module and `files` is 10 423 again. The profile also needs `lto = false`, because under `lto = true` the rlib holds LLVM bitcode, which `ld` cannot read.

    | Program | before | after | what moved |
    |---|---|---|---|
    | `hello` | 3 187 | **3 187** | — |
    | `hello-raw` | 752 | **752** | — |
    | `examples/shim` | 4 475 | **4 475** | — |
    | `examples/alloc` | 4 320 | **4 474** | +154: the heap's lock |
    | `examples/files` | 10 423 | **10 552** | +129: the heap's lock |
    | `examples/atomics` | — | **11 582** | new |

    **The atomics archive itself costs a program that does not use it nothing**, which
    is the whole point of the separate archive. The two programs that did move are the
    two that allocate, and what they pay for is [`symbian_alloc::serialise`]: one
    out-of-line guard around each `GlobalAlloc` method, so that two threads sharing one
    heap cannot corrupt it. Inlined into all four methods that guard cost `alloc` 183
    bytes instead of 154, which is why both halves are `#[inline(never)]`.

  - **The access violation is settled, and it was never about code.** The survey's `Access violation reading address 0x8000A4` is a **heap** address, not `.text` + 0xA4: the process heap sits at 0x800000 (or 0x700000 — it moves) and `User::Alloc` hands back base + 0xa0. The fault is the main thread's next allocation after a worker thread exits. It is specific to `RThread::Create(name, fn, stack, RAllocator* aHeap, ptr, owner)`, the overload that shares the creator's heap; with the `(heapMin, heapMax)` overload the main thread allocates happily after the join and the program runs to the end. `RAllocator::Open()` on the shared heap before the create does **not** help. Reproduced in **pure Symbian C++ with no Rust in the picture**, one `#define` apart: `repro alloc while it runs = 7340192`, `repro joined, exit type = 0`, `Access violation reading address 0x700000 in thread Main`; with the define flipped, `repro alloc after join`, `repro alloc after close`, `repro still alive`. EKA2L1 hand-writes the thread entry routine (`src/emu/kernel/src/libmanager.cpp`, `thread_entry_routine_`) — given an allocator it calls euser's heap-switch export, given sizes it calls the chunk-heap export and then switches — bypassing the SDK's own thread heap setup, which is where a supplied allocator's reference counting would live. **No device here, so "the emulator is wrong and a phone is right" stays unproven**, but the fault is certainly not ours; the reproducer is 60 lines of euser-only C++ and is worth sending upstream.
  - **The reproducer, in full**, because the scratchpad it was written in does not
    survive and this is worth sending upstream. One `symdev` C++ project, `euser.lib`
    only, `CAPABILITY None`; flip `SHARE_THE_HEAP` to 0 for the control.

    ```cpp
    #include <e32base.h>
    #include <e32std.h>

    #define SHARE_THE_HEAP 1

    LOCAL_C void Say(const TDesC& aText, TInt aValue)
        {
        TBuf<128> b;
        b.Append(aText); b.Append(_L(" ")); b.AppendNum((TInt64)aValue);
        User::InfoPrint(b);
        }

    LOCAL_C void Probe(const TDesC& aWhere)
        {
        TAny* cell = User::Alloc(64);
        Say(aWhere, (TInt)cell);
        if (cell) User::Free(cell);
        }

    LOCAL_C TInt Worker(TAny*) { return 0; }   // does nothing at all

    GLDEF_C TInt E32Main()
        {
        Probe(_L("repro alloc before create ="));
        Say(_L("repro allocator ="), (TInt)&User::Allocator());

        RThread t;
        TInt rc;
    #if SHARE_THE_HEAP
        rc = t.Create(_L("repro-worker"), Worker, KDefaultStackSize,
            &User::Allocator(), NULL);
    #else
        rc = t.Create(_L("repro-worker"), Worker, KDefaultStackSize,
            0x1000, 0x10000, NULL);
    #endif
        Say(_L("repro create rc ="), rc);
        if (rc != KErrNone) return rc;

        TRequestStatus s;
        t.Logon(s);
        t.Resume();
        Probe(_L("repro alloc while it runs ="));
        User::WaitForRequest(s);
        Say(_L("repro joined, exit type ="), t.ExitType());

        Probe(_L("repro alloc after join ="));    // <-- faults with SHARE_THE_HEAP 1
        t.Close();
        Probe(_L("repro alloc after close ="));
        User::InfoPrint(_L("repro still alive"));
        return KErrNone;
        }
    ```

  - **The way out, measured.** Create the worker with the own-heap overload and have it call `User::SwitchAllocator(creator_heap)` as its **first** instruction. Observed: the worker's allocator becomes the creator's heap, 400 interleaved alloc/free pairs on each thread with forced yields all succeed, and after the join the main thread allocates again including a 16 kB block that walks the whole free list. That is what `symbian_std::thread::spawn` does, so a `Box` or an `Arc` really can cross threads.
  - **The allocator takes a lock from the moment a thread exists.** Whether the process heap is internally locked is not observable from outside — `RAllocator::TFlags` has an `ESingleThreaded` bit, the field is protected, and `RHeap` is not declared in this SDK at all — so `spawn` turns on `symbian_alloc::serialise_across_threads` before creating the first thread rather than trusting the test above. It is a plain `TInt` flag and not an `AtomicBool` on purpose: an atomic would pull the libcall archive into every program that allocates and make every allocation take the atomics' global lock as well.
  - **`RSemaphore::Wait(0)` blocks for ever.** 0 means "no timeout", not "do not wait" — a probe printed `about to Wait(0) on an empty semaphore` and never came back. `Wait(1)` on an empty semaphore returns **−33 `KErrTimedOut`** at once, `Wait(1000)` likewise, and with a token both return 0. That is what `Mutex::try_lock` uses.
  - **`Mutex` is a one-token `RSemaphore`, which contradicts the survey's recommendation and has to.** Experiment 72 proposed `RFastLock` for `lock` and `RSemaphore`'s timeout for `try_lock`; those are two different kernel objects, so that is not one mutex. Neither `RFastLock` nor `RMutex` has a timed wait, `RSemaphore` is the only primitive in 9.3 that does, and a one-token semaphore is non-recursive — exactly Rust's deadlock-not-UB contract. `Mutex::new` stays `const` (so a `Mutex` can be a `static`) by creating the handle on first use through `Once`.
  - **The counts, from inside the emulator.** 2 000 iterations on each of two threads: `fetch_add` **4000 of 4000**, a `static Mutex` **4000 of 4000**, an `Arc<Mutex<u32>>` **4000 of 4000**, `Once` **1 run**; and the control, a load-then-store on a third counter in the same loop with the same yields, **2000 of 4000** on one run and **2008 of 4000** on the next — about half lost, and varying between runs, which is what says the threads really interleaved rather than the number being an artefact. `symdev test --emulator` exits 0 with `atomicsdemo: 23 passed`, and `examples/files` still reports its 16.
  - What this does not show: a device; whether the four euser atomics or a plain aligned load are atomic on real hardware; whether `__sync_synchronize` must be more than a compiler barrier there; whether the access violation is the emulator or the SDK contract; `RCondVar`, which still returns `KErrNone` from `CreateLocal()` with `Handle()` 0 and which nothing in this slice needed.
## 85. `symbian_std::time`: which Symbian clock is `Instant`, which is `SystemTime`, and where the Unix epoch really is (T5, Rust SDK)

- **Requires:** experiments 68, 69, 77, 78, 79, 81. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a: the developer writes `Duration`, `Instant`, `SystemTime`, not `TTimeIntervalMicroSeconds32` and `TTime`. This is the **time half of step 76**; TLS is untouched and still open.
- **Procedure:** read `e32std.h`, `e32hal.h`, `hal_data.h` and `e32def.h` for what `User::TickCount`, `User::NTickCount`, `User::FastCounter` and `TTime::HomeTime`/`UniversalTime` measure; take the mangled names from `nm -D euser.dso`; measure every period, resolution and monotonicity inside EKA2L1 with a probe paced by `User::After`; establish the Unix epoch offset from **euser's own calendar code** rather than by arithmetic; build `symbian_std::time` and prove it with `examples/time` through `symdev test --emulator`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-20, `symbian-rs/corpus/85-time/` (the 20 583-byte E32); source is `symbian-rs/examples/time`. Three throwaway probe versions, kept only as the numbers below.
  - **The four clocks, measured.** `UserHal::TickPeriod` = **15 625 µs** (1/64 s), and `User::TickCount` moved exactly **64** across `User::After(1_000_000)`, so the stated period and the measured one agree. `User::NTickCount` moved **1000** across the same second → **1 000 µs**. `User::FastCounter` moved **33 337** → ~33.3 kHz. 4 096 back-to-back reads moved the two tick counters not at all and the fast counter 8 times. `TTime::UniversalTime`'s smallest non-zero step is **1 µs**. 32 768 `NTickCount` samples: **0** decreases, raw or wrapping.
  - **The HAL route is closed, which is what decides the design.** `hal.dll` is not on this SDK's link line, but euser exports `_ZN7UserSvr6HalGetEiPv` — `UserSvr::HalGet(TInt, TAny*)`, what `HAL::Get` calls. Tried with the `HALData::TAttribute` ordinals counted out of `hal_data.h` (`ESystemTickPeriod` 14, `EMemoryRAM` 15, `ENanoTickPeriod` 92, `EFastCounterFrequency` 93, `EFastCounterCountsUp` 94): **every one returns `KErrNotSupported` (-5)** inside EKA2L1, attribute 14 included, although `UserHal::TickPeriod` hands that same number over. So the nanokernel tick period is unreadable here.
  - **`Instant` is therefore `User::TickCount` + `UserHal::TickPeriod`**, the only counter whose period the platform will state through a call that works, so the tick→`Duration` factor is read at run time on whatever board this lands on. The price is resolution, 15.625 ms against `NTickCount`'s 1 ms; the gain is the wrap window, 2^32 × 15.625 ms = **776.7 days** against 49.7 days. `NanoTicks` and `FastCounter` stay reachable as `symbian_core::time` for a program that knows its own board. **`FastCounter` may count *down*** — `HALData::EFastCounterCountsUp` exists for that reason — and nothing in this SDK builds a duration from it.
  - **The Unix epoch was 12 days off when computed, and is now measured.** Proleptic Gregorian arithmetic gives 719 528 days from 0000-01-01 to 1970-01-01. euser's own `Time::LeapYearsUpTo(1970)` answers **490**, not 478, and `IsLeapYear` says 100, 400 and 1600 are leap years while 1582 and 1900 are not: **Symbian's calendar is Julian before 1600 and Gregorian from 1600**, which is what `e32std.h`'s "nominal Gregorian" means. 1970·365 + 490 = **719 540 days = 62 168 256 000 000 000 µs**. Checked against a known date through euser's own accessors, which run the ROM DLL and not emulator code: that value decodes as day 1 of the year, day 0 of the month, weekday 3 = `EThursday`, 31 days in the month — 1 January 1970, a Thursday. The 719 528 candidate decodes as 20 December. Cross-check: `UniversalTime` decoded as day 263, day-of-month 19, weekday 6, 30 days in the month = 2026-09-20, a Sunday, which it was.
  - **`SystemTime` is `TTime::UniversalTime`, not `HomeTime`**, because `UNIX_EPOCH` is a UTC instant. Measured `HomeTime - UniversalTime` = **7 200 000 0xx µs**, the emulated time zone, so the two are genuinely different readings and picking the wrong one would be two hours of silent error.
  - **No shim, and no new C++ at all.** Against the three rules in `shims/common/symrs_shim.h`: nothing here is declared as leaving or carries an `L`, every signature is scalar in and scalar or `void` out, and `TTime::HomeTime`/`UniversalTime` are non-virtual, non-static members of a class whose only storage is one `TInt64` (`__DECLARE_TEST`, `e32def.h` line 2024, adds only member functions), so experiment 78's `this`-as-argument-0 convention covers them.
  - **What `std`'s shape could not be given, and why.** (1) **`Instant::now()` returns `io::Result<Instant>`** — the tick period is a `TInt`-returning platform call and this crate may not panic, so the failure is in the signature rather than behind an assumption; `elapsed`, `duration_since`, `checked_*`, `Add` and `Sub` all keep `std`'s exact shapes. (2) **`Instant` is not `Ord`** — a 32-bit wrapping counter has no total order, and RFC 1982 serial comparison is not transitive, so `Ord` would be a lie `BTreeMap` would find; `checked_duration_since` returning `None` is `std`'s own honest comparison. (3) Where `std` panics on overflow, `Instant` wraps and `SystemTime` saturates, with the `checked_*` pair as the reporting form.
  - **`examples/time` through `symdev test --emulator`: 29 passed, exit 0.** A 1 000 ms sleep measures **1 000 ms** and a 500 ms one **500 ms** (64 and 32 ticks exactly); a 100 ms sleep measures 93.75 or 109.375 ms, because `User::After` rounds to a tick boundary — all inside the stated ±32.25 ms, two ticks and a millisecond.
  - **Monotonic, and not vacuously.** 20 000 instants over 218–234 ms across runs: **0 backwards**, and the counter moved **14–15** times. The first version of the test took its 20 000 samples inside a single 15.625 ms tick and proved nothing; "the counter really moved" is now a case of its own, and a run where it does not move fails.
  - **A device clock change, which is step 76's own criterion.** `User::SetUTCTime` works inside EKA2L1: the wall clock jumped the full hour, `SystemTime` jumped with it, and the `Instant` taken before it measured **0** across the change. The clock is put back afterwards, and two back-to-back runs showed no drift from doing so.
  - **The emulator's clock is the host's.** Its last `SystemTime` reading landed **50, 51 and 78 ms** before the host's own clock at the moment `symdev test` returned, on three runs — and those tens of milliseconds contain the report write and the emulator's shutdown, so the true skew is smaller than any of them, does not drift between runs and does not accumulate from the `SetUTCTime` above.
  - **Sizes, after the rebase onto experiment 80's tree.** `hello` **3 187**, `hello-raw` **752**, `alloc` **4 474**, `shim` **4 474**, `files` **10 552**, `atomics` **11 582**, and the new `time` **20 583**. Measured against a baseline built from main's own `symbian-rs` in the same tree: all six older examples come out at exactly those bytes with this work present and with it absent, so `time` is the whole cost of this change.
  - **`time` carries 6 124 bytes of soft float it never calls**, and it is experiment 77's one-codegen-unit problem again: `Duration` arithmetic needs u64 division, the 636-byte `u64_div_rem` pulls the single `compiler_builtins` object, and `__adddf3`/`__divdf3`/`__muldf3` and their `sf` siblings come with it — `objdump` finds no caller for any of them outside each other. **22 `compiler_builtins` symbols, 7 556 bytes, 6 124 of them float**; `files` and `atomics` have 0. Measured separately, before the rebase: using `Duration::as_micros`/`as_millis`, whose `u128` drags 128-bit division and formatting in as well, cost a further **1 081 bytes** (21 560 → 20 479).
  - What this does not show: a device, for any of it — every period, the epoch conversion, `SetUTCTime` and `HalGet`'s refusal are EKA2L1 facts. On a phone `UserSvr::HalGet` may well work, which would make `NTickCount` usable at its real period; `User::SetUTCTime` needs `WriteDeviceData`, which a self-signed SIS cannot grant. TLS, the other half of step 76, is untouched.

## 84. Networking as `symbian_std::net`, blocking, with no executor (T5, Rust SDK)

- **Requires:** experiments 68, 69, 77, 78, 79, 81. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a: the developer-facing API is `std`-shaped, and what Symbian does that `std` has no word for is said plainly instead of hidden.
- **Procedure:** read `es_sock.h` and `in_sock.h` for what the socket API is and whether any of it leaves; take every mangled name from `nm -D` on `esock.dso`, `insock.dso` and `euser.dso`; measure the layouts with a compile probe on the recorded GCCE argv; build `symbian-sys::esock`, `symbian_core::net` and `symbian_std::net`; make a missing capability a build error; run `examples/net` against a Python peer on this host's loopback through `symdev test --emulator`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/84-net/` (the 13 183-byte E32); source is `symbian-rs/examples/net`. `netdemo: 22 passed`, `symdev test --emulator` exit 0.

  - **Step 73 is not a prerequisite, and that is the headline.** Every interesting `RSocket` operation takes a `TRequestStatus&` and completes later, which looks like it demands a `CActive` and a `CActiveScheduler`. It does not: the blocking form Symbian itself documents is `RSocket::Connect(addr, status); User::WaitForRequest(status);`, which blocks the calling thread on its own request semaphore. That is exactly `std::net`'s contract, so the whole module is 41 lines of `symbian_core::net::request::blocking` and no executor exists anywhere below it. Step 73 will add the non-blocking path *beside* these declarations rather than underneath them. The spec's "73 before 74" dependency was wrong and §11 now says so.
  - **Nothing in the socket API leaves, so there is no shim.** `es_sock.h` declares no leaving member on `RSocketServ`, `RSocket` or `RHostResolver` — every `IMPORT_C … L(` in that header is on the `CSubConParameterSet`/`CSubConParameterFamily`/`CSubConParameterBundle`/`CSubConNotificationEvent` family (lines 1204-1394), none of which this SDK calls — and `in_sock.h` declares **none at all**. Checked against the declarations, not against the trailing `L`, the way experiment 79 checked `f32file.h`. `shims/common/symrs_esock.cpp` was never created. The one non-C signature on these classes is `RSocketServ::Version()`, which returns `TVersion` by value (sret, rule 2); nothing needs it, so it is not declared.
  - **`RHostResolver::GetByName` has a synchronous overload**, `TInt GetByName(const TDesC&, TNameEntry&)` (`_ZN13RHostResolver9GetByNameERK7TDesC16R8TPckgBufI11TNameRecordE`), so resolving a name needs no request status at all.
  - **Layouts measured, not assumed** (compile probe on the recorded GCCE argv; note that the compiler emits `movs rN,#k; lsls rN,#2` above 255, so the immediate is a quarter of the answer — the first reading of this probe was wrong by a factor of four until the `lsls` was noticed): `sizeof` `TSockAddr` **40** (align 4), `TInetAddr` **40** (align 4 — it adds no data members), `RSocketServ` **4**, `RSocket` **8**, `RHostResolver` **8**, `TRequestStatus` **8** (align 4, `TInt iStatus; TUint iFlags;`, `e32cmn.h` line 2056), `TNameRecord` **564**, `TNameEntry` (`TPckgBuf<TNameRecord>`) **576** (align 8), `TSockXfrLength` (`TPckgBuf<TInt>`) **16** (align 8); `TNameRecord` offsets `iName` 0, `iAddr` 520, `iFlags` 560, and a `TPckgBuf<T>`'s payload at **8**.
  - **A packaged buffer is built by euser, not by Rust.** `TPckgBuf<T>()` is `TAlignedBuf8<sizeof(T)>(sizeof(T))` (`e32cmn.inl` lines 2659 and 1179), which is `TBufBase8(aLength, S)` — exported as `_ZN9TBufBase8C1Eii`. So `TNameEntry` and `TSockXfrLength` are zeroed storage of the measured size plus that one call, and the 8-bit type nibble is still never guessed (experiment 79's rule, unchanged).
  - **A `TSockAddr` a call fills in must be *constructed*, and finding that out cost three failing cases.** `TSockAddr` is a `TBuf8<KMaxSockAddrSize>`, so `RSocket::LocalName`, `RemoteName` and `RecvFrom` write into it **as a descriptor** and honour its `iMaxLength`. Handed zeroed storage, whose `iMaxLength` is 0, the socket server writes nothing and there is no diagnostic anywhere — `peer_addr`, `local_addr` and the UDP port simply read back as `KAFUnspec`. Fixed by calling `TInetAddr::TInetAddr()` (`_ZN9TInetAddrC1Ev`) into the storage first, which is the same pattern as `TPtrC8`/`TPtr8`.
  - **End of input on a socket is `KErrEof` (-25), not a zero-byte completion.** Observed: `read_to_end` against a peer that had closed came back `UnexpectedEof (KErrEof (-25))`. `symbian_core::net::Socket::recv` now reports it as the bytes that arrived — `0` when none did — which is the same statement `RFile::Read` makes for end of file, so both ends of the crate say end of input the same way. **`RecvOneOrMore` and not `Recv`**, too: `Recv` does not complete until the descriptor is full, which is not what `std::io::Read` promises; `RecvOneOrMore` has no overload without a `TSockXfrLength`, which is the only reason that type exists here.
  - **The emulated phone's loopback is this host's loopback, in both directions.** EKA2L1's internet protocols hand the guest's address straight to libuv, so there is no port mapping and no special address: the app connected to `127.0.0.1:18974` and the Python server on the host logged the connection, and the host's `poker.py` connected *into* the app's `TcpListener` on `127.0.0.1:18975` and got `served\n` back. Both TCP and UDP protocols are registered unconditionally at start-up (`internet/protocols/overall.cpp:44`).
  - **EKA2L1 does not enforce `NetworkServices`.** The same example, rebuilt with `capabilities = []`, still resolved a name and completed a round trip. So the emulator cannot tell a developer they forgot the manifest line, and the check moved to build time: `symdev build` reads the linked ELF's `DT_NEEDED` list — which names `esock`/`insock` only when the program really reached for a socket, because the SDK puts them on every Rust link line under `--as-needed` — and refuses to write the E32, naming the capability, the manifest key and the SDK header that says so (`in_sock.h` lines 66 and 74, `@capability NetworkServices Required for opening 'tcp' sockets. @ref RSocket::Open()`). `crates/symdev-build/src/required_capability.rs`, four unit tests, Rust builds only.
  - **What has no `std` word is named, not faked** (§6a), in the crate documentation and in [net-access-points.md](net-access-points.md): choosing an access point (`RSocket::Open`'s `RConnection` overload, which this module deliberately does not use), the capability, and IPv6 — refused with `KErrNotSupported` rather than guessed at, while a v4-mapped `KAfInet6` answer from the resolver is converted back with `TInetAddr::ConvertToV4`.
  - **Sizes.** `examples/net` **13 183**. Everything else unchanged: `hello` **3 187**, `hello-raw` **752**, `alloc` **4 320**, `shim` **4 474**, `files` **10 423**. Adding `esock.dso` and `insock.dso` to `RustSdk::LIBRARIES` costs nothing, because ld drops a `DT_NEEDED` no symbol references under `--as-needed` (experiment 78's mechanism, re-confirmed).
  - **A papercut fixed on the way.** `symdev test` installs `build/<name>.sisx` and neither builds nor packages, so a `build` without a `package` silently runs the *previous* image — which here was one with no test report in it at all, and the only symptom was "the application never wrote one" after 180 s. `symdev test` now refuses a SIS older than the E32 beside it and says to run `symdev package`.
  - What this does not show: a device; an access point; the capability actually being enforced; `try_clone`, `set_nodelay`, timeouts, non-blocking mode, `peek` or `UdpSocket::connect`, none of which were run and none of which are shipped; a second answer from the resolver (`RHostResolver::Next` is wrapped but never produced one here); which `e32err.h` code a real stack returns for a refused connection — the example asserts that a closed port *fails*, not which code comes back.
## 88. `thread_local!` over Symbian's TLS: how many slots an EXE really has, and what one access costs (T5, Rust SDK)

- **Requires:** experiments 68, 69, 78, 79, 80, 81, 85. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §4, §6a, §11 steps 76 and 77. This is the **TLS half of step 76**, the half experiment 85 left open, and step 77 is gated on it.
- **Procedure:** read `e32std.h` and `e32svr.h` for what the platform's thread-local storage is and take the mangled names from `nm -D euser.dso`; **probe** in EKA2L1 rather than reason — how many slots, keyed by what, per thread or per process, and whether the uid overloads interoperate; design `LocalKey` on the answers; build `symbian_std::thread_local!`, run destructors at thread exit, measure one access against the bare kernel call and against one atomic; prove it with `examples/tls` through `symdev test --emulator`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/88-tls/` (the 16 272-byte E32); source is `symbian-rs/examples/tls`. `tlsdemo: 45 passed`, `symdev test --emulator` exit 0.

  - **There is no `Dll` class in this SDK, and the design spec's premise was wrong.** Every account of Symbian TLS names `Dll::Tls`/`Dll::SetTls`/`Dll::FreeTls` in `e32std.h`. On S60 3rd FP2 `grep -rn 'class Dll\b\|Dll::Tls\|Dll::SetTls\|Dll::FreeTls' epoc32/include/` matches **one line, and it is a comment** (`banamedplugins.h:249`); `e32std.h` does not contain the string `Tls` anywhere; and `nm -D` over every `.dso` in `epoc32/release/armv5/lib` and every `.lib` in `urel` finds no `_ZN3Dll…` export at all. `Dll::Tls` is the inline wrapper a later Symbian put over the real thing, and here the wrapper is simply absent.
  - **The real surface is five `@internalAll` statics of `class UserSvr`**, `e32svr.h` lines 39–43, all exported by `euser.dso`: `_ZN7UserSvr9DllSetTlsEiPv`, `_ZN7UserSvr9DllSetTlsEiiPv`, `_ZN7UserSvr6DllTlsEi`, `_ZN7UserSvr6DllTlsEii`, `_ZN7UserSvr10DllFreeTlsEi`. They are statics taking scalars, so experiment 78's member ABI does not arise and **no C++ shim is needed** — checked against the three rules in `shims/common/symrs_shim.h`: nothing here leaves, nothing carries an `L`, every signature is scalar in and scalar out.
  - **The critical question, answered by probe and not by reasoning: an EXE gets *many* slots, and they are *per thread*.** The fear in the brief — that a program with no DLL of its own has exactly one slot — is not what this platform does.

    | Probed | Observed |
    |---|---|
    | An untouched handle | reads null |
    | `DllSetTls(h, p)` then `DllTls(h)` | returns 0, reads `p` back |
    | Two handles | two independent slots; a repeat set replaces |
    | `DllFreeTls(h)` | empties `h` only |
    | **How many at once** | **64 of 64 distinct handles, no set failure** |
    | **A worker thread** | reads **null** where the creator stored a value; its own store is visible to itself; **the creator's slot is untouched after the join** |
    | The uid overloads | do **not** interoperate with the one-argument ones in **either** direction — both cross reads came back null |

    `aHandle` is the argument `Dll::Tls` would have filled in with the calling DLL's code segment handle. An EXE has none, so the caller chooses one, and nothing validates it. The one-argument pair round-trips and is what the SDK uses.
  - **Emulator-side mechanism**, read for understanding only (EKA2L1 is GPL-3.0 and is never copied in): `svc_register_funcs_v93` maps exec `0x4D` `dll_tls(h, uid)`, `0x75` `dll_set_tls(h, uid, ptr)` and `0x76` `dll_free_tls(h)`, and the store is `kernel::thread_local_data::tls_slots`, a **per-thread** hash map from handle to slot with a ceiling of 10 000. Whether a device's `DThread` array behaves the same is **UNKNOWN**: no device.
  - **The design: one slot for the whole program, not one per key** — although the platform would allow one per key. Two reasons a slot each cannot give. (1) **Destructors.** `Drop` at thread exit needs the list of live values and the kernel will not enumerate a thread's slots: `DllTls` only answers a handle you already know. One slot holding our own list is the only place that list can be. (2) **The handle is unobserved on hardware**, so taking exactly one means one assumption to retest on a phone rather than one per `thread_local!`.
  - **A key needs no registration and therefore no atomic.** The table is keyed by each `LocalKey` static's **own address**, which is unique, stable and free. Assigning indices instead would need a process-global counter, and on this CPU every atomic operation is a kernel `Wait`/`Signal` pair (experiment 80) — the cost would have landed on the thing being made cheap.
  - **Measured, inside the emulator, 200 000 iterations each.**

    | Operation | ns |
    |---|---|
    | one `thread_local!` access (`with`, already initialised) | **100** |
    | the bare `UserSvr::DllTls` call under it | 53 |
    | `AtomicU32::fetch_add` beside it | 158 |

    So **a thread-local is the cheap way to hold per-thread state on this device, not the expensive one**: one kernel call beats the two an atomic takes. The clock is `SystemTime` (`TTime::UniversalTime`, 1 µs) and not `Instant`, whose 15.625 ms tick (experiment 85) cannot resolve a per-operation figure.
  - **Destructors run, and where they do not is stated rather than leaked.** `table::destroy()` is the last thing every thread `symbian_std::thread::spawn` creates does, after the closure's result is stored and before the thread exits: observed, the worker's `Drop` ran when it ended and the creator's value survived. The **main** thread has no hook below `symbian-std` — `symbian_runtime::entry!` cannot call into `symbian-std` without a dependency cycle, and an unmangled hook symbol would be a `--gc-sections` root that pulls this machinery into every program that links the crate, which is exactly the 756-byte mistake experiment 80 measured. So the main thread calls `thread::drop_thread_locals()` itself, and **step 77's `lang_start` is where that call belongs** once `std` owns the entry point.
  - **After a sweep the slot holds a sentinel, not nothing.** Freeing the slot outright would let a later access quietly start a fresh set of thread-locals that nothing would ever drop; the sentinel makes it `KErrDied` instead, which is `std`'s contract after a thread's destructors have run. Observed, along with a second sweep being harmless.
  - **Where the value lives, given per-thread heaps: there is only one heap.** `spawn` creates a worker with its own heap and switches it onto the creator's allocator as its first instruction (experiment 80), so a `Box` holding a thread-local is allocated and freed on the same heap whichever thread does which. Observed: a worker's `String` thread-local was built on the worker and the process allocated happily afterwards.
  - **`with` ends the process, `try_with` returns.** `try_with` has `std`'s exact signature and is the road (CLAUDE.md: a library returns errors). `with` has `std`'s exact signature too and, when it cannot reach the value, ends the process with `User::Panic`, category `symrs-tls` — which is what `std`'s own `with` does under `panic = "abort"`, and the same rule the atomics archive already follows rather than returning a value that is quietly wrong. `AccessError::reason()` is an `e32err.h` code: −4 the kernel refused a slot, −14 an initialiser asked for its own key, −13 the thread has been swept. All three are reachable and the second and third are tested.
  - **`const { … }` initialisers are accepted**, so `std`-shaped source compiles unchanged; they buy nothing here beyond saying the initialiser is a constant, because there is no thread-local *storage* for the compiler to place a value in.
  - **`has-thread-local` stays `false`, now with evidence rather than a note.** The field governs `#[thread_local]` statics, which need an ELF `PT_TLS` segment. `symdev-elf2e32`'s reader walks the program headers and **keeps only `PT_LOAD`** (`src/elf/image.rs:63`, `if elf.u32_at(ph)? != Self::PT_LOAD { continue }`), mapping the executable one to E32's code section and the writable one to data + BSS; there is no third destination and `grep -rniE 'pt_tls|shf_tls|\.tdata|\.tbss|thread.?local'` over `symdev-elf2e32` and `symdev-build` matches nothing. A `PT_TLS` segment would therefore be **silently dropped**, so turning the field on would buy a feature whose storage never reaches the image. Nothing in this slice needs it: `thread_local!` here is a kernel call, not a relocation.
  - **Sizes, against a baseline built from main's own `symbian-rs` in the same tree.** `hello` **3 187**, `hello-raw` **752**, `alloc` **4 474**, `shim` **4 474**, `files` **10 552**, `time` **20 583**, `net` **13 322** — every one unchanged. The one that moved is **`atomics` 11 582 → 11 719, +137**, and it is the only other program that spawns a thread: what it pays for is the thread-exit sweep. `tls` itself is **16 272**. (The `alloc` 11 499 / `net` 13 183 figures quoted in earlier records are from a different tree state; the baseline above was measured with the same `symdev` binary in the same session.)
  - What this does not show: a device, for any of it — the slot count, the per-thread scope, the handle being unvalidated, the timings and the sweep are all EKA2L1 facts. On a phone `UserSvr::DllSetTls` may well validate `aHandle` against a real code segment, and that is the one assumption this design deliberately takes only once. Also untested: more than three keys on one thread beyond the 64-handle platform probe, a destructor that creates a new thread-local while the sweep runs (it is refused, not supported), and any thread this SDK did not create.

## 87. `async`/`await` on the active scheduler: a `CActive` that wakes a Rust waker (T5, Rust SDK)

- **Requires:** experiments 68, 69, 76, 77, 78, 79, 80, 81, 85. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a, §8, §11 step 73, and by [avkon-rust-spec.md](avkon-rust-spec.md) §1.3, which forbids a GUI application its own scheduler.
- **Procedure:** read `e32base.h` for what `CActive`/`CActiveScheduler` expose and `e32std.h` for `RTimer`; take the mangled names from `nm -D euser.dso`; measure the `RTimer::After` argument ABI with a compile probe on the recorded GCCE argv; write the one C++ subclass; build `symbian-async` (executor, `Request`/`Source`, `sleep`, `join`, `race`) and make the blocking model refuse to share a thread with it; prove it with `examples/async` through `symdev test --emulator` and re-run every earlier example against a baseline built from `main`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/87-async/` (the 21 659-byte E32); source is `symbian-rs/examples/async`. `asyncdemo: 15 passed`, `symdev test --emulator` exit 0 on two consecutive runs.

  - **The criterion, measured.** Written by the application itself to `E:\symdev\async73\measured.txt` with `symbian_std::time::Instant`: one 300 ms sleep **312 ms** (328 on another run); two 300 ms sleeps **awaited together 312 ms**; the same two in sequence **625 ms**; a race between 100 ms and 20 s **109 ms**; twenty scheduler install/run/uninstall rounds 312 ms. 312 against 625 is the whole claim — both requests were outstanding at once. 300 reads as 312 because the system tick is 15 625 µs and `RTimer::After` rounds to a tick boundary (experiment 85).
  - **One C++ file, and it is rule 3 of `symrs_shim.h` rather than rule 1.** `CActive::RunL` and `DoCancel` are pure virtual and its constructor and `SetActive()` are protected, so a subclass is the only way to have one and a Rust type cannot be a C++ subclass. `shims/common/symrs_active.cpp` forwards both virtuals to a Rust-owned table of two function pointers with one opaque context — the shape experiment 76 settled for the Avkon virtuals — and carries **one** `TRAP`, around `CActiveScheduler::Start()`, because `e32base.h` says nothing either way about it leaving. `RunL` calls Rust, then `User::LeaveIfError` on what Rust returned, after the Rust frame has gone.
  - **Everything else is Rust.** `CActiveScheduler::Current()` is a plain non-leaving static returning a pointer, and `RTimer::CreateLocal`/`After`/`Cancel` are non-virtual members with no `L` and scalar signatures, so `symbian-sys` declares them and Rust calls them directly.
  - **A reading gotcha worth one line, because it cost four searches.** This SDK's headers are extended-ASCII, so GNU grep treats them as binary and prints **nothing at all** for a match unless it is given `-a`. `grep -n "class RTimer" e32std.h` is silent; `grep -an` finds it at line 3159. Anyone reading these headers from a script needs `-a`.
  - **The `RTimer::After` ABI, observed rather than assumed.** `t->After(*s, TTimeIntervalMicroSeconds32(us))` compiled with the recorded GCCE argv is `push {r4,lr}; bl _ZN6RTimer5After…; pop {r4,pc}` — **no register shuffle at all**, so `this`, the status and the interval stay in r0, r1 and r2. `TTimeIntervalMicroSeconds32` is `TTimeIntervalBase`'s single `TInt` with only inline constructors, so it passes as the integer it is. Sizes from the same probe: `RTimer` **4**, `TTimeIntervalMicroSeconds32` **4**, `CActive` **28**, `TRequestStatus` **8**.
  - **A `TRequestStatus` becomes a `Waker` without an `unsafe impl Send`.** `core::task::Waker` is `Send + Sync` by declaration, so the value behind it must really be: the one this crate builds holds **nothing but an `AtomicU32`**, and waking is one atomic store. Everything that is not thread-safe — polling, the task list, stopping the scheduler — runs in the `RunL` the shim calls, which is on the scheduler's own thread by construction. The completion callback stores the code, marks the waker and drives the executor; the executor scans a short list of flags rather than sharing a ready queue, because an atomic here is a kernel call (~90× an increment, experiment 72).
  - **Two shapes, one executor, and the caller picks by which function they call.** `block_on` installs a `CActiveScheduler`, polls the future **on its own stack** — so it needs no `'static` and allocates nothing for it — and hands the thread to `Start()` between polls; a completion stops it. `spawn` adds a task to whatever scheduler is already installed and starts and stops nothing, which is the only thing an Avkon application may do. The executor is one process-wide value **bound to the scheduler it first saw**: a `CActiveScheduler` is per thread, so a second thread's is a different pointer and gets `KErrInUse` instead of corrupting the task list.
  - **Deleting a `CActive` from inside its own `RunL` is the trap, and it is closed by one pointer.** A future that finishes inside a completion drops its request, and `~CActive` on the object whose `RunL` is on the stack below is a use-after-free. The executor remembers which object is running; a drop of exactly that one is deferred and the deletion happens at the top of the next completion, after `Start()` returns, or at shutdown. At most one object is ever in that state, because `RunL` is never re-entered.
  - **What stops the two models being mixed, and it is the platform's own rule.** `symbian_core::net::blocking` now returns `KErrInUse` when `CActiveScheduler::Current()` is non-null. `User::WaitForRequest` waits on the *thread's* request semaphore, which every active object's completion also signals, so one wait can consume the other's — in both directions, and neither failure says anything. The check is one euser call and no state of this SDK's own; a console program installs no scheduler, so step 74 is untouched (`netdemo: 22 passed`, re-run). It cost `examples/net` 272 bytes inlined, because `blocking` is generic over its closure and monomorphised per call site; `#[inline(never)]` on the check brought that to **96 bytes of `.text`, 57 of E32**.
  - **The refusals are cases in the example, not claims:** `spawn` with no scheduler `KErrNotReady (-18)`; a nested `block_on` `KErrInUse (-14)` rather than the panic a second `Install` would raise; `blocking` under a scheduler `KErrInUse (-14)`; a 3 600-second sleep `KErrOverflow (-9)`, because `RTimer::After`'s interval is a `TInt` of microseconds and 35 minutes 47 seconds is its limit.
  - **Sizes, against a baseline built from `main` (3f30616, after experiment 88) with the same `symdev` binary in the same session.** Unchanged: `hello` **3 187**, `hello-raw` **752**, `alloc` **4 474**, `shim` **4 474**, `files` **10 552**, `time` **20 583**. `net` 13 322 → **13 379**, which is the guard: +96 bytes of `.text`. Two moved without gaining anything — `atomics` 11 719 → **11 726** and `tls` 16 272 → **16 271**, both with *identical* `.text` (19 036 and 27 088) and `.bss`; the only differences in the ELF are branch offsets, because `symbian-std` gained a dependency and its crate disambiguator changed, so LTO laid the same code out differently and the E32's deflate stream came out a few bytes apart. That is the effect experiment 78 recorded on `alloc` and did not diagnose. The new `async` is **21 659**. No new `DT_NEEDED`: `CActive`, `CActiveScheduler` and `RTimer` are all euser, and `operator delete` already came from `scppnwdl.dso`.
  - **Regression, all rebuilt and re-run in the emulator:** `files` 16 passed, `atomics` 23, `time` 29, `net` 22 (with the new refusal in place — a console program installs no scheduler, so it never fires there), `async` 15 on two runs.
  - What this does not show: a device, for any of it; **the Avkon shape actually running on CONE's scheduler** — `spawn` was exercised on the scheduler `block_on` installed, which is the same code path but not the same owner, and step 75 is what proves the real one; an asynchronous *socket*, which needs only a `Source` implementation and no new C++, but was not written; a waker used from another thread, which is sound but does not interrupt the scheduler's wait (`RThread::RequestComplete` is **TODO: not observed**); and the one hole the guard does not close, `symbian_std::thread::JoinHandle::join`, which is itself a `User::WaitForRequest` and is not refused under a scheduler.

## 86. An Avkon application whose logic is Rust: the shim's subclasses, the vtable and a key that changes the drawing (T5, Rust SDK)

- **Requires:** experiments 65, 68, 69, 76 (the ABI probe), 77 (the size fix), 78 (the shim and the member ABI), 81 (`#[symbian_std::main]`), 83 (keys into the emulator). Design: [avkon-rust-spec.md](avkon-rust-spec.md), [2026-09-20-rust-sdk-design.md](../superpowers/specs/2026-09-20-rust-sdk-design.md) §11 step 75.
- **Procedure:** (1) `symbian-rs/shims/s60/symrs_avkon.{h,cpp}` — the four Avkon subclasses forwarding each virtual to a Rust vtable, per the spec's shape B; (2) `symbian-rs/crates/symbian-ui` — the `App` trait, `Gc`, `Rect`/`Point`/`Rgb`, `KeyEvent` and the key tables, re-exported as `symbian_std::ui`; (3) `#[symbian_std::main(gui)]` in `symbian-macros`; (4) a `[ui]` manifest section and a resource stage in `RustBuild` that generates `<app>.rss` and `<app>_reg.rss` text and feeds the byte-verified `cpp`+`rcomp` path; (5) `symbian-rs/examples/ui` through `symdev build`/`package`/`run`/`test --emulator`, with PID-bound screenshots either side of `docs/research/acceptance/emukey.py`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/86-ui/` (the 12 715-byte E32 and both screenshots); source is `symbian-rs/examples/ui`.

  - **A key changes what a Rust `draw` painted.** `uidemo-before.png`: three bars, a baseline and `bars=3 keys=0`, every pixel of the client area painted by the Rust callback. `emukey.py keys <pid> Up Up`, then `uidemo-after.png`: five bars and `bars=5 keys=2` — **3 767 of the client area's 58 080 pixels changed**, bounding box (63,110)–(152,227). `Down` four times gives `bars=1 keys=6`, a fifth `Down` is not counted (the application returns `EKeyWasNotConsumed` at the clamp), and `Return` resets to `bars=3 keys=7`. `grep -c 'Panic\|KERN-EXEC'` on the log is 0. This is the half experiment 76 could not test at all and the half its `uiprobe` never saw fire; the control stack here is the same `iView->SetMopParent(this); iView->ConstructL(ClientRect()); AddToStackL(iView)` that the C++ `examples/gui` uses.
  - **Seven virtuals forwarded, and no more.** `CShimAppUi::ConstructL`/`HandleCommandL`/`~CShimAppUi`, `CShimView::Draw`/`SizeChanged`/`OfferKeyEventL`, plus `create`/`destroy` around the object's life. Left to the framework, deliberately: `AppDllUid` (a constant symdev generates onto the compile line as `SYMRS_UID3`), `CreateDocumentL` and `CreateAppUiL` (`new (ELeave)` with no Rust on the stack), `CountComponentControls` (a leaf view returns 0), and every `Reserved_*`, `HandleResourceChangeL`, `HandleForegroundEventL`, `MopSupplyObject` and pointer-event slot. Subclassing cost is fixed and forwarding is nearly free, as the spec predicted: `symrs_avkon.o` has **186 undefined symbols** against 182 for the C++ `examples/gui` and 185 for experiment 76's probe, and is 31 516 bytes of `.o`.
  - **The 107 KB did not come back.** `uidemo.exe` is **12 715 bytes**, of which 5 156 are the result-file harness (`symbian_std::fs` plus `core::fmt`); the drawing-and-keys application alone is **7 559**. Experiment 76's mixed C++/Rust probe was 107 028. The cause is the one experiment 77 found and this step did not touch: `-l:euser.dso -l:drtaeabi.dso` before the Rust archive resolves `mem*` from ROM, so the single `compiler_builtins` object is never pulled even with a 31 KB C++ object on the line. `hello` 3 187, `hello-raw` 752, `alloc` 4 474, `shim` 4 474, `files` 10 552, `time` 20 583 and `async` 21 659 are unchanged **to the byte**; `atomics` 11 719, `tls` 16 272 and `net` 13 380 differ from `main`'s 11 726, 16 271 and 13 379 by −7, +1 and +1. Those three were A/B'd in one tree with and without the `pub use symbian_ui as ui;` line and their **`.text` is byte-identical either way** (15 716, 22 016, 17 696): it is experiment 87's own layout wobble — `symbian-std` gaining a dependency changes its crate disambiguator, LTO lays the same code out differently, and the deflated E32 body comes out a few bytes apart. So `symbian_std::ui` costs a console application nothing.
  - **The unexplained black band of experiment 76 is isolated and fixed.** Two probes in the emulator. A red stripe drawn at `Rect::new(0, 0, area.width, 6)` landed at the *top of the band*, so the band is inside the control's own area and not another window over it. A filled `DrawRect(area)` in place of `Clear()` covered it completely. So **`CGraphicsContext::Clear()` with no argument leaves the top ~40 px of a window-owning control unpainted on this platform, while `Clear(const TRect&)` over the same rectangle does not.** The host table now carries `Clear(aRect)` and `Gc::clear()` means "the whole view". *Why* the no-argument form's region is narrower was **not** determined.
  - **Draw's coordinates, settled for the application** (spec §5.1's open item C): the shim hands Rust `TRect(TPoint(0,0), Size())`, not `Rect()`. Drawing through a window gc is window-relative, so a view that always lays out from `(0,0)` has no second coordinate system to get wrong — and a bar at `y = area.height - 40` lands exactly above the softkeys. Measured client area: **240×245**.
  - **The link needs a second `-u`.** The shim archive follows the Rust archive, because that is the direction the `symrs_*` references usually run — but a GUI application's one reference runs the other way, from the shim to `symrs_app_vtbl`, and ld 2.29.1 does not rescan. `-u symrs_app_vtbl` before the Rust archive makes that member be pulled on the first pass. The `-l:euser.dso -l:drtaeabi.dso`-before-the-archive ordering is untouched and still unit-tested.
  - **The s60 shim needs the case-fold overlay that `shims/common` does not.** Without it the Avkon header chain stops at `fbs.h`'s `#include <FbsMessage.h>` (the file is `fbsmessage.h`). `RustBuild` builds the same `SdkIncludeCaseFold` the C++ path builds, and only for a `[ui]` project.
  - **What symdev newly generates, and from what.** A new `[ui]` section (`kind = "avkon"`, `caption`, `short_caption`, `softkeys`) makes `RustBuild` write `build/<app>.rss` and `build/<app>_reg.rss` **as text** and compile them with the existing native `cpp` + `rcomp` (byte-verified on 143 SDK resources, experiment 56); no new binary resource writer exists. The registration carries `localisable_resource_file` and `localisable_resource_id`, which is exactly what `Rsc::registration`'s fallback cannot do — and the proof it worked is the title pane reading **`Bars`**, the manifest's `short_caption`. `[symbian] icon` now also builds `<app>_aif.mif` on the Rust path. `symdev package` re-derives its file list rather than being handed the build's, so all three were named there too; the `.pkg` now carries four files and the fallback registration no longer fires. The generated resources are small: `uidemo.rsc` 140 bytes, `uidemo_reg.rsc` 91, `uidemo_aif.mif` 268.
  - **`symdev test --emulator` works on a GUI application**, which is not obvious for a program that never exits: the application writes the result file from `construct`, and `await_report` returns as soon as it appears. Three cases pass — the framework reached the Rust `construct`, the view was sized (240×245) *before* `construct` as the ABI's order promises, and a redraw can be asked for from there. It tests the entry path; the screenshots test `draw` and `key`.
  - **The softkey is still dead, now confirmed for a Rust application too.** F2 (`EStdKeyDevice1`) with `cba = R_AVKON_SOFTKEYS_EXIT` drawn on screen leaves the process running, exactly as experiment 83 found for `examples/gui`. **Unknown — requires experiment.** The acceptance test is written against arrows and the selection key for this reason.
  - **A stale registration outlives an uninstall.** The very first run showed `uidemo` in the title pane rather than the caption; a later install of the same package showed `Bars`. Not diagnosed, and worth knowing because it can make a correct resource look broken once.
  - What this does not show: a device; a menu, a menu bar or `R_AVKON_SOFTKEYS_OPTIONS_EXIT` (no menu resource is generated, so `softkeys` has one value); the view architecture (`AVKON_VIEW`); more than one language in the resource (the Rust path compiles once, as `LANGUAGE_SC`); a leave raised from inside a Rust callback (nothing in `symbian-ui` can raise one); the vtable size check firing (no short table was built); pointer events; `Gc::text` measuring a string, which is why `gdi.dso` is on the line for a symbol nothing calls yet.

## 89. `std` for `target_os = "symbian"`: an application with no `#![no_std]` (T5, Rust SDK)

- **Requires:** experiments 65, 68, 69, 78, 79, 80, 81, 84, 85, 87, 88. Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §3, §4, §6a, §9, §11 **step 77** — the last row of the plan.
- **Procedure:** (1) find a way to build a *patched* standard library that does not edit the shared rustup component and is reproducible from this repository; (2) get an unmodified `std` to compile for `arm-symbian-e32` and count what breaks; (3) decide where the platform code lives, given that `std` cannot depend on the `symbian-*` crates; (4) fill in the facilities in the order the brief set — `alloc`, `stdio`, `time`, `thread_local`, `thread`, `sync`, `fs`, `net` — leaving `process`, `env` and `args` as the `Unsupported` they honestly are; (5) settle the entry point, since rustc generates no `lang_start` call for a `staticlib`; (6) decide where `println!` goes on a phone with no console; (7) an application with **no `#![no_std]`** reporting through `symdev test --emulator`; (8) then a crate from crates.io that needs only `std`, added and not vendored.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/89-std/` (the 52 210-byte E32); source is `symbian-rs/examples/std-hello`. `stdhello: 20 passed`, `symdev test --emulator` exit 0.

  - **How a patched `std` is built, and it is one environment variable.** `__CARGO_TESTS_ONLY_SRC_ROOT=<tree>/library` makes cargo's `-Zbuild-std` read the standard library from a copy instead of from `<sysroot>/lib/rustlib/src/rust/library`. Proved before anything else was written, with a `compile_error!` at the top of a copied `core/src/lib.rs`: the build failed inside the copy and named it. The variable wants the **`library/` directory**, not the `src/rust` above it — cargo reads `<root>/Cargo.toml`. A `cp -a` of the 82 MB tree takes 0.4 s, so materialising per build is free.
  - **`rustup toolchain link` does not work, and the reason is worth writing down.** A directory of symlinks that mirrors the nightly but carries a patched `lib/rustlib/src` is the obvious answer and it fails silently: `bin/rustc` as a symlink makes rustc resolve `/proc/self/exe` to the **real** toolchain, so `rustc --print sysroot` answers with the original and the patched source is never read. It would need `bin` hard-linked or copied onto the same filesystem as `~/.rustup` — more machinery for the same result. A `[patch]` on the sysroot crates was not tried: `[patch]` replaces a *dependency*, and `std` is not one.
  - **Reproducible as an overlay of twenty-odd files, not a fork.** `symbian-rs/rust-src/overlay/library/` holds them; `StdSrc::materialise` (`crates/symdev-build/src/std_src.rs`) copies the toolchain's `library/`, copies `crates/symbian-sys` in, copies the overlay over the top, and points cargo at the result. Nothing is vendored: the repository carries 0 of the 82 MB. **The replaced files carry a SHA-1**, in `symbian-rs/rust-src/overlay.toml`, of the toolchain file each was derived from, and materialising checks every one — a nightly bump that touches `sys/sync/mod.rs` stops the build and names it instead of quietly undoing whatever upstream changed. `symbian-rs/rust-src/README.md` is the whole procedure, including doing it by hand.
  - **Unmodified `std` gives exactly 14 errors in 5 dispatch files.** `sys/pal/mod.rs` already falls through to `unsupported` for an unknown `target_os`, and so does almost every per-facility backend, so the all-unsupported milestone is much closer than it looks. What has no `unsupported` variant, and therefore had to be real before anything would compile: `sys/alloc`, `sys/io/error`, `sys/random`, `sys/thread_local/key`, and `sys/sync`'s four — `no_threads` carries `#[cfg(target_has_threads)] compile_error!`, and this target has threads.
  - **Where the implementation lives, and why.** A `library/std` path dependency on `symbian-rs/crates/symbian-sys` **fails** with `error[E0463]: can't find crate for 'core'`: a crate outside the sysroot build graph is given no `--extern core`, and the supported route — an optional `rustc-std-workspace-core` dependency behind a `rustc-dep-of-std` feature, the way `libc` and `hashbrown` do it — needs a crates.io resolve for a crate that is `[patch]`ed to a path, which this offline host cannot do and which would put the entry in `symbian-rs/Cargo.lock` for the `no_std` path too. **What works is copying the crate into the patched tree** as `library/symbian-sys` with a rewritten manifest naming `core = { path = "../core" }`, exactly as `library/rustc-std-workspace-core` does. There is one source of truth in git and the copy is generated, so nothing drifts. Everything **above** the raw declarations — `fs`, `net`, `time`, `sync`, `thread_local` — was written into `sys/*/symbian` rather than shared, because those are `std`'s own interfaces and the `symbian-std` facade keeps its `no_std` versions; the two are not duplicates of each other but implementations of two different contracts over the same FFI.
  - **What is real, and what is `Unsupported` and why.**

    | facility | over | note |
    |---|---|---|
    | `alloc` | `User::Alloc`/`AllocZ`/`Free`/`ReAlloc` | experiment 68's allocator re-hosted, including the 8-byte alignment measurement and the over-aligned back pointer, and experiment 80's heap lock, which goes on when `sys::thread` creates the first thread and not before |
    | `io::Error` | `e32err.h` | every code mapped to an `ErrorKind` where the two mean the same thing and to `Uncategorized` where they do not; `format_error` prints the `KErr*` name, because there is no `strerror` and the name is the message |
    | `stdio` | `RFile` | see below |
    | `time` | `User::TickCount` + `UserHal::TickPeriod`, `TTime::UniversalTime` | experiment 85's clocks; the measured epoch, not the computed one |
    | `thread_local` | `UserSvr::DllTls` | **one kernel slot per key**, not experiment 88's single-slot table: `std`'s own `sys/thread_local/os.rs` wants exactly a `create`/`get`/`set`/`destroy` key, and experiment 88 measured that an EXE gets many slots. Keys come from one counter, so the live key space *is* a range and [`run_dtors`] can walk it — which is how destructors run on a platform whose kernel will not enumerate a thread's slots |
    | `thread` | `RThread` | experiment 80's own-heap-then-`SwitchAllocator` recipe, `Logon` + `WaitForRequest` for `join`, `User::After` for `sleep` and `yield_now` |
    | `sync` | `RSemaphore`, `RCondVar`, `RMutex` | see below |
    | `fs` | `RFs`, `RFile` | one session **per thread**; `read_dir` is `Unsupported` because `CDir::AddL` leaves and would need a shim |
    | `net` | — | **not done.** A full `TcpStream`/`UdpSocket`/`LookupHost` backend is a slice of its own; `symbian_std::net` (experiment 84) is still the way to a socket, and a `std` application reaches it through the `no_std` facade. This is the one gate of step 77 that did not close. (**Corrected 2026-09-21, experiment 90:** the reason first written here — that `sys/net` has no `unsupported` fallback — was wrong. `library/std/src/sys/net/connection/unsupported.rs` exists, `connection/mod.rs`'s `cfg_select!` has a `_ =>` arm onto it, and `sys/net/hostname/` has the same shape, so `std::net`'s types already compiled and merely answered `Unsupported`. The work was a backend, not a skeleton, and experiment 90 wrote it.) |
    | `process`, `env`, `args`, `pipe` | — | `Unsupported`, and legitimately: a Symbian process reads its command line with `RProcess::CommandLine` and has no environment at all. `std` supports that as a platform answer |
    | `path` | `unsupported_backslash` | backslash separators, and **no drive-letter prefix**: `Path::new("C:\\x").is_absolute()` is `false`. A gap, named rather than papered over |

  - **`Mutex` is an `RSemaphore` and `Condvar` is an `RCondVar`, and the pairing needed a third object.** `std`'s `Mutex` needs `try_lock`, and `RSemaphore` is the only primitive on 9.3 with a timed wait (`RFastLock` and `RMutex` have a bare `Wait()`), so the mutex is a one-token semaphore as in experiment 80 — non-recursive, which is what Rust's contract wants, and whether `RMutex` is recursive is not stated in `e32std.h` and has never been observed. But `RCondVar::Wait` takes an `RMutex&` and nothing else. So the condition variable **brings its own `RMutex`**, used for nothing but pairing, and hands the caller's mutex back and forth around it: `inner.Wait(); user.unlock(); cv.Wait(inner); inner.Signal(); user.lock()`. No wakeup is lost, and the ordering is the proof: a waiter takes `inner` *while it still holds the user's mutex*, so a notifier — which changed the condition under that mutex — cannot reach `inner.Wait()` until the waiter is already inside `cv.Wait`, which is where `inner` is released. `Once` and `RwLock` cost nothing at all: `std`'s `sys/sync/{once,rwlock}/queue.rs` are portable over atomics and `thread::park`, so they came free with a `Parker` over a zero-token `RSemaphore`.
  - **Where `println!` goes: `E:\symdev\stdout.txt`.** `User::InfoPrint` was the alternative and it is the wrong one — it is a server round trip that draws a dialog and holds it for seconds, so a program printing a hundred lines would not be printing, it would be interrupting the user. A file is also the channel `symdev` can already read: drive E is `~/.local/share/EKA2L1/data/drives/e/`, which is where `symdev test --emulator` already collects the result file. Both streams append to the one path, in write order, as a console does; `Stdin` is always at end of file. What is **not** hidden is a crash: `sys::pal::symbian::abort_internal` ends the process with `User::Panic(_L("RUST"), KErrGeneral)`, which the platform shows on screen itself, and the message that goes with it is in the file. Verified: after the run, `~/.local/share/EKA2L1/data/drives/e/symdev/stdout.txt` holds both lines the example printed.
  - **The entry point: `std::os::symbian::start`.** On every other platform the C runtime calls `main`, rustc generates the `lang_start` call and the linker ties them together. Neither happens here — the image's entry is the C++-mangled `_Z7E32Mainv` that `eexe.lib` reaches, and the application crate is a `staticlib`, in which rustc never looks for a `fn main`. So the overlay adds `rt::symbian_start` (private `lang_start_internal`, plus `sys::thread::drop_thread_locals` for the main thread, which nothing in this kernel does) and exposes it as `std::os::symbian::start`, `#[stable]`, so the application needs no feature gate. `#[symbian_std::main]` now expands to `::symbian_std::__start(main)` in **both** shapes, and `symbian-std` has one `__start` per shape — `ExitCode::from_main` without `std`, `std::os::symbian::start` with it. The `no_std` path is untouched and every example still builds to the byte.
  - **`symbian-std` had to split, and feature unification forced the workspace to split too.** The crate gained `runtime` (default: the allocator, the panic handler, `ExitCode`) and `std`; a `std` application takes `default-features = false, features = ["std"]` and gets only the entry attribute, the prelude and `test_report`. It is not tidiness: `symbian_std::thread::spawn` and `std::thread::spawn` each switch heap serialisation on behind their **own** flag, so a program with both would put two locks over one heap. And because cargo unifies features across a workspace, `examples/std-hello` is its own workspace — as a member it asked `symbian-std` for both halves at once, which is two `__start`s and an `extern crate std` in a build that has no `std`.
  - **`std/build.rs` needed one line.** Without `|| target_os == "symbian"` in the not-`restricted_std` list, every `use std::…` in an application is `error[E0658]: use of unstable library feature 'restricted_std'`. That list is the compiler's own statement of which platforms have a real `std`, so adding the target to it is the change, not a workaround.
  - **Sizes.** The full example is **52 210 bytes**; a minimal `std` hello — `fs::write` and nothing else — is **29 512**, against the `no_std` hello's 3 187. What dominates is `std` and `core` themselves and not the platform layer: of 43 705 bytes of linked symbols in the minimal one, `core` is 20 350 and `std` 16 081, and the largest single items are `std::panicking::panic_with_hook` (2 100), `core::fmt`'s formatter and `Debug` machinery, and the `core::unicode` tables (1 461 + 767) that the panic and `Debug` paths pull in. The Symbian backends are small by comparison: `sys::fs::symbian::file::File::open` is 1 028 bytes and `DirBuilder::mkdir` 728. In the full example the application's own `main` is 5 060 and `ryu` 2 072. The `-l:euser.dso -l:drtaeabi.dso`-before-the-Rust-archive ordering is untouched.
  - **Every other example is unchanged**: `hello-raw` 752, `hello` 3 187, `shim` 4 474, `alloc` 4 474, `files` 10 552, `atomics` 11 719, `ui` 12 715, `net` 13 379, `tls` 16 272, `time` 20 583, `async` 21 659. (`shim` is 4 474 and not the 4 475 the plan recorded; `main` builds 4 474 too, so that figure was already stale.)
  - **Crates from crates.io, added and not vendored.** Compiled unchanged for `arm-symbian-e32`: **`itoa`, `ryu`, `memchr`, `base64`, `hex`, `bitflags`, `log`, `serde_json`** (with `serde_core`, `itoa`, `ryu`, `memchr`), **`regex`** (with `regex-automata`, `regex-syntax`, `aho-corasick`) and **`chrono`** (`default-features = false, features = ["clock"]`, with `num-traits`). `itoa` and `ryu` are in the passing example and their output is checked inside the emulator. **`rand` does not compile**, and not because of anything `std` is missing: `getrandom` 0.3 keeps its **own** list of supported targets in `src/backends.rs` and `compile_error!`s on one it does not know. `std`'s own randomness works — the example's `HashMap` case passes, which is `sys::random::symbian` over `Math::Random`.
  - **What an application's first three lines are now**, which is the whole point of the step:

    ```rust
    use std::fs::File;
    use std::io::{Read, Write};
    use std::time::Instant;
    ```

  - **Open, and stated rather than left to be discovered:** `std::net` (the facade is still the way to a socket); `read_dir`; a drive letter as a `Path` prefix; `File` crossing threads, which the type system allows and the file server will answer `KErrBadHandle` to, because `RFs::ShareProtected` has not been observed; `process`/`env`/`args`, which are `Unsupported` by choice; **`symdev new` has no `rust-std` template**, so a `std` project is written by hand from `examples/std-hello` (the manifest line, `default-features = false, features = ["std"]`, its own `[workspace]` and `[profile.release]`, and a `.cargo/config.toml` naming `-Zbuild-std=std,panic_abort`); and, as for every step so far, **no device has run any of this**.

## 90. Closing the gaps step 77 left in `std`: paths, directories, arguments, sockets, processes (T5, Rust SDK)

- **Requires:** experiment 89 (and, through it, 70, 74, 78, 79). Governed by [the design spec](../superpowers/specs/2026-09-20-rust-sdk-design.md) §6a and §11 **step 77**.
- **Procedure:** (1) parse a drive letter as a `Path` prefix, after settling from the SDK what a drive-relative path means on Symbian; (2) `read_dir` over `RFs::GetDir`, and `args` over the command line, leaving `env` `Unsupported` on purpose; (3) a real `sys::net` backend, a port of step 74 rather than research, with each of `sys::net`'s wider internal contract refused individually; (4) `process` as far as `RProcess` goes, and honest about the rest; (5) all of it exercised through `symdev test --emulator`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/90-std-gaps/` (three E32s: `stdhello.exe` 73 633, `stdnet.exe` 53 359, `spawnee.exe` 3 208); sources are `symbian-rs/examples/std-hello`, `std-net` and `spawnee`. `stdhello: 50 passed`, `stdnet: 31 passed`, `symdev test --emulator` exit 0 for both.

  - **Symbian has no per-drive current directory, and `f32file.h` says so in as many words.** The `RFs` class documentation lists what the file server is for, and point 5 is: "maintaining a default path; **unlike some other systems, there is a single system default path, rather than one for each drive**: the default path consists of a drive and a path specification." That single default is the **session path**, one per `RFs` session (`RFs::SessionPath`/`SetSessionPath`, lines 1733-1734), and `RFs::Parse` is what merges an incomplete name against it. So `C:x` is still relative — it needs that path's *directory* — and `Path::is_absolute` is `false` for it, the same **shape** as Windows for a different **reason**: on Windows each drive remembers a directory of its own, so `C:x` and `D:x` can resolve under different ones, while here both take the one session path's directory and differ only in the drive. `sys/path/symbian/` therefore parses `E:` as `Prefix::Disk` and nothing else — there is no UNC, no `\\?\` verbatim form and no device namespace on this platform. The drive-letter parser is a file of its own (`drive.rs`) so that the **host** can test it: `crates/symdev-build/src/std_src/tests.rs` compiles that exact file through `#[path]`, and `cargo test --workspace` covers the grammar.
  - **`std::path::absolute` stays `Unsupported`**, and names what it would need: `RFs::Parse` against the session path, through a `TParse` whose layout has not been measured. Reimplementing the merge in Rust would be a guess at rules only the file server states.
  - **The real blocker for `read_dir` was not a leave. No thread had a `CTrapCleanup`.** `RFs::GetDir` returns a `TInt` and traps its own `GetDirL` — proved twice over, once by wrapping it in a shim `TRAP` that changed nothing, and once by the emulator's own `Leave trapped by trap handler` for the missing-directory case. What actually happened is that the cleanup stack is **per thread** and nothing in this SDK ever installed one, so the first `CleanupStack::PushL` inside `GetDirL` panicked `E32USER-CBase 69` (`EClnNoTrapHandlerInstalled`, `e32panic.h` line 2667) and killed the thread. A panic is not a leave and no `TRAP` catches it. **It looked exactly like experiment 76's silent death and was not**: EKA2L1 logs it, at `Kernel:trace`, which the default `log-filter` (`Kernel:Warn`) hides — three hours of the wrong hypothesis are the cost of that line in the config. `std` now installs a `CTrapCleanup` in `rt::symbian_start` and in `sys::thread`'s trampoline, which is what a Symbian `E32Main` and every thread it creates conventionally do. **The `no_std` runtime still installs none**, which is a step of its own.
  - **`read_dir` needs one shim, and it is not a `TRAP`.** `CDir`'s leaving entry points (`NewL`, `AddL`, `ExtractL`) are **protected** and reachable only by its `friend class RFs`; `Count()` and `operator[]` are non-virtual `const` members Rust calls directly. What needs C++ is `delete aDir`, because `~CDir()` is `IMPORT_C virtual` — rule 3 of `symrs_shim.h`. Reaching for the exported `_ZN4CDirD0Ev` instead would be assuming the dynamic type. The entry's name comes from `TEntry::iName` at the offset experiment 79 already measured (28), read through euser's own `TDesC16::Ptr()` and the documented length mask, so nothing guesses how a `TBufC16` stores its units.
  - **`args` was written off as impossible and is not, but it is `User::` and not `RProcess::`.** This SDK's `euser.dso` exports `_ZN4User11CommandLineER6TDes16` and `_ZN4User17CommandLineLengthEv` (`e32std.h` 4569-4570) and **no** `RProcess::CommandLine` at all. It gives **one string**, and nothing in `e32std.h`, `e32cmn.h` or `f32file.h` states a quoting convention, an escape character or a separator — no Symbian API anywhere in this SDK splits a command line into a vector. So there is nothing to imitate and the rule is ours: **split on ASCII whitespace, and there is no quoting.** A program that needs an argument with a space must agree its own encoding with whatever starts it, which is what it would have to do in C++ here too. `std::env::args()`'s first element is `RProcess().FileName()` — the running image's own path — because Symbian's command line does **not** carry the program name (`RProcess::Create` takes the image and the command as two separate arguments), and every argument parser in the language skips element 0.
  - **`env` is `Unsupported` deliberately and permanently, and the module says why.** Symbian has no environment: no `environ`, no `getenv`, and `RProcess::Create` passes one descriptor and nothing else. Backing `std::env::var` with a process-local map would be a lie the first child process exposes — on every other platform `set_var` is inherited. One thing did change from the `unsupported` backend: `env()` returns an **empty iterator** instead of panicking, because "this platform has no environment variables" is a fact and an empty list is its honest spelling.
  - **`std::net` is step 74's layer re-hosted, and the step-73 rule came with it.** `sys/net/connection/symbian/` is session, request, addr, socket, tcp, udp. Real: `TcpStream::connect`/`read`/`write`/`shutdown`/`peer_addr`/`socket_addr`, `TcpListener::bind`/`accept`, `UdpSocket::bind`/`send_to`/`recv_from`/`connect`/`send`/`recv`, and `ToSocketAddrs` over `RHostResolver::GetByName`'s synchronous overload. The session is **per thread**, as `sys::fs`'s is, because `es_sock.h` has no `RSocketServ::ShareAuto` and nothing has observed the socket server accepting a shared session. And every blocking call refuses under a `CActiveScheduler` rather than eating its completions: `examples/std-net` installs one through the shim and `TcpStream::connect` answers `ResourceBusy` (`KErrInUse`) — **verified, not asserted**, so an Avkon program or anything under `block_on` gets an error and not a hang.
  - **What `sys::net` refuses, and why each one.** *Timeouts* — a deadline needs an `RTimer` request outstanding beside the socket's on the same thread, which one `User::WaitForRequest` may not have; that is an executor's job. *`set_nonblocking(true)`* — an `RSocket` has no non-blocking mode at all; its asynchronous form is the request/`TRequestStatus` pair, not a call that returns `EWOULDBLOCK`. `set_nonblocking(false)` succeeds, because blocking is what this backend is. *`peek`* — `es_sock.h` names `KSockReadPeek` but nothing here has issued it. *`try_clone`* — `RSocket::Transfer` moves a socket to another **thread**; there is no dup within one. *Every socket option* — `linger`, `nodelay`, `keepalive`, TTL, broadcast, multicast: all `SetOpt`/`GetOpt` with a constant out of `in_sock.h`, and this repository has never set one and watched what changed. *IPv6* — `TInetAddr` carries it and a v4-mapped answer is converted, but no packet has ever gone to a real IPv6 address on this stack. *`hostname`* — left on the `unsupported` arm; a phone's "name" is a Bluetooth setting.
  - **`process` is half real and says which half.** `RProcess::Create(exe, cmdline)` + `Resume` + `Logon` + `User::WaitForRequest` give `Command::spawn`, `wait`, `try_wait`, `kill` and an `ExitStatus` that carries Symbian's **two** values, `ExitType()` and `ExitReason()` — so `code()` answers `Some` only for `EExitKill` and `None` for a panic, as it does for a signal on Unix. `std::process::id` and `getpid` go through a shim because `RProcess::Id()` returns an 8-byte `TProcessId` the EABI returns indirectly. **`std::process::exit` is now `User::Exit`**: `sys/exit.rs`'s fallback arm is `intrinsics::abort()`, an undefined instruction that surfaces as a bare `KERN-EXEC 3` and loses the code. What is refused at the spawn rather than dropped: a working directory (the file server keeps one session path, not one per process), any environment change, and any stdio that is not "leave it alone" — so `Command::spawn()?.wait()` and `status()` are real while `output()` and `Stdio::piped()` are `Unsupported`.
  - **`RPipe` does not exist in this SDK, and that was checked rather than recalled.** No header under `epoc32/include` declares it; it is Symbian 9.4 and later. So there is nothing for a child's output to go into.
  - **Two EKA2L1 limitations this slice measured, neither of them ours.** (1) **It cannot spawn an image with a writable data section through the loader.** `RProcess::Create` succeeds, the emulator gives the child an extra `anonymous` chunk of 0x1000 bytes at 0x400000 for its data, and the child dies with `KERN-EXEC 3` reading its own heap base + 0xA4 before it reaches `main`; the parent then waits forever on a `Logon` that never completes. Every `std` image tried does this and no `no_std` one does — the emulator logs their `runtime data` as `0x0`. That is why `examples/spawnee`, the child `std-hello` spawns, is `no_std`. (2) **Its `mkdir` answers `KErrAlreadyExists` where Symbian answers `KErrPathNotFound`** when the *parent* of a new directory is missing, so `std`'s `create_dir_all` reads that as "already there" and stops — more than one missing level silently creates nothing. (3) A file's size reads back as 0 from both `RFs::Entry` and a directory listing until `RFile::Flush`; closing the handle is not enough.
  - **One shim wrapper per translation unit, and it is measured.** The recorded GCCE argv carries no `-ffunction-sections`, so an object's whole `.text` is one section and `--gc-sections` can only drop it whole: putting `symrs_f32_dir_delete` in `symrs_f32.cpp` cost `examples/shim` 26 bytes. Splitting it, `symrs_cleanup_destroy` and the two `RProcess` wrappers into files of their own brought every `no_std` example back to its recorded size **to the byte**.
  - **Measured with a compile probe on the recorded GCCE argv**, the way experiments 78 and 79 did: `sizeof(TPtr16)` **12** (align 4), `sizeof(RProcess)` **4**, `sizeof(TBuf16<256>)` **520**, `sizeof(TRequestStatus)` **8** (confirming experiment 84).
  - **Sizes.** `std-hello` **73 633** (was 52 210 with 20 cases; now 50), `std-net` **53 359**, `spawnee` **3 208**. Every `no_std` example is unchanged to the byte: `hello-raw` 752, `hello` 3 187, `shim` 4 474, `alloc` 4 474, `files` 10 552, `atomics` 11 719, `ui` 12 715, `net` 13 379, `tls` 16 272, `time` 20 583, `async` 21 659. The `-l:euser.dso -l:drtaeabi.dso`-before-the-Rust-archive ordering is untouched.
  - **Open, and stated rather than left to be discovered:** `std::path::absolute`; the socket option surface; IPv6; pipes, which need a platform that has them; `symdev new` still has no `rust-std` template; the `no_std` runtime installs no `CTrapCleanup`, so a `no_std` program that reaches a framework call using the cleanup stack still dies as `std` did; and, as for every step so far, **no device has run any of this**.

## 92. The Avkon notes: a modal popup an application shows with one call (T5, Rust SDK)

- **Requires:** experiment 86 (the Avkon shim and `symbian-ui`), 85 (`Instant`), 83 (keys into the emulator). Design: [avkon-rust-spec.md](avkon-rust-spec.md) §3 (the forwarding ABI) and §4.3 (the leave rule).
- **Procedure:** (1) settle from `aknnotewrappers.h` and `nm -D avkon.dso` which note classes this SDK has; (2) `symbian-rs/shims/s60/symrs_note.cpp` — one `extern "C"` entry, one `TRAP`, the four classes; (3) `symbian-rs/crates/symbian-ui/src/note.rs` — `note::info`/`confirm`/`warn`/`error(&str) -> Result<()>`; (4) `symbian-rs/examples/notes` through `symdev build`/`package`/`run`/`test --emulator`, with a PID-bound screenshot per kind either side of `docs/research/acceptance/emukey.py`.
- **Outcome:** pass (emulator only; no device)
- **Evidence:** 2026-09-21, `symbian-rs/corpus/92-notes/` (the 15 298-byte E32 and eight screenshots); source is `symbian-rs/examples/notes`.

  - **Four note classes, and the SDK has no fifth.** `aknnotewrappers.h` declares `CAknNoteWrapper` (the base, `ExecuteLD(TInt)` and `ExecuteLD(TInt, const TDesC&)`), `CAknResourceNoteDialog` (a helper base that stores the resource id, so it has `ExecuteLD()` and `ExecuteLD(const TDesC&)`), the value class `TAknNoteResData`, and the four concrete ones: **`CAknConfirmationNote`, `CAknInformationNote`, `CAknWarningNote`, `CAknErrorNote`**. All four are wrapped. Each has three constructors — `()`, `(TBool aWaitingDialog)` and `(T** aSelfPtr)` — and only the default one is used; the other two have no observed need behind them yet. `ExecuteLD(const TDesC&)` is exported by the **base**, `_ZN22CAknResourceNoteDialog9ExecuteLDERK7TDesC16`, not by the four; with the four `C1Ev` constructors that is the whole import list, all five from `avkon.dso`, which is already on a `[ui]` project's link line. **No new import library.**
  - **`CAknGlobalNote` is deliberately not wrapped.** `aknglobalnote.h` is a different thing — `NewL`/`NewLC`, `ShowNoteL(TAknGlobalNoteType, const TDesC&)`, `CancelNoteL(TInt aNoteId)` — a note the notifier server draws, which outlives the application that asked for it and can be cancelled by id. It has no place in a module whose whole premise is that a note needs no owner. Left for whoever has a use for one.
  - **Ownership under the `TRAP`: the header does not say, and the safe reading is taken and written down.** `eikdialg.h` on `CEikDialog::ExecuteLD` says it "loads, displays, and destroys the dialog... there is no need for the application program to destroy the dialog" and says **nothing about the leave path**; neither does `aknnotewrappers.h`. So the shim assumes the object has already deleted itself on **every** exit path including a leave: it never deletes after the `TRAP` and never pushes the note on the cleanup stack. Being wrong that way leaks one dialog on a path that only runs when the note could not be shown at all; the other way round is a double delete. **Unknown — requires experiment** (it needs `CEikDialog::RunLD` traced through a second ROM DLL).
  - **A note does not block, measured three ways.** `eikdialg.h` says `ExecuteLD` "returns immediately unless `EEikDialogFlagWait` has been specified in the `DIALOG` resource", and the four `R_AKN_..._NOTE` resources do not set it. Measured with `symbian_std::time::Instant` around the call: **62 500 µs** for the first note of a process (four ticks of the 15 625 µs clock — Avkon's own warm-up) and **15 625 µs**, one tick, the smallest non-zero this clock can express, for every one after it. A note that waited would be dismissed by a person or by a 1.5–3 s timeout, which is 96–192 ticks. The third and plainest proof is in the pixels: the application repaints its `shown=<n>` counter *after* the call returns, and every screenshot shows the incremented counter **with the note still on screen**.
  - **A note can be shown from `construct`,** which was the open question about whether one needs `CCoeEnv` to exist yet. The shim has already run `BaseConstructL` by then, so it does; `examples/notes` shows one there and `symdev test --emulator` passes on it. A note shown with no `CEikonEnv` is refused by the shim with `KErrNotReady` rather than dereferencing a null inside Avkon — a guard, not an observation, because nothing in a `[ui]` build can reach that state.
  - **One shim entry, not four.** `symrs_note_show(TInt aKind, const TUint16*, TInt)` with a kind opcode, because the four kinds differ only in which class is `new`ed and the signature is identical — none of §3.1's argument-erasure objection to shape C applies. The recorded GCCE argv carries no `-ffunction-sections`, so four functions in one translation unit would cost the same as one anyway. An unknown kind is `KErrArgument`, not a default note.
  - **What all four cost: 1 056 bytes of `.text`, 647 of E32.** A/B in one tree with the four `note::*` calls replaced by a function returning `Ok(())`: `.text` 22 996 → 21 940, E32 15 298 → 14 651. `symrs_note.o` is 2 796 bytes with 18 undefined symbols. Twelve `NEEDED`, the eleven of experiment 86 plus `efsrv` for the result file.
  - **A console application and a GUI one that shows no note pay nothing.** `examples/ui` rebuilt on this branch is **12 715** and differs from `corpus/86-ui/uidemo.exe` only in the E32 header's timestamp and checksum words: hidden visibility plus the archive member never being pulled means `--gc-sections` drops the whole object and the four avkon imports with it. Every other example is unchanged, `hello-raw` 752 among them.
  - **Text is a `&str` and never a descriptor** (design spec §6a). `Note::show` encodes into a 256-unit stack buffer — `MAX_NOTE_TEXT`, which is the one length the SDK names for note text, `TAknNoteResData::iText` being a `TBuf<256>` — and returns `KErrOverflow` beyond it. That field is the *resource-borne* text and not the descriptor passed here, so the bound is defensible rather than measured: **what Avkon does with a longer prompt was not observed.**
  - **A route into the ROM's own code, worth keeping for the next SDK question.** The header was silent, so disassembly was tried before the safe reading was settled for. Two facts make it cheap: a `.dso` export stub's four bytes at a symbol's address **are its ordinal** (`_ZN22CAknResourceNoteDialog9ExecuteLDERK7TDesC16` is 1996, read straight out of `avkon.dso`'s `ER_RO` at the `nm -D` address plus the section's file offset), and the ROM `avkon.dll` under `~/.local/share/EKA2L1/data/drives/z/rm-469/sys/bin/` is a raw `TRomImageHeader` image, so ordinal *n*'s entry is at file offset `iExportDir - iCodeAddress + 0x78 + (n-1)*4` and the address it holds maps back the same way. **`sizeof(TRomImageHeader)` is 0x78 on this ROM** — brute-forced by scoring candidate offsets against Thumb prologues, because `e32rom.h` declares the class but the SDK never states its size. Entry addresses are odd: everything in avkon is Thumb. It did not settle the leave path within a sensible budget — that needs `CEikDialog::RunLD` traced through a second DLL — but it is the tool for the next time a header says nothing.
  - What this does not show: a device; the waiting forms of the four classes (`CAknInformationNote(ETrue)` → `R_AKN_INFORMATION_NOTE_WAIT`) and therefore what `ExecuteLD`'s `TInt` return means when it is not zero; `CAknGlobalNote`; the tone and timeout setters of `CAknNoteDialog`; `SetImageL`; a note raised from `draw` (the type system makes it unwriteable); and the leave path of `ExecuteLD`, which is the safe reading above and not an observation.

## 93. Avkon's query dialogs from Rust: a modal prompt that needs no control stack (T5, Rust SDK)

- **Requires:** experiments 76 (the ABI probe), 78 (the shim and the member ABI), 83 (keys into the emulator), 86 (the Avkon application). Design: [avkon-rust-spec.md](avkon-rust-spec.md) §3 and §4.
- **Procedure:** (1) read `aknquerydialog.h`, `eikdialg.h`, `avkon.rsg` and `avkon.rh` in `~/sdk/S60_3rd_FP2/epoc32/include/`, and `nm -D` `epoc32/release/armv5/lib/avkon.dso`, to settle which query classes exist, which resource ids the ROM already has and what `ExecuteLD` owns; (2) `symbian-rs/shims/s60/symrs_query.cpp` — two `extern "C"` wrappers, each a complete `TRAP` unit; (3) `symbian-rs/crates/symbian-ui/src/query.rs` — `text` and `number` over them, plus one `pub mod query;` line in `lib.rs`; (4) `symbian-rs/examples/query` through `symdev build`/`package`/`run`/`test --emulator`, driven with `docs/research/acceptance/emukey.py` and screenshotted either side of each key.
- **Outcome:** pass for text and number (emulator only; no device); **refused** for a confirmation query, which has no resource.
- **Evidence:** 2026-09-21, `symbian-rs/corpus/93-query/` (the 20 120-byte E32, four screenshots and the pixel figures); source is `symbian-rs/examples/query`.

  - **One factory covers every query kind.** `aknquerydialog.h` declares `CAknQueryDialog` (base) and the concrete `CAknTextQueryDialog`, `CAknNumberQueryDialog`, `CAknTimeQueryDialog`, `CAknDurationQueryDialog`, `CAknFloatingPointQueryDialog` and `CAknMultiLineDataQueryDialog`. None of them needs naming: the base's overloaded `static CAknQueryDialog* NewL(X&, const TTone&)` returns the right subclass for `TDes&`, `TInt&`, `TTime&`, `TTimeIntervalSeconds&`, `TReal&`, `TInetAddr&` and `TPosition&`, and `nm -D` on `avkon.dso` exports all of them. The shim therefore imports exactly three symbols — `_ZN15CAknQueryDialog4NewLER6TDes16RKNS_5TToneE`, `_ZN15CAknQueryDialog4NewLERiRKNS_5TToneE`, `_ZN15CAknQueryDialog9ExecuteLDEiRK7TDesC16` — and **wrapped are text and number**. Time, duration, float, IP and position are left; each is one more overload and one more ROM resource id, and none was needed yet.
  - **The resource ids are in the ROM and we generate nothing — except for a confirmation.** `avkon.rsg` defines `R_AVKON_DIALOG_QUERY_VALUE_TEXT` 0x8cc0052, `_NUMBER` 0x8cc0053, `_PHONE` 0x8cc0054, `_TIME` 0x8cc0055, `_DATE` 0x8cc0056 and `_DURATION` 0x8cc0057, so the shim `#include <avkon.rsg>` and the ids are the SDK's rather than typed out. **There is no confirmation query id in any `.rsg` in `epoc32/include`.** `AVKON_CONFIRMATION_QUERY` is only a resource STRUCT in `avkon.rh` line 165 — a shape an application instantiates in its own `.rss` — so an in-app confirmation needs symdev to generate a `RESOURCE DIALOG` with `buttons = R_AVKON_SOFTKEYS_YES_NO` (0x8cc0024, which *is* in the ROM) into the `<app>.rss` that `ui_resources.rs` already writes, **and** a way for the shim to learn the id that file gets — whether the application's resource-file offset is already folded into it was **not observed**. So no `symbian_ui::query::confirm` exists and none was guessed. The exact resource text is at the foot of `symrs_query.cpp`. `CAknGlobalConfirmationQuery` was considered and rejected: it lives in `aknnotify.dso`, is asynchronous, and is a *global* notifier query rather than an in-application one.
  - **`ExecuteLD` under a `TRAP`: the SDK is silent, so the safe reading.** `eikdialg.h` says `ExecuteLD` "loads, displays, and destroys the dialog … there is no need for the application program to destroy the dialog", and says nothing at all about what happens when it leaves. `PrepareLC` pushes the dialog on the cleanup stack, so a leave from any point after that already destroys it. The shim therefore **never deletes the dialog after a leave**; the cost is that a leave raised in the window between `NewL` returning and `PrepareLC` pushing would leak one dialog, and that window was not observed. A leak in an error path is the lesser fault against a double free.
  - **The maximum input length is an argument, not a default.** `CAknTextQueryDialog` writes into a descriptor the *caller* owns and uses that descriptor's `MaxLength` as the bound, so a number has to come from somewhere. `query::text(prompt, max_len)` takes it; the Rust side allocates a `Vec<u16>` of exactly that many code units, the shim wraps it in `TPtr16(buf, 0, max)`, and the length that comes back is `TDes16::Length()`, clamped on the Rust side rather than trusted. A `max_len` of 0 or above `query::MAX_TEXT_LEN` (1024, a bound on the allocation and not an Avkon limit) is `KErrArgument` before any C++ frame exists. Nothing defaults to 256.
  - **Driven, not merely shown.** `Return` opens the text query (23 494 of the client area's 58 800 pixels change — the dialog covering the bottom 98 rows); `5` `5` `5` puts `555` in the field (171 pixels, box (20,207)–(47,219)) and turns the hidden left softkey into "OK"; a second `Return` **confirms** it and the application's own `draw` repaints its label as `name=555` (1 469 pixels, box (72,63)–(191,83)). `Up` opens the number query with the `initial` this side passed (`7`, selected), `4` `2` `Return` gives `age=42` (635 pixels). The small figures are the point: the text is inside the Rust `String`, not merely on screen.
  - **The confirm path is not blocked on a softkey — and the softkeys work here anyway.** `Return` (`EStdKeyDevice3`) both opens and confirms a data query, so the acceptance test never needs `F1`. Separately, and contradicting how [eka2l1-input.md](eka2l1-input.md) reads: **`F2` cancels an open query and `F1` confirms one.** A query dialog's CBA comes from the ROM's own resource, so whatever swallows the softkeys is specific to a CBA built from the `.rss` symdev generates — a discriminator for that investigation, not a fix. `Escape` does nothing for a duller reason: it is not bound. The profile at `~/.local/share/EKA2L1/bindings/default.yml` has 22 binds and they are `F1`–`F4`, `Return`, the four arrows, `0`–`9`, `*`, `/` and `Backspace`.
  - **A query can run from `construct`.** A throwaway build that opened `query::text("FromConstruct?", 32)` from the `construct` callback showed the dialog during startup, took `7` `7` `Return`, came back `Some("77")` and carried on normally. So `CCoeEnv` already exists when `CShimAppUi::ConstructL` calls into Rust — CONE builds the environment before the app UI — and `construct` simply blocks inside the dialog's own loop. The shim still checks `CEikonEnv::Static()` and answers `KErrNotReady` rather than crashing if one is ever asked for earlier.
  - **Shape A, and no shared abstraction invented.** A query needs no place on the control stack, no observer and no state, so it is two free `extern "C"` symbols rather than fields on `SymRsHost`: `symrs_avkon.h` and `crates/symbian-ui/src/abi.rs` are untouched, and the only shared edit is one `pub mod query;` line in `symbian-ui/src/lib.rs`.
  - **It costs a program that asks nothing nothing at all.** `querydemo.exe` is **20 120 bytes**; `uidemo` is still **12 715** and `hello-raw` still **752**, byte-identical to `main`. `symrs_query.cpp` is a new member of `build/shims/libsymrs.a` and an archive member nobody references is never pulled — not even `avkon.dso`'s import of `CAknQueryDialog`. The 7 405 bytes over `uidemo` are `alloc::String`, `core::fmt`'s `Debug` for the report details and the wrappers.
  - **`symdev test --emulator` reports 4 passed, exit 0**, on a run nobody drove: the entry path (client area 240×245) and the three guards `query::text` applies before reaching the framework. The driven run writes 6, the extra two being `a text query came back: "555"` and `a number query came back: 42`. No case needs a key press to pass — a test that fails until somebody types is a broken test, not a pending one — so the dialogs are evidenced by the screenshots.
