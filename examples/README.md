# Examples

## hello

`examples/hello` is exactly what `symdev new hello --target nokia-e52` generates; a test in `symdev-cli` fails if the two drift. It is a console EXE (`econs`) that prints `Hello, world!` and waits for a key.

Build and package it on the Linux host (toolchain paths from `docs/superpowers/plans/2026-09-17-symdev-m1-build.md`):

```bash
cd examples/hello
export SYMDEV_EPOCROOT=~/sdk/S60_3rd_FP2
export SYMDEV_GXX=~/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-g++
export SYMDEV_LD=~/gcc-builds/binutils-2.29.1/bin/arm-none-symbianelf-ld
export SYMDEV_ELF2E32=~/src/elf2e32_next/bin/Release/elf2e32
export SYMDEV_GCC_LIB=~/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0
export SYMDEV_GCC_TARGET_LIB=~/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib
export SYMDEV_SIGN_PASSWORD=...   # self-signed key password, 4+ characters
symdev build     # build/hello.exe
symdev package   # build/hello.sisx (EXE + hello_reg.rsc), self-signed
```

Run it in the patched EKA2L1 ([docs/research/eka2l1-bringup.md](../docs/research/eka2l1-bringup.md)):

```bash
eka2l1_qt --install build/hello.sisx
eka2l1_qt --run 0xef9f2cab
```

Verified 2026-09-19 in EKA2L1 (RM-469 firmware): installs, appears as `hello` (UID `0xEF9F2CAB`), shows `Hello, world!`. Emulator only — not a claim of E52 support.

`build/` is ignored by git; never commit `.sis`, `.sisx`, `.cer` or `.key`.
