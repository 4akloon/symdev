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
- **Outcome:** skip
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
