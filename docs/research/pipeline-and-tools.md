# Pipeline and tools

Bootstrap note for M0. Facts are **Verified**, **Unknown**, or **Needs experiment**. There are no **Likely** claims here; do not treat gaps as Verified.

Command fragments below are promoted from spec §4.2 only. This note does not invent argv, clone URLs, or install steps. Primary sources are the plan-time URLs listed in § Sources.

## Verified pipeline

```
arm-none-symbianelf-g++  →  arm-none-symbianelf-ld  →  elf2e32  →  makesis  →  signsis
```

`abld` and `makmake` (Perl) are bypassed entirely on this path.

Never pass `-fPIC` or `-fPIE` (Verified: causes “Import relocation does not refer to code segment”).

## Sources (plan-time)

Do not invent further commands from these URLs. Later runbook work may copy documented steps from them and then experiment.

- GCC project: [https://github.com/fedor4ever/GCC4Symbian](https://github.com/fedor4ever/GCC4Symbian) (`build-toolchain.sh`; README warns incompatible with abld).
- Prebuilt GCC 14.2.0 + binutils 2.29.1: [https://sourceforge.net/projects/gcce4symbian/files/GCC-14.2.0_BINUTILS-2.29.1/](https://sourceforge.net/projects/gcce4symbian/files/GCC-14.2.0_BINUTILS-2.29.1/)
- Blog: [https://fedor4ever.wordpress.com/2024/09/22/gcc-14-1-0-for-symbian-out/](https://fedor4ever.wordpress.com/2024/09/22/gcc-14-1-0-for-symbian-out/)
- `elf2e32_next` (tested S60_3rd_FP2_SDK_v1.1): [https://github.com/fedor4ever/elf2e32_next](https://github.com/fedor4ever/elf2e32_next)
- Older C++14 port: [https://github.com/fedor4ever/elf2e32](https://github.com/fedor4ever/elf2e32)

## GCC branches (Verified)

There is no `darwin*` GCC branch (Verified per research prompt). A `linux*` branch exists.

Exact clone commit and build/install flags: **Unknown**. Whether to build from the GCC4Symbian source tree or use the SourceForge GCC 14.2.0 + binutils 2.29.1 tarball: **Unknown** / **Needs experiment**.

## Compile flags (Verified)

Verified against fedor4ever’s manual-build write-up:

```
-O2 -fexceptions -march=armv5t -mapcs -mthumb-interwork -mthumb -msoft-float
-D__SYMBIAN32__ -D__EPOC32__ -D__MARM__ -D__GCCE__ -D__EXE__
-include <SDK>/GCCE.h
-D__PRODUCT_INCLUDE__="<SDK>/Symbian_OS.hrh"
```

Optional size flags (Verified as a size-reduction technique, not required for a first hello): `-ffunction-sections -fdata-sections` plus linker `--gc-sections --strip-discarded`.

## Link flags (Verified)

```
--target1-abs --no-undefined -nostdlib -shared
-Ttext 0x8000 -Tdata 0x400000 --strip-debug
--entry _E32Startup -u _E32Startup
```

Link against: `eexe.lib`, `usrt2_2.lib`, `euser.dso`, `dfpaeabi.dso`, `drtaeabi.dso`, `scppnwdl.dso`, plus `-lsupc++ -lgcc`.

Exact library search paths, C runtime objects, and flag order are **Needs experiment**. The runbook may only assemble argv from Verified fragments plus experimentally recorded paths.

## elf2e32 (Verified fragment)

```
elf2e32 --uid1=0x1000007a --uid3=<UID3> --capability=<caps> --fpu=softvfp
        --targettype=EXE --output=hello.exe --elfinput=hello.elf
        --linkas=hello{000a0000}[<UID3>].exe --libpath=<SDK>/armv5/LIB
```

soname / `--linkas` format is `<name>{version}[uid3].<ext>` and **must match** between the linker `-soname` and `elf2e32 --linkas` (Verified).

How to obtain/build `elf2e32` on Linux: **Unknown**. Plan-time sources: `elf2e32_next` (tested S60_3rd_FP2_SDK_v1.1) and the older C++14 port. Do not invent build argv.

## Signing (Verified)

Signing is **self-sign only** in this product phase. Symbian Signed is closed (Verified). `makekeys -expdays 3650` requires Symbian 9.2+ tools (Verified). Without `-expdays` a certificate lasts about one year.

Whether native Linux binaries exist for `makesis` / `signsis` / `makekeys`, or Wine is required: **Needs experiment**.

## Unknown / Needs experiment

Do not fill these with guessed commands. Label in the runbook as `UNKNOWN — requires experiment` until recorded.

| Item | Status |
|---|---|
| Ubuntu 24.04 host package list to *build* fedor4ever GCC | **Unknown** |
| Exact GCC4Symbian clone commit | **Unknown** |
| Source tree (`build-toolchain.sh`) vs SourceForge GCC 14.2.0 + binutils 2.29.1 tarball | **Unknown** / **Needs experiment** |
| Wine vs native `makesis` / `signsis` / `makekeys` | **Needs experiment** |
| Full compile/link argv (crt, `-L`, `-soname` matching `--linkas`) | **Needs experiment** |

Never curl SDK or ROM URLs.
