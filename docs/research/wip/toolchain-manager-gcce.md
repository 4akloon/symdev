# WIP: GCCE recipe (toolchain manager, Track C, tasks C1–C4)

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md` Track C. Spec §6, §8 item 1.
Downloads/builds: `~/src/gcce-recipe/` (outside git). Recipes: `~/projects/symdev-packages/`.

## Facts (C1)

- `~/src/GCC4Symbian` = github.com/fedor4ever/GCC4Symbian at fe1b15a (2022-06-15), clean
  apart from untracked build markers (`build-*-started/finished`, `experiment-2-build.log`).
- Its `build-toolchain.sh` on Linux installs into `PREFIX=$HOME/gcc-builds/gcc-12.1.0`
  (matches `~/gcc-builds/gcc-12.1.0`; markers dated Sep 17 16:35–16:41). Steps:
  binutils **2.35** into the same prefix, `cp -Ru sys-include $PREFIX/$TARGET`,
  copies `build-data/GCC-12.1.0/libgcov-driver.c` over `gcc-12.1.0/libgcc/libgcov-driver.c`,
  unpacks gmp-6.1.0, mpfr-4.1.0, mpc-1.2.1, isl-0.16.1 into the gcc tree (in-tree build),
  `CFLAGS=-pipe`, gcc configure (flags = spec §6 list), `make -j6 -k -l6`,
  `make -k install-strip`, then gdb 10.2 into the same prefix.

- `~/src/GCC4Symbian/gcc-12.1.0.tar.xz` has the same SHA-256 as the official
  `ftp.gnu.org` tarball (`62fd6348…19c7b`). `diff -rq` of the official tarball against
  `~/src/GCC4Symbian/gcc-12.1.0` (minus the in-tree `gmp mpfr mpc isl`): **one file
  differs, `libgcc/libgcov-driver.c`** — 8 added lines after the includes:
  `#ifndef __INTPTR_TYPE__ / #ifndef __intptr_t_defined / #ifndef intptr_t /
  #define __INTPTR_TYPE__ int / #endif×3`. Lines 457 and 476 of that file cast through
  `__INTPTR_TYPE__`, which `arm-none-symbianelf-gcc -dM -E` does **not** predefine
  (`config.gcc`: `arm*-*-symbianelf*` adds `arm/symbian.h`, no `*-stdint.h`, so
  `use_gcc_stdint=none` and no `stdint.h` is installed either).
- `~/gcc-builds/gcc-12.1.0/arm-none-symbianelf/sys-include/{stdint.h,stdio.h}` are
  GCC4Symbian's `sys-include/` (stdio.h = `EOF`, `SEEK_SET/CUR/END` only; stdint.h =
  a GCC-generated `GCC_GENERATED_STDINT_H` "for xgcc.exe 11.2.0"). Fixincludes turned
  stdio.h into `lib/gcc/.../include-fixed/stdio.h`. symdev compiles with `-nostdinc`
  and only `-I $SYMDEV_GCC_LIB/include`, so user code never sees them.
- `build-gcc/config.status`: configure flags = spec §6 list exactly, plus env
  `CFLAGS=-pipe`. `build-gcc/Makefile`: `AS_FOR_TARGET`/`LD_FOR_TARGET`/`AR_FOR_TARGET`
  = `~/gcc-builds/gcc-12.1.0/arm-none-symbianelf/bin/*` = **binutils 2.35** (installed
  first into the same prefix). So libgcc/libsupc++ were assembled by gas 2.35, and
  `g++` still runs gas 2.35 at compile time (`arm-none-symbianelf-as --version` in the
  gcc prefix = 2.35); only the link uses ld 2.29.1 (`SYMDEV_LD`). ld 2.35 is rejected
  by SDK `euser.dso` (`.gnu.version_d`, experiment 5).
- Original gcc logs (`build-gcc/make-gcc.log`, `install-gcc.log`): no fatal error, only
  `largefile-config.h ... Error 1 (ignored)` (a `-` rule in libstdc++'s Makefile).
- **binutils 2.29.1**: `~/src/binutils-2.29.1-build/build-binutils/config.log` line 7 +
  `config.status`: `--target=arm-none-symbianelf --prefix=$HOME/gcc-builds/binutils-2.29.1
  --disable-option-checking --enable-ld --enable-gold --enable-lto --enable-vtable-verify
  --enable-werror=no --without-headers --disable-nls --disable-shared
  --disable-libquadmath --enable-plugins --enable-multilib`, env `CFLAGS='-pipe -Bstatic'`
  — i.e. GCC4Symbian's binutils step with a different version and prefix. Installed by
  `make install-strip` (`install-binutils.log`: `install-strip-target`). Source tree
  equals the official `binutils-2.29.1.tar.xz` (`diff -rq` empty). No `ld.gold`
  installed although `--enable-gold`.
- Host build tools used for both (`experiment-2-build.log` header): `~/.local/bin`
  wrappers around `~/.local/native-cc` (Ubuntu GCC 15.2.0 debs unpacked, no sudo):
  gcc, g++, make, flex, bison, m4, gawk. No makeinfo ("Makeinfo is missing").

## Measurements

- Official tarballs, GNU signatures checked with `gpgv --keyring gnu-keyring.gpg`
  (keyring from ftp.gnu.org): gcc-12.1.0.tar.xz `62fd634889f31c02b64af2c468f064b47ad1ca78411c45abe6ac4b5f8dd19c7b`
  (Good signature, Jakub Jelinek); binutils-2.29.1.tar.xz
  `e7010a46969f9d3e53b650a518663f98a5dde3c3ae21b7d71e5e6803bc36b577` (Good, Nick Clifton).
- Example build is not byte-reproducible by design: two baseline builds of the same
  tree differ in `.exe` bytes 21–24 and 37–40 only = `iHeaderCrc` and `iTimeLo`
  (`symdev-elf2e32` stamps `SystemTime::now()`); `.elf`, `.exe.map` identical — but only
  when built at the same path (rustc embeds source paths: the Rust hello's `.elf`
  differed between `work/baseline` and `work/baseline2`). Helper
  `~/src/gcce-recipe/work/build-examples.sh` builds at one fixed path `work/run`.
- With `LD_PRELOAD=work/fixtime.so FIXTIME_EPOCH=1790000000` (pins `CLOCK_REALTIME`; test
  aid outside git) two baseline builds give **identical** `.exe` files (all three).
- Same g++ 12.1.0, gas 2.29.1 instead of 2.35 (`-B` wrapper): every `.o` that has a
  COMDAT group differs (gas 2.35 lists the group member's `.rel` section in `.group`,
  12 vs 8 bytes; `objdump -dr` identical), but every `.elf` and `.exe.map` is identical.

- The original gcc build was run through `~/.local/bin/make`, a wrapper that exports
  `CPATH`, `CPLUS_INCLUDE_PATH` (host libstdc++ 15 headers), `LIBRARY_PATH`,
  `GCC_EXEC_PREFIX` to every child — including the new cross compiler building the target
  libraries: `build-gcc/arm-none-symbianelf/libstdc++-v3/config.log` has 55 hits of
  `~/.local/native-cc/usr/include/c++/15/...` (each test that reached them failed on
  `features.h`). A Debian container has no such leak; build.sh unsets these variables.
- Prerequisites: `gmp-6.1.0.tar.bz2`, `mpfr-4.1.0.tar.bz2`, `mpc-1.2.1.tar.gz` from
  ftp.gnu.org (Good signatures: Niels Möller, Vincent Lefevre, Andreas Enge) have the same
  SHA-256 as the copies on gcc.gnu.org/pub/gcc/infrastructure and in `~/src/GCC4Symbian`;
  `isl-0.16.1.tar.bz2` (gcc.gnu.org only, no signature) equals the GCC4Symbian copy.
  GCC4Symbian files at fe1b15a via raw.githubusercontent.com equal the local clone.
- GCC4Symbian has no LICENSE file; its scripts say "Attribution-NonCommercial 4.0".
  The libgcov file is GCC's own (GPL-3+ with the runtime exception) plus 8 lines; the two
  headers carry no notice. **Owner's call** before they go into a public package.

## Dead ends

- build-a, first run (env -i, PATH = clean make + `~/.local/bin` + `/usr/bin`): in-tree
  mpfr inherits GCC's `--enable-lto`, builds itself with `-flto` and checks that
  `ar` handles LTO objects; `/usr/bin/ar` has no `bfd-plugins` here, and `gcc-ar` was not
  on PATH → `configure: error: Link Time Optimisation is not supported`. The original
  passed because the make wrapper put `~/.local/native-cc/usr/bin` (ar with
  `../lib/bfd-plugins/liblto_plugin.so`) first. Rerun with that dir on PATH (after the
  `~/.local/bin` wrappers). A Debian container needs `gcc-ar` (package gcc) or
  `/usr/lib/bfd-plugins/liblto_plugin.so` — derived, not observed.

## Next step

build-a running (`~/src/gcce-recipe/build-a.log`), then compare prefix-a with
`~/gcc-builds`, relocate to `moved`, build examples.
