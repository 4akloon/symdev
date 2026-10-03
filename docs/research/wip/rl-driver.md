# RL3 — wire rust-lld into symdev's Rust build (branch `rl-driver`)

Task from the lead (2026-10-03): default Rust linker = rust-lld, `SYMDEV_RUST_LINKER=gnu`
keeps GNU ld byte-identical; prebuilt rust-sdk => no GCCE; SDK lld-fix cache; ImportStubs
second link; `--defsym=symrs_uid3`. Record as experiment 113. Scratch: `~/src/rl-driver-scratch/`.
Mid-task (lead): TARGET2 rewrite and `.strtab` fix are **public byte-level types in
`symdev-elf2e32`** so the packages publisher can reuse them; API recorded below.

## Inputs read
exp 109 (§2 seven changes, §5 no GCCE), exp 112 (§8 = spec, sizes table §4), wip/rust-shims.md
(driver section: prebuilt at `<rust-sdk>/symbian-rs/prebuilt/lib/{libsymrs.a,libsymrs_ui.a,
libsupc++.a,libgcc.a}`), v0.2 owner decisions, spec §3/§12. Spike argv
`~/src/rust-lld-spike/nogcce/ui/argv`; link2.py (`~/src/rl-scratch/stubs/`): stubs.o appended
at the END of argv, then `--wrap=f` each. Prebuilt set: `~/src/rl-shims-scratch/run1/out/
rust-sdk/symbian-rs/prebuilt/` (lib: 4172/7736/19294/59174 B).

## Design (decided)
- elf2e32 crate (pub, bytes in/out, no I/O): `Target2Rewrite::{object,archive}`,
  `StrtabPadding::zeroed`; crate-private `ElfSectionHeaders`, `ArMembers`.
  `ElfImage::jump_slots()` for the post-check (all JUMP_SLOT symbols).
- `Toolchain { epocroot, elf2e32, gcce: Option<GcceTools> }`; `Toolchain::gcce()` errors when
  absent. `GcceTools` = gxx, ld, ar, gcc_lib, gcc_target_lib (+ `ar()`).
- `RustLinker` (symdev-build): `Lld(LldLinker{rust_lld override, cache})` | `Gnu`;
  `wants_gnu(Option<&str>)` parses `SYMDEV_RUST_LINKER` value (CLI reads env);
  `needs_gcce(&sdk)` = Gnu || sdk has no prebuilt.
- `RustSdk::prebuilt()` -> `Option<RustPrebuilt>` (dir `prebuilt/` present => all 4 archives
  required, else error). `RustSdk::lld_script()` = `targets/symbian-lld.ld` (not in REQUIRED;
  missing => error at lld link naming SYMDEV_RUST_LINKER=gnu).
- `RustLld`: path from `rustc --print sysroot` + host from `rustc -vV` (pure parse);
  RustBuild runs rustc in the project root, RUSTUP_TOOLCHAIN removed. Override SYMDEV_RUST_LLD.
- `SdkLldCache` at `$SYMDEV_HOME/cache/sdk-lld/<key>/{lib,urel}`, key = sha256("v1", canonical
  epocroot, sorted (dir/name, sha256(file))) of exactly the `-l:` files the line names.
  **Lazy, at the first lld link, not at `symdev sdk install`**: a user `SYMDEV_EPOCROOT` never
  passes through `sdk install`, an SDK installed by an older symdev has no cache, and a
  content-addressed key can never be stale. Staging dir + rename (atomic; loser drops its copy).
- Link (lld): GNU-shaped line with `Linker::lld` program `[rust-lld,-flavor,gnu]` and runtime
  dirs (GCCE's, or prebuilt/lib), drop `--default-symver`, SDK lib/urel -> cache dirs, append
  `-z notext --target2=abs -Bsymbolic -T <script>` (+ `--defsym=symrs_uid3=0x..` only with
  prebuilt ui). First link -> `<name>.first.elf`, ImportStubs, `build/import_stubs.o`, second
  link (same argv, `-o <name>.elf`, + stubs.o + --wrap=f...), no JUMP_SLOT check.
- Dev checkout (no prebuilt): shims compiled with GCCE as today (incl. -DSYMRS_UID3), archive
  TARGET2-rewritten in place for lld only.

## Status
- [x] elf2e32 byte types (commit below)  - [x] Toolchain split (71b3f48)  - [x] RustPrebuilt, RustLld (d56811a), SdkLldCache  - [x] RustLinker
- [x] link line + two links (LldLine, rust_lld_link.rs)  - [x] CLI + Provision (needs_gcce, toolchain(device, gcce))  - [ ] real builds  - [ ] no-GCCE run
- [ ] emulator  - [ ] docs + exp 113  - [ ] gates

## For the packages repo
Crate `symdev-elf2e32` (path `crates/symdev-elf2e32`, deps only `symdev-core` + `symdev-uidcrc`,
`thiserror`). Bytes in, bytes out; no env, no file I/O. Errors: `symdev_core::Error`
(`Display` names the problem; archive errors are prefixed ``archive member `<name>`: ``).
```rust
use symdev_elf2e32::{StrtabPadding, Target2Rewrite};
// R_ARM_TARGET2 (41) -> R_ARM_ABS32 (2), only the type byte of each SHT_REL/SHT_RELA entry.
Target2Rewrite::object(bytes: &[u8]) -> Result<Target2Rewrite>   // ELF32 LE ARM ET_REL only
Target2Rewrite::archive(bytes: &[u8]) -> Result<Target2Rewrite>  // SysV/GNU ar; skips `/`, `//`,
                                    // `/SYM64/`; any other member must be such an object
t.rewritten() -> usize; t.bytes() -> &[u8]; t.into_bytes() -> Vec<u8>
Target2Rewrite::R_ARM_TARGET2 / R_ARM_ABS32: u8
// bytes after the last NUL of every SHT_STRTAB -> 0 (any ELF32 LE ARM file, e.g. a .dso)
StrtabPadding::zero(bytes: &[u8]) -> Result<StrtabPadding>  // a strtab with no NUL: error
p.zeroed() -> usize; p.bytes(); p.into_bytes()
```
Checked against the real SDK (`~/src/rl-driver-scratch/fixcheck`): 570 `.dso`, 428 padded,
all byte-equal to the spike's `dso-fixed/`; `usrt2_2.lib` 1 rewritten = spike's copy;
`eexe.lib` 0.

## Findings

## Dead ends

## Next step
Real builds. GNU byte-identical check first (main binary vs branch with SYMDEV_RUST_LINKER=gnu,
argv via SYMDEV_LD wrapper + .exe cmp masking CRC 0x14-0x17 / time 0x24-0x2B), then lld default
on the four apps + all examples, sizes vs exp 112.

## Real builds (running)
- Binaries: `~/src/rl-driver-scratch/bin/symdev-main` (git archive 04f8e7d → main-src, release),
  `bin/symdev-rl3` (branch f71d1d5, release). Env `env-gcce.sh` (scratch SYMDEV_HOME/XDG).
- Smoke: examples/hello rust-lld default (GCCE shims) → hello.exe **975 B** = exp 112 lld+stubs;
  hello.first.elf 79 624 = exp 109 lld; cache key dir `home/cache/sdk-lld/6178838f…`.
- Batches (background): `out/main-gnu.txt` (main bin, gnu-log, main-src tree), then
  `out/rl3-gnu.txt` → `out/rl3-lld.txt` (worktree). Per example: `out/<run>/<ex>.{log,ld.argv}`, `*.exe`.
- No-GCCE: `bin/symdev-rl3-release` (SYMDEV_RELEASE=1, target `release-target/`); file:// repo
  `stage/repo` made by `stager/` from `stage/trees/{rust-sdk (recipe include list from HEAD +
  run1 prebuilt/), sdk (copy of rl-shims-scratch installed SDK, same sha cbec6da8…), gcce
  (DECOY stub)}`. Script `nogcce.sh` → `nogcce.txt`, `nogcce/{new,build}-{hello,ui}.{log,strace}`.
