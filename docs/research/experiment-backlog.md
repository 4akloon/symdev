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
