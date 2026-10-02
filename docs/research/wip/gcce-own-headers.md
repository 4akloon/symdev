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

- Our files (packages worktree, `recipes/gcce/12.1.0/`): `sys-include/stdint.h` (47 lines,
  2416 bytes: `intptr_t` = `__PTRDIFF_TYPE__`, `uintptr_t` = `__SIZE_TYPE__`, nothing else),
  `sys-include/stdio.h` (42 lines, 2227 bytes: include guard only), MIT;
  `libgcov-intptr.patch` (4 added lines `#ifndef/#define __INTPTR_TYPE__ int/#endif`+blank
  after `#define MAX` at line 413, GPL-3.0-or-later WITH GCC-exception-3.1).
  Both headers compile with the reference g++ `-nostdinc -std=c++98`; neither contains a
  fixincludes trigger (`va_list`, `stdarg.h` after include, `GNU C Library`).

## Experiment setup (outside git, `~/src/gcce-own/`)
- `dl/`: the six official tarballs (hashes OK against recipe.toml); `hostbin/make` →
  native-cc make (no wrapper env).
- `run-variant.sh <name> <recipe dir>`: builds in `run/` into `prefix/` (fixed paths),
  `env -i HOME PATH=hostbin:~/.local/bin:~/.local/native-cc/usr/bin:/usr/bin:/bin`, then
  renames to `run-<name>`, `prefix-<name>`; log `build-<name>.log`.
  Variants: `recipe-P` (patch file), `recipe-D` (no patch; gcc configure env
  `CFLAGS_FOR_TARGET="-g -O2 -D__INTPTR_TYPE__=int"`).
- `work/build-examples.sh <out> <gcc prefix> <ld>`: symdev release binary of this
  worktree; fresh copy of Cargo.toml, Cargo.lock, crates, examples, symbian-rs at
  `work/tree` per example. `work/compare-examples.py`, `work/compare-archives.py`
  (own scripts; ~/src/gcce-recipe/work was not opened).
- Baseline (`~/gcc-builds`, ld 2.29.1) twice: `.elf` identical, `.exe` differ only at
  0x14-0x17 and 0x24-0x27; sizes hello 3588/24244, gui 5061/58532, rs hello 968/16456
  (= experiment 107's table).
- Reference has 11 archives: libgcc+libgcov ×4 multilibs, libsupc++ for default, softfp,
  v5te/softfp (none for v5te), 7336 members.

- Build P, default multilib's libstdc++ (build tree, before install): `c++config.h`
  byte-identical (`cmp`) to the reference's. config.log: TR1 `<stdint.h>` no (first error
  `'int8_t' does not name a type`), tmpnam no, gets no, C99 `<stdio.h>` C++98/C++11 no;
  float.h, stdint.h, stdbool.h, stdalign.h found.
- Comparator negative test: one flipped `.text` byte in a copy of the reference
  libsupc++.a is reported (`DIFFER in array_type_info.o`).

## Dead ends

## Next step
Read libstdc++/gcc configure for what stdio.h/stdint.h must provide; write headers; build.
