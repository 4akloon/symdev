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

- What GCC 12.1.0's own build needs from the two headers (GCC source, grep):
  - `stdint.h`: only `libstdc++-v3/libsupc++/new_opa.cc:28` includes it; it uses
    `uintptr_t` only in the `aligned_alloc` fallback (line 107), compiled when the target
    has no memalign — newlib crossconfig hardcodes `_GLIBCXX_HAVE_MEMALIGN 1`, so not here.
    libgcc includes `<stdint.h>`/`<stdio.h>` only under `!inhibit_libc` or for DFP targets.
    `GCC_HEADER_STDINT` (config/stdint.m4) runs only when `is_hosted` (configure.ac:396).
  - `stdio.h`: no compiled source needs it (vterminate.cc/pure.cc only `#if _GLIBCXX_HOSTED`).
    Configure: autoconf's default includes start with `#include <stdio.h>`, so every
    `AC_CHECK_HEADERS` in libstdc++ fails without it (the four `_GLIBCXX_HAVE_*_H`).
- Constraints from the reference `c++config.h` (GCC output, all four multilib copies
  identical): `_GLIBCXX_USE_C99_STDINT_TR1` undefined, so the reference stdint.h FAILS
  acinclude.m4's TR1 test (C++98 compile of every int*/fast/least/max/ptr type and their
  MIN/MAX macros); no `_GLIBCXX_USE_TMPNAM`, `HAVE_GETS`, `_GLIBCXX{98,11}_USE_C99_STDIO`,
  so the reference stdio.h declares none of tmpnam, gets, or the C99 vfscanf/vscanf/
  vsnprintf/vsscanf/snprintf set; `SIZEOF_*` all `#undef` (stdint.m4 not run: freestanding).
  `_GLIBCXX_STDIO_EOF`/`SEEK_*` are computed only when hosted.
- gcc/configure reads `$target_header_dir/stdio.h` only for `inhibit_libc`, which
  `--without-headers` already forces.
- fixincludes (`inclhack.def`): `stdio_stdarg_h` wraps every stdio.h that does not match
  `include.*(stdarg\.h|machine/ansi\.h)`; `stdio_va_list` rewrites every stdio.h that does
  not match `__gnuc_va_list|_BSD_VA_LIST_|__DJ_va_list|_G_va_list`. Reference
  `include-fixed/` lists README, limits.h, stdio.h, syslimits.h (names only; its stdio.h is
  fixincludes' copy of GCC4Symbian's file — never opened). No include-fixed/stdint.h, so
  the reference stdint.h matched no fix.
- Reference compiler predefines (`-dM -E`, no header read): `__PTRDIFF_TYPE__ int`,
  `__SIZE_TYPE__ unsigned int`, `__INTMAX_TYPE__ long long int`, no `__INT*_TYPE__`.
  GCC's `gcc/config/newlib-stdint.h`: `INTPTR_TYPE PTRDIFF_TYPE`, `UINTPTR_TYPE SIZE_TYPE`.
- Top-level configure: cross `CFLAGS_FOR_TARGET` defaults to `-g -O2` (configure.ac:2548).
- libgcov: no `__LINE__`/`__FILE__` use; `gcc_assert` has none either, so a line shift in
  libgcov-driver.c changes debug line info only.

## Decisions
- Headers: MIT (packages repo is MIT; installed into the target's sys-include, so code
  that includes them carries no copyleft condition; they contain no GCC code).

## Dead ends

## Next step
Read libstdc++/gcc configure for what stdio.h/stdint.h must provide; write headers; build.
