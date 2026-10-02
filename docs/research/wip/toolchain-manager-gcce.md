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
- Same g++ 12.1.0, gas 2.29.1 instead of 2.35 (`-B` wrapper): every `.o` that has a
  COMDAT group differs (gas 2.35 lists the group member's `.rel` section in `.group`,
  12 vs 8 bytes; `objdump -dr` identical), but every `.elf` and `.exe.map` is identical.

## Next step

Write build.sh (binutils 2.29.1 + gcc with the GCC4Symbian libgcov fix, sys-include,
in-tree gmp/mpfr/mpc/isl), run it into prefix-a.
