# Rust shims prebuilt (experiment 109 → product, track (a))

Branches: symdev `rl-shims` (~/worktrees/symdev/rl-shims), symdev-packages `rl-shims`
(~/worktrees/symdev-packages/rl-shims). No push, no merge. Scratch: ~/src/rl-shims-scratch/.
Spike evidence: ~/src/rust-lld-spike/ (experiment 109 §1, §5).

## Task
1. symdev: `symrs_avkon.cpp` reads UID3 from symbol `symrs_uid3` (`--defsym`), compile-time
   fallback keeps today's GNU build byte-identical (examples/ui .exe before/after, mask CRC/time).
   Driver files (`crates/symdev-build/src/driver/*`) are NOT mine: record needed change here.
2. packages: `recipes/symdev/<ver>/build.sh` adds `prebuilt/` to `rust-sdk`: libsymrs.a,
   libsymrs_ui.a, 4 GCC runtime members as small archives, TARGET2→ABS32 rewrite (tested tool),
   closure check. SPDX `MIT AND GPL-3.0-or-later WITH GCC-exception-3.1`; notice names the gcce
   source archive. Target ~90 KB raw (spike 90 592 B).
3. Verify: build prebuilt set locally, link the spike's 4 apps with rust-lld using ONLY it
   (bwrap, no GCCE visible), post-link with spike's elf2e32 fork, `.exe` identical to spike lld.

## Log
- 2026-10-03 start. Read backlog §109 and v0.2 owner decisions.
- Scratch SYMDEV_HOME `~/src/rl-shims-scratch/home`: published `gcce;12.1.0` and
  `sdk;s60-3rd-fp2;1.1` installed there with main's symdev (keys from keys.env), 3m50s.
- **Published GCCE ≠ spike's GCCE for the assembler**: spike used `~/gcc-builds/gcc-12.1.0`
  whose `as` is GCC4Symbian's binutils **2.35**; the published `gcce;12.1.0` has `as` 2.29.1.
  Shim objects differ in layout (2.35 puts `.rel.rodata._ZTI15XLeaveException` in the COMDAT
  group, 2.29.1 does not: 4 bytes), code identical; the GNU `uidemo.exe` is the same size and
  differs only at CRC/time. So the archive byte counts will not equal the spike's 90 592 B to
  the byte with the published GCCE; the `.exe` identity is the criterion.
- The 4 runtime members from the published GCCE, `objcopy --strip-debug`, are byte-identical
  to the spike's `prebuilt/rt/s-*.o` (raw `eh_personality.o`, `del_ops.o` identical too).
- **Step 1 done (symdev).** `symrs_avkon.cpp`: `#ifdef SYMRS_UID3` → `SymRsAppUid()` returns
  the define; else `extern "C" char symrs_uid3[]` and returns its address. RED: without the
  define the old source stops at its `#error`. GREEN: compiles, `U symrs_uid3`, AppDllUid is
  `ldr r0,[pc]; bx lr; .word 0 (R_ARM_ABS32 symrs_uid3)`; disassembly identical to the
  spike's `prebuilt/obj/symrs_avkon.o`. **examples/ui before/after (driver unchanged, still
  passes -DSYMRS_UID3):** all 10 `build/shims/*.o` byte-identical (avkon included),
  `uidemo.elf` identical, `uidemo.exe` 10 315 B both, differs only at 0x14–0x17 (CRC) and
  0x24–0x27 (time). Evidence `~/src/rl-shims-scratch/ui-{before,after}/`.
- **GNU ld + `--defsym` does NOT work today** (tested, so the driver must keep `-DSYMRS_UID3`
  on the GNU path): `examples/ui` built by symdev with a logging `SYMDEV_LD` wrapper
  (`~/src/rl-shims-scratch/gnu-defsym/ld-wrap.sh`) that swaps in an archive with the UID-free
  avkon object and appends `--defsym=symrs_uid3=0xe0000687`: GNU ld 2.29.1 links, but writes a
  dynamic `R_ARM_ABS32` at 0x9de0 against `symrs_uid3` (exported to `.dynsym` GLOBAL, ABS),
  and symdev's elf2e32 refuses: "relocation at 0x9de0 targets 0xe0000687 outside code and
  data". Declaring it `__attribute__((visibility("hidden")))` changes nothing that matters:
  symbianelf copies it into `.dynsym` as LOCAL ABS and still emits the `R_ARM_ABS32`, same
  error. lld with `-Bsymbolic` resolves it statically (spike §5). So: `--defsym` is an lld-only
  mechanism; the compile-time define stays the GNU path. Kept the spike's plain declaration.
- **packages: `tools/target2_abs32.py` + `tests/target2_abs32_test.py`** (13 tests, synthetic
  ELF32 objects; refuses non-ELF/ELF64/BE/non-ARM/non-REL/bad entsize/out of bounds). On the
  10 real shim objects: common 3 (active, f32, leave), UI 7 (avkon 1, list 3, note 1, query 2)
  = the spike's counts; output byte-identical to the spike's `fix-target2.py` on all 10.
- **packages: `tools/runtime_closure.py`** (12 tests; parses the GNU ld map's "Archive member
  included" section, fails on a needed-but-not-shipped or shipped-but-unneeded member;
  checked on the real `uidemo.exe.map`) and **`tools/sdk_casefold.py`** (6 tests; the rule of
  symdev's `SdkIncludeCaseFold`; on the published SDK it writes the same 260 links as symdev's
  `build/sdk-include-casefold`, `find -printf '%p -> %l'` identical).
- Next: `recipes/symdev/0.2.0/` (recipe.toml copy of 0.1.0 + prebuilt.sh), workflow step.
- **packages: `recipes/symdev/0.2.0/prebuilt.sh <symdev-src> <gcce> <epocroot> <out-dir>`**
  (committed): drift guard on the tag's `RustBuild::SHIM_OPTIONS`; casefold overlay; the
  10 shims with the `[ui]` argv minus `-DSYMRS_UID3`; check `U symrs_uid3` in avkon; TARGET2
  rewrite; `ar crD`; runtime members `ar x` + `objcopy --strip-debug` + `ar crD`; closure
  link (GNU ld, shims `--whole-archive`, 17 SDK DSOs, full libsupc++/libgcc) → map →
  `runtime_closure.py` exact; second link against `lib/` only → `nm -D --undefined-only`
  must equal the first; NOTICE + COPYING3 + COPYING.RUNTIME (`tools/notices/gcc-12.1.0/`,
  from the pinned gcc-12.1.0.tar.xz, sha256 62fd6348…).
- **Run 1, published gcce;12.1.0 + published SDK** (`~/src/rl-shims-scratch/run1/`), 2.2 s:
  libsymrs.a 19 294, libsymrs_ui.a 59 174, libsupc++.a 7 736, libgcc.a 4 172 = **90 376 B**.
  All 10 objects byte-identical to symdev's own per-application objects of the `examples/ui`
  build (TARGET2-rewritten the same way; avkon = the no-define compile) → the recipe's argv
  is symdev's. Runtime archives byte-identical to the spike's. Closure: del_ops,
  eh_personality ← symrs_active; _thumb1_case_uqi ← symrs_note; pr-support ← eh_personality.
- **Run 2, the spike's toolchain** (symlink prefix `~/src/rl-shims-scratch/spike-gcce`:
  g++ of `~/gcc-builds/gcc-12.1.0` with its as 2.35, ar/ld/nm/objcopy of binutils 2.29.1)
  + `~/sdk/S60_3rd_FP2`: **90 592 B, all four archives byte-identical to the spike's
  `prebuilt/lib`** — the spike's numbers reproduced exactly.
- **Step 3 done: link with only the prebuilt set, no GCCE.** `~/src/rl-shims-scratch/verify/
  nogcce-link.py` (the spike's script, PREBUILT = run1's lib/ from the published GCCE):
  rust-lld in `bwrap --dev-bind / / --tmpfs ~/gcc-builds --tmpfs <scratch home>/gcce
  --tmpfs <spike-gcce>` under `strace -f -e trace=%file`; inside, both GCCE dirs are empty and
  no `arm-none-symbianelf-*` on PATH. **All four link, 0 strace lines touch a GCCE path**;
  files opened (ui): the 4 prebuilt archives, the spike's `dso-fixed/*.dso` and
  `sdk-fixed/urel/{eexe,usrt2_2}.lib`, `symbian-lld.ld`, the two Rust archives, LLVM.
  The ELFs are **byte-identical to the spike's `nogcce/*/*.elf`**. Post-linked by the fork
  (`e2e.sh`), the `.exe` vs the spike's `lld/*/*.exe` and `nogcce/*/*.exe`: hello 1 044,
  async 18 577, shim 4 572, ui 10 511 B — **no differing byte outside CRC 0x14–0x17 and time
  0x24–0x2B**. So the emulator results of experiment 109 §4 hold for them byte for byte.
- Negative runs of prebuilt.sh (`~/src/rl-shims-scratch/neg/`): member list without
  `_thumb1_case_uqi.o` → exit 1; with an extra `eh_terminate.o` → exit 1; tag's
  SHIM_OPTIONS `-O1` → exit 1 naming both lists; the old avkon shim (main 25053a2) → its
  `#error`, exit 1.
- Next: recipe.toml 0.2.0 (+ build.sh copy), symdev.yml install + prebuilt steps, README;
  gates in both repos.

## Resume 2 (2026-10-03, after the 0.2.0 release)
- symdev 0.2.0 released (tag v0.2.0); the prebuilt set ships with the NEXT release, so the
  packages recipe is `recipes/symdev/0.3.0` (tag v0.3.0 does not exist yet).
- symdev `rl-shims` rebased on main 0a426e6: clean (main touched driver/rust_build.rs
  `prepare`, not SHIM_OPTIONS or the shims). packages `rl-shims` was already on main 149c67d.
- packages f640113: `recipes/symdev/0.3.0/{recipe.toml,build.sh,prebuilt.sh}` committed
  (prebuilt.sh moved from 0.2.0; recipe ids 0.3.0, tag v0.3.0, rust-sdk licence
  `MIT AND GPL-3.0-or-later WITH GCC-exception-3.1`, include + `symbian-rs/prebuilt`).
- CI facts: symdev reads `$XDG_CONFIG_HOME/symdev/sources.toml`, keys from
  `SYMDEV_SOURCE_PRIVATE_{ACCESS_KEY_ID,SECRET_ACCESS_KEY}` (source name `private`);
  packages has the reader key at repo level under those names and variable
  `SYMDEV_PRIVATE_SOURCE_URL` (v0.2.md "G2 landed"). Packages live in
  `$SYMDEV_HOME/gcce/12.1.0`, `$SYMDEV_HOME/sdk/s60-3rd-fp2/1.1`; downloads in
  `$XDG_CACHE_HOME/symdev/downloads`.
- Next: build.sh guard (tag must not track symbian-rs/prebuilt); publish test for the real
  0.3.0 recipe (packs prebuilt/, refuses a tree without it); symdev.yml steps; README; gates.
- packages 16b2322: publish tests on the real 0.3.0 recipe (`publish/src/recipe/tests/
  prebuilt.rs`: composite licence; packs prebuilt/ = 7 files; refused without prebuilt/,
  "matches nothing"; mutation: dropping the include line fails 2 of 3); build.sh guard:
  the tag must not track `symbian-rs/prebuilt` (checked on fake tags v9/v10, v0.2.0 clean).
- **Rehearsal of the 0.3.0 release, local** (`~/src/rl-shims-scratch/rehearse/`):
  `symdev-src` = clone of rl-shims + commit "workspace version 0.3.0" + tag v0.3.0 (scratch
  clone ONLY; the real repo has no v0.3.0), `packages` = git archive of packages HEAD with
  the recipe's git = file:// of that clone. build.sh running in background, PID 3684169,
  log `rehearse/build.log`, `CC_x86_64_unknown_linux_musl=gcc` (no musl-gcc on this host),
  `CARGO_TARGET_DIR=rehearse/target`. Then: sources.toml (private + key = "builtin") under
  rehearse/config, `out/symdev/bin/symdev sdk install` into rehearse/home with the
  keys.env reader key, prebuilt.sh from rehearse/work, pack dry run into
  rehearse/artifact/packed, epoc32 check of the artifact.
- **Rehearsal results.** build.sh (local tag v0.3.0 in the scratch clone): static-pie
  symdev, out/{symdev,rust-sdk,symdev-0.3.0-source.tar.gz}. The just-built symdev with
  `sources.toml` = private source (`auth = "s3"`, `key = "builtin"`, dummy URL, no keys)
  installed `gcce;12.1.0` (67.5 MB) from public into a fresh SYMDEV_HOME: it reads
  `key = "builtin"` and checks the signed public index. **Not rehearsed by me: the SDK
  install from the private bucket** — the auto-mode classifier refused reading the owner's
  sources.toml/keys.env (credential materialisation); prebuilt.sh used the SDK the previous
  run installed in `~/src/rl-shims-scratch/home/sdk/s60-3rd-fp2/1.1`. CI's first run (or
  the lead, with the key) covers it. prebuilt.sh with that GCCE: 2.1 s, 90 376 B, the four
  archives byte-identical to run1 (which the no-GCCE link verified). Pack dry run (cwd
  rehearse/artifact/packed, PUBLISH_PUBLIC_URL = public r2.dev): rust-sdk;0.3.0 417 468 B
  (sha e121a7ca…) with symbian-rs/prebuilt/{COPYING.RUNTIME,COPYING3,NOTICE,lib/4×.a},
  symdev;0.3.0 3.2 MB; index shows licence `MIT AND GPL-3.0-or-later WITH
  GCC-exception-3.1`. gcce's source-code entry: src/gcce/12.1.0/1af72bd0….tar.gz.
- packages c01f8e7: `tools/sdk_free.py <sdk-dir> <path>...` (9 tests): fails on any file,
  nested tar/tar.gz members included, with the bytes of an SDK file, or a path through
  epoc32/. Rehearsal artifact (symdev-out.tar + packed/*.tar.gz): pass against 2 411
  distinct SDK files; positive control (e32std.h renamed in a tar.gz in a tar): exit 1.
- Next: symdev.yml (toolchain under $RUNNER_TEMP/toolchain, install step only with the
  reader key, prebuilt step, pack into $RUNNER_TEMP/artifact/packed, sdk_free.py check,
  upload `${{ runner.temp }}/artifact/`), tests.yml (recipes path, step name), README.
