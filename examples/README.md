# Examples

## hello

`examples/hello` is exactly what `symdev new hello --target nokia-e52` generates; a test in `symdev-cli` fails if the two drift. It is a console EXE (`econs`) that prints `Hello, world!` and waits for a key.

Build and package it on the Linux host (toolchain paths from `docs/superpowers/plans/2026-09-17-symdev-m1-build.md`). The E32 post-link step is native; set `SYMDEV_ELF2E32` only to use an external `elf2e32_next` instead:

```bash
cd examples/hello
export SYMDEV_EPOCROOT=~/sdk/S60_3rd_FP2
export SYMDEV_GXX=~/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-g++
export SYMDEV_LD=~/gcc-builds/binutils-2.29.1/bin/arm-none-symbianelf-ld
export SYMDEV_GCC_LIB=~/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0
export SYMDEV_GCC_TARGET_LIB=~/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib
export SYMDEV_SIGN_PASSWORD=...   # self-signed key password, 4+ characters
symdev build     # build/hello.exe
symdev package   # build/hello.sisx (EXE + hello_reg.rsc), self-signed
```

Run it in the patched EKA2L1 ([docs/research/eka2l1-bringup.md](../docs/research/eka2l1-bringup.md)):

```bash
export SYMDEV_EKA2L1=~/.local/bin/eka2l1   # eka2l1_qt or a wrapper that sets its env
symdev run      # installs build/hello.sisx and launches 0xef9f2cab; log in build/eka2l1.log
```

Verified 2026-09-19 in EKA2L1 (RM-469 firmware), built without `SYMDEV_ELF2E32` (experiment 47): installs, appears as `hello` (UID `0xEF9F2CAB`), shows `Hello, world!`. Emulator only — not a claim of E52 support.

`build/` is ignored by git; never commit `.sis`, `.sisx`, `.cer` or `.key`.

## gui

`examples/gui` is exactly what `symdev new gui --target nokia-e52 --template gui` generates (also kept in sync by a test): a minimal S60 3rd Edition Avkon application (`CAknApplication` / `CAknDocument` / `CAknAppUi` and one control that draws a line of text) with an application resource (`EIK_APP_INFO`, `LOCALISABLE_APP_INFO`) and a registration resource.

Same steps as `hello`. Resources are compiled with the SDK's `cpp.exe` + `rcomp.exe` under Wine (`SYMDEV_WINE`, default `/usr/bin/wine`) until a native RSS compiler exists; `LIBRARY` lines are linked as `.dso`. `symdev package` installs `gui.exe`, `\resource\apps\gui.rsc` and the registration resource.

Verified 2026-09-19 in EKA2L1 (experiment 51): title pane shows the caption, the view draws `Hello from symdev`, right softkey `Exit`. Emulator only.

