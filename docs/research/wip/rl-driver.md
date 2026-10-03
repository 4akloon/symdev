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
- **No-GCCE result** (nogcce.sh + nogcce-ui.sh after a script bug: duplicate symbian-core dep):
  `symdev new` installed rust-sdk;0.2.0 from local, `symdev build` installed sdk only;
  `$SYMDEV_HOME` = {cache, rust-sdk, sdk}, no gcce (decoy never run); strace: 0 lines with
  gcc-builds or arm-none-symbianelf; symdev exec'd nightly rust-lld 2× per build (+3 host
  links by cargo). nohello.exe 975 B (= exp 112), noui.exe 10 291 B (exp 112 ui 10 288; other
  name/UID). Cache: 2 dirs (console set, GUI set).
- GNU identity so far: argv identical for every example (58/65 args); exes equal (masked) except
  netdemo (10 499 main / 10 506 branch): its Rust archive differs between the two trees
  (rustc output depends on the source path) — recheck with main binary in the worktree.
- **Batches done** (`out/{main-gnu,rl3-gnu,rl3-lld}.txt`): all 21 examples rc=0 in all three.
  GNU on request: ld argv identical 21/21 (58 args console, 65 GUI); exe equal (masked) 17/21;
  net, tls, std-hello, std-net differ — their Rust archives differ between main-src and the
  worktree (path-dependent rustc output). Relink with main binary + SYMDEV_RUST_SDK=worktree
  running (`out/main-gnu-wt.txt`, mode gnu-sdk).
- **rust-lld default sizes** (`sizes.py`): 17/18 of exp 112's table equal both columns; scaffold
  hello 975/1348 and exp-109 leave probe (apps/probe) 4464/7112 = exp 112 too. **notes +9/+8**:
  one more `.ARM.exidx` entry, `__gnu_thumb1_case_uqi` CANTUNWIND — full GCCE libgcc.a puts
  `_thumb1_case_uqi.o` before `pr-support.o`, so its entry follows `__gxx_personality_v0`'s and
  cannot merge; exp 112's prebuilt libgcc.a puts it after. Checking with the prebuilt route
  (`out/pre.txt`: mode pre = staged rust-sdk tree + SYMDEV_RUST_SDK, GCCE vars unset).
- Mistake caught: `git add -A` swept build-rewritten std-hello/std-net Cargo.lock into 2 commits;
  restored (main's locks are stale: symdev-locale 0.1.0; chip suggested). Stage by name now.
- Lesson: `pkill -f` pattern matched my own shell; use `ps | grep -F` and kill exact PIDs.
- **Emulator** (`~/src/rl-driver-scratch/emu.sh` = symdev package + runshot.py under the lock).
  Pitfall: exporting a scratch XDG_DATA_HOME makes EKA2L1 look for its data under it ("Devices
  file not found") — emu.sh now unsets XDG_*. No-GCCE hello: `Trying to display: Hello from Rust
  SDK (19 chars)`; no-GCCE GUI (prebuilt shim + defsym): "Bars", bars=3 keys=0 cmd=0 → F1 F1 →
  bars=4 keys=0 cmd=1 (`shots/nogcce-ui-{1,2}.png`). PIDs killed, none left.
- GNU relink with main binary + SYMDEV_RUST_SDK=worktree (`out/main-gnu-wt`): net, tls, std-hello,
  std-net exe equal (masked) and argv equal ⇒ **GNU on request byte-identical 21/21**.
- Prebuilt route (`out/pre`, staged tree, no GCCE vars): uncompressed = exp 112 for all 18
  (notes included ⇒ libgcc member order explains dev-route notes +8); compressed net +2, tls −18
  (tree path changes Rust code bytes, as in the GNU comparison).
- Emulator, dev route (rust-lld + GCCE shims): examples/ui = no-GCCE screenshots but 44 px clock;
  examples/shim `trapped=-12 … alive`; apps/probe (exp 109 probe) `trapped=-1 … alive`;
  `symdev test --emulator` examples/async **15 passed**. No-GCCE leave probe (`nogcce-probe.sh`):
  4464/7112, `trapped=-1 … alive`. Every PID killed by runshot / exited.
## CHECKPOINT 2026-10-03 (usage limit; lead asked to stop) — resume here
HEAD = cf127b8 (+ this notes commit). Tree clean except build-rewritten
`symbian-rs/examples/std-{hello,net}/Cargo.lock` — ALWAYS `git checkout --` them, stage by path
(`git add crates docs …`), never `git add -A`. No emulator or background job of mine running.

**Done:** all code (sections above); README "Linking Rust programs", examples README, spec §4/§12;
experiment 113 §1–§2 in experiment-backlog.md (commit ad1309a). Real evidence complete:
GNU identity 21/21, rust-lld sizes (table generated by the python in this session — regenerate
with `~/src/rl-driver-scratch/sizes.py rl3-lld` / `sizes.py pre`; full markdown table printer
was an inline script: GNU=out/rl3-gnu, checkout=out/rl3-lld, prebuilt=out/pre, exp112 dict in
sizes.py), no-GCCE builds + emulator runs (all recorded above). std-net checkout vs prebuilt
+8 B = `.rodata` path strings (0x3de0 vs 0x3de8), same 24 exidx entries.

**Code review (agent) — fixed:** #1 GNU ignores prebuilt/ (`RustLinker::prebuilt(&RustSdk)`,
`needs_gcce(&RustSdk)`), #2 no-HOME error names the cache + gnu, #3 lld_script checked before
cargo + message without a version, #4 `LldLine::adapt`/`second_link` return Result (3c5bad1,
cf127b8). **Still to do (review items, all Minor):**
- #5 DONE (commit after cf127b8; tests driver/tests/rust_lld_link.rs: stub rust-lld + golden ELFs)
- #6 DONE (90fcf00), #7+#8 DONE (ArMember/ElfSectionHeader/TestSection own files, checked_add,
  `//` long names; all 10 SDK urel .lib read: TARGET2 in usrt2_2 1, libcrt0 1, exiflib 33)
- #9, #11, #12 DONE (see git log). All review items closed. Exp 113 §3–§5 + conclusion written (5049161). NEXT: gates, then
  fresh whole-branch review (base 04f8e7d), fixes, then REBUILD final binary and re-check a few
  images (masked equal) — exp 113 Evidence promises 'a re-check note below'; then gates again.
- (was #5) `rust_lld_link.rs`: run the `jump_slots()` check on the final ELF in BOTH paths (also the
  rename path); wrap rust-lld errors with "first/second rust-lld link" + gnu way out; add the
  ELF path to the `ImportStubs::from_first_link` error; `io` helper calls in rust_lld_link.rs
  (rename/write) and rust_shims.rs (`shim_archives` read/write) must name the path.
- #6 `SdkLldCache`: DIRS order `["urel", "lib"]` (the line searches urel first); if the key
  dir exists but a named file is missing → remove_dir_all + remake; (leave stale `.staging-*`,
  mention).
- #7 `checked_add` in `elf_section_headers.rs` (`shoff + shnum*SHDR`) and `ar_members.rs`
  (`start + size`); add tests: bad shentsize, extended numbering, bad ar size/terminator.
- #8 one type per file: move `ArMember` → `ar_member.rs`, `ElfSectionHeader` →
  `elf_section_header.rs`, `TestSection` → `test_section.rs`; optionally resolve `/123`
  long names via `//` in error messages.
- #9 delete unused `SdkLldCopy::dir()`; `SdkLldCopy` + `SdkLldCache::ensure` → pub(crate)
  (drop `pub use sdk_lld_copy::SdkLldCopy` from lib.rs); `RustBuild::link_line` → pub(super).
- #11 `Provision::needed` doc: says it installs the Rust SDK for a Rust project;
  `Toolchain::resolve`: resolve epocroot first (`let base = Self::without_gcce(o, sdk)?;`) so the
  EPOCROOT error comes before GCCE's, as before.
- #12 `.gitignore`: add `/symbian-rs/prebuilt/`; drop the `prebuilt() == None` assert from
  `rust_sdk/tests.rs::the_checkout_ships_the_lld_script_and_no_prebuilt_set` (rename test).
- Optional test: `link_lld` with a stub rust-lld script that copies
  `symdev-elf2e32/src/testdata/hello_lld.elf.hex` (unhex) to its `-o` → second link still has
  JUMP_SLOTs → error names them; fake SDK from `sdk_lld_cache/fixture.rs` `dso()` for every
  `-l:` name.

**Then:** finish experiment 113 (append §3 sizes table, §4 no GCCE, §5 EKA2L1, conclusion,
evidence — all numbers are in the findings above; "release order" note: lld default needs
the 0.3.0 bump in the same release, rust-sdk 0.2.0 has no script/prebuilt); gates:
`cargo test --workspace --offline`, `cargo clippy --workspace --all-targets --offline`
(0 warnings), `cargo fmt --all --check`; every .rs ≤ 300 lines; superpowers:verification-
before-completion; report to the lead (commits, behaviour, sizes table, no-GCCE evidence,
emulator evidence, experiment 113, the packages-repo API section above).
- Gates after review items + exp 113 (HEAD at this commit's parent): `cargo test --workspace
  --offline` 869 passed 0 failed; clippy 0 warnings; fmt clean; all changed .rs ≤ 300 lines.
  Logs: ~/src/rl-driver-scratch/gate-{test,clippy}.log. NEXT: superpowers:requesting-code-review
  base 04f8e7d.
- Running (2026-10-03): whole-branch review agent (base 04f8e7d, head b8fbedd); re-check job
  `~/src/rl-driver-scratch/recheck.sh` PID 4066899, log `recheck.txt` (final binary at HEAD,
  relinks hello/async/ui/notes/shim lld, ui/notes prebuilt, hello/ui gnu; masked cmp vs exp 113).
- Re-check done: 9/9 images equal (masked) with the binary at b8fbedd (exp 113 note, d6e74eb).
  Branch merges cleanly onto main ffa090b (`git merge-tree --write-tree main HEAD`).
- HANDBACK at d6e74eb+notes: the whole-branch review (base 04f8e7d, head b8fbedd) was still
  running when the report was due. NEXT: get its findings (re-run superpowers:requesting-code-review
  base 04f8e7d if lost), fix Critical/Important, re-run `cargo test --workspace --offline` and
  `cargo clippy --workspace --all-targets --offline`, and redo `~/src/rl-driver-scratch/recheck.sh`
  if a fix touches the link path.
