# WIP: GCCE recipe without GCC4Symbian files (experiment 108)

Task: replace the three GCC4Symbian files the gcce;12.1.0 recipe fetches (libgcov-driver.c,
sys-include/stdint.h, sys-include/stdio.h) with our own, written clean-room, and prove the
toolchain is the same as `~/gcc-builds/gcc-12.1.0` (experiment 107's comparisons).

## Clean-room rule (binding)
Never open: `~/src/GCC4Symbian/**`; any `sys-include/` in `~/gcc-builds` or
`~/src/gcce-recipe`; `~/src/gcce-recipe/{gcc4symbian,work,variants}/`; `build-*/` copies of
the three files; `include-fixed/stdio.h` of any prefix built with GCC4Symbian's stdio.h
(fixincludes' copy of it); the GCC4Symbian URLs. Allowed: official tarballs, our own
builds' logs/config.log, reference toolchain's GCC output (c++config.h, objects, archives).

## Where
- packages worktree: `~/worktrees/symdev-packages/gcce-own` (branch gcce-own)
- builds: `~/src/gcce-own/` (outside git); official GCC unpacked in `~/src/gcce-own/src/`

## Facts
- Pristine `libgcc/libgcov-driver.c` (official tarball, sha256 of tarball 62fd6348…) uses
  `(__INTPTR_TYPE__)` at lines 457 and 476 (casting a gcov_type to a pointer).
  `config.gcc`: `arm*-*-symbianelf*` adds `arm/symbian.h` but no `newlib-stdint.h` (the
  `arm*-*-eabi*` branch does), so no `__INTPTR_TYPE__` is predefined.

## Decisions

## Dead ends

## Next step
Read libstdc++/gcc configure for what stdio.h/stdint.h must provide; write headers; build.
