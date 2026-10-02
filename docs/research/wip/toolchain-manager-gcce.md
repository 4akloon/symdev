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

## Next step

Diff the GCC4Symbian gcc tree against the tarball; recover binutils 2.29.1 flags.
