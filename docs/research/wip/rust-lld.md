# rust-lld productisation (branch `rl-elf2e32`)

Task: (A) elf2e32 accepts lld ELFs (5 rules of experiment 109 §3, gated on lld detection,
labelled; GNU goldens unchanged; lld ELF hex golden). (B) 8-byte import stubs with lld
(`ImportStubs` type), proven on hello/async/shim/ui vs GNU sizes + EKA2L1; experiment 112.

Worktree `/home/genius/worktrees/symdev/rl-elf2e32`; scratch `~/src/rl-scratch/`.
Spike: `~/src/rust-lld-spike/` (fork `elf2e32-fork/`, grep `SPIKE 109`).
Do NOT edit: `crates/symdev-build/src/driver/{rust_build,libcalls}.rs`, `crates/symdev-sdk`,
`symbian-rs/shims`, packages repo.

## State
- [x] A: read spike fork + product elf2e32
- [x] A: lld detection rule (`ElfLinker`, `elf/linker.rs`)
- [x] A: five rules + tests (`elf/lld.rs`, `elf2e32/tests/experiment_109.rs`)
- [x] A: lld golden (`testdata/hello_lld*.hex`, `hello_lld_ordinals.txt`)
- [x] B: lld options investigation (dead ends below)
- [x] B: stub mechanism + ImportStubs type (`src/import_stubs.rs`, `import_stubs/object.rs`,
      example driver `examples/import_stubs.rs`)
- [ ] B: four apps sizes / elf2e32 / EKA2L1
- [ ] experiment 112 entry

## Findings
- Detection: `.comment` holds a NUL-separated string starting `Linker: LLD ` (lld writes
  `Linker: LLD 23.1.1 (…)` into every non-relocatable output). GNU ld 2.29.1 writes no linker
  string (GNU ELFs carry only `GCC: (GNU) 12.1.0`, `rustc version …`, SDK `ARM Linker,
  RVCT2.2 …`). Checked: `rust-lld -r` writes NO `Linker: LLD` (so an lld `-r` object linked
  by GNU is not misdetected). A missing/stripped `.comment` fails safe: refused as before.
  Section names now parsed (`sh_name`, `e_shstrndx`); unreadable names match nothing.
- Rules live in `elf/lld.rs` (each labelled), call sites: relocs.rs (a, d), layout.rs (b, c),
  reloc_section.rs (e), code_section.rs (a: `ElfImportReloc::addend_in_place`).
- Rule (b) data base = the `.data` output section's address (lld keeps the empty section
  header at -Tdata); no `.data` → error naming the fix.
- Golden: spike `nogcce/hello` argv relinked with `-z max-page-size=0x1000` → 22 KB ELF
  (`~/src/rl-scratch/golden/`); fork E32 of it == spike's nogcce/hello.exe (masked). Hex
  fixtures: hello_lld.elf.hex, hello_lld.exe.hex, hello_lld_uncompressed.exe.hex.
- Product elf2e32 (this branch) on spike ELFs: 4 apps GNU → identical to main's output,
  4 apps lld → identical to fork output; 15 batch examples GNU+lld → identical (net only
  differs by the capability my script omitted; rerun with NetworkServices: identical).
  Scripts: `~/src/rl-scratch/cmp/cmp4.sh`, `cmp15.sh` (arg: elf2e32 binary).
- Built bins in `CARGO_TARGET_DIR=~/src/rl-scratch/target`.

- Part B mechanism: two-pass link. First lld link (as spike) → `ImportStubs::from_first_link`
  reads `R_ARM_JUMP_SLOT` undefined symbols (= imported functions called via PLT) → object
  written by a tiny Rust ELF REL writer (`ImportStubs::object`; no assembler → no GCCE; chosen
  over `rustc --emit=obj` of `global_asm!`, which needs a rustc run per build with the
  target's `core`, slower and indirect) → second link with `stubs.o` + `--wrap=<f>` each.
  Second link has no JUMP_SLOT, no .plt/.got.plt. objdump (binutils 2.29.1) disassembles the
  object correctly (`ldr pc, [pc, #-4]` / `.word` + R_ARM_ABS32 __real_f).
- Driver: `~/src/rl-scratch/stubs/link2.py <proj> [--place=end|start|after-rust] [--sandbox]`
  (spike nogcce argv; outputs in `~/src/rl-scratch/stubs/<proj>/`).
- First sizes (place=end, .exe GNU | stubs | lld PLT): hello 968|975|1044, async
  18431|18360|18577, shim 4452|4464|4572, ui 10315|10288|10511. Stubs = GNU PLT count minus
  GNU's PLT entries for the image's own globals (async 69−61=8, ui 80−71=9): with -Bsymbolic
  lld calls those directly → async/ui smaller than GNU. hello +8 uncompressed = one exidx
  CANTUNWIND entry lld writes for its `__ARMv4PILongBXThunk_RunThread` (GNU's veneer has none);
  code size equal.

- Step 1 (resume 3): `cargo test -p symdev-elf2e32 --offline` → 60 passed, incl. experiment_109
  lld golden (`experiment_109_lld_hello_matches_the_image_that_ran_in_eka2l1`, hex fixtures).
- Step 2, the exidx difference — CORRECTION of the earlier "thunk entry" reading: GNU also
  has a 16-B veneer for `_E32Startup`'s `bls RunThread` (`.emb_text.__stub` 0x10 in
  gnu/hello/hello.exe.map), so the thunk itself costs the same. The extra lld entry sits at
  the END of `__cpp_initialize__aeabi_` (0x82e8+0x48 = 0x8330), i.e. it is lld's
  terminating sentinel; the thunk merely starts there. Rerun 2026-10-03 (link2.py, place=end):
  exidx entries GNU|stubs: hello 7|8, async 27|30, shim 18|21, ui 38|42. shim diff: lld
  keeps `_Unwind_GetLanguageSpecificData` and `_Unwind_GetDataRelBase` (same inline
  0x80a8b0b0 as `_Unwind_GetRegionStart`) — GNU merges identical adjacent entries inside one
  input `.ARM.exidx`, lld only drops whole duplicate input sections — plus the sentinel.
  Sizes unchanged from the first table (hello 968|975|1044 …; uncompressed hello 1340|1348,
  shim 7092|7112).
- Step 2 attempts (scripts `~/src/rl-scratch/exidx/`: `relink.py <proj> <stem> [lld args]`
  reruns the stubs second link + post-link; `unwound-first.sh <elf>`; README):
  1. Synthetic proof, no thunk (`a.s`: f1 inline unwind, f2 `.cantunwind`): rust-lld writes 3
     entries (sentinel CANTUNWIND at f2+4), GNU ld 2.29.1 writes 2. The sentinel is
     unconditional in lld, even after a CANTUNWIND entry.
  2. `--no-merge-exidx-entries`: worse (hello 8→12 entries, 1348→1380 B uncompressed; shim
     21→32). lld has no other exidx option (`--help`: only merge/no-merge).
  3. `--symbol-ordering-file` with `_E32Startup` (to let `__cpp_initialize__aeabi_`'s
     CANTUNWIND merge into the stubs' run): no effect — lld does not reorder the SDK's
     `.emb_text` input sections (ordering a `.text` symbol, `RunThread`, does work).
  4. `--symbol-ordering-file` = every function with a real unwind entry first (from the
     link's own exidx, `unwound-first.sh`): the CANTUNWIND runs then merge, compensating
     the sentinel. GNU | stubs | stubs+order (.exe / uncompressed / exidx entries):
     hello 968/1340/7 | 975/1348/8 | 969/1340/7; async 18431/33948/27 | 18360/33892/30 |
     18388/33864/26; shim 4452/7092/18 | 4464/7112/21 | 4455/7096/19; ui 10315/17092/38 |
     10288/17028/42 | 10267/16980/36. Uncompressed always smaller, compressed mixed (async
     +28 vs default order). Decision: NOT adopted — it moves code away from link order for
     a few bytes and needs its own EKA2L1 proof; documented in exp 112 as an optional knob.
  Conclusion: the one known difference is lld's terminating exidx sentinel (+8 B
  uncompressed) plus identical adjacent entries inside one input `.ARM.exidx` that lld
  does not merge (libgcc unwinder in shim/async/ui); no lld option removes either.
- Step 3 setup: the spike work dirs cannot be rebuilt (their Cargo.toml names the deleted
  spike worktree's symbian-rs; this branch's symdev refuses the absolute path). `symdev
  package` does not rebuild, it packages build/<name>.exe; e2e.sh output == symdev's own
  GNU .exe for all four (only 0x14-0x17, 0x24-0x27 = time/CRC differ). So: copy
  `stubs/<proj>/final.exe` → `work/<proj>/build/<name>.exe`, `$S package` (S = this
  branch's symdev, `~/src/rl-scratch/target/debug/symdev`), then
  `flock -w 900 ~/.local/share/EKA2L1/.symdev-agent.lock python3 ~/src/rl-scratch/runshot.py
  <work/proj> stubs-<proj> '<regex>' [delay] [keys]` (shots in `~/src/rl-scratch/shots/`).
  `~/src/rl-scratch/stubs/ld-stubs` (a SYMDEV_LD wrapper) is unused for this reason.
- Step 3 emulator (2026-10-03, this branch's symdev package + runshot, installed exe on E:
  `cmp`-equal to `stubs/<proj>/final.exe` each time):
  hello: `Trying to display: Hello from Rust SDK (19 chars)`; shot 0 px from gnu-hello-1.
  shim (leave probe): `lld109 mkdirall=0 trapped=-1 bad=0 ensured=0 sign=-42 alive`; 0 px.
  ui: "Bars", `bars=3 keys=0 cmd=0`; F1 F1 → `bars=4 keys=0 cmd=1`; vs gnu-ui-cmd-{1,2}:
  84 px each, all in bbox (527,157)-(554,165) = the status-pane clock. Shots:
  `~/src/rl-scratch/shots/stubs-{hello,shim}-1.png`, `stubs-ui-cmd-{1,2}.png`.
  async: `symdev test --emulator` → `asyncdemo: 15 passed`, exit 0 (300 ms sleep 328 ms,
  together 312, sequence 625, race 109); log `~/src/rl-scratch/stubs/async-test.log`.
## Dead ends
- lld options for an 8-byte PLT: none. `.plt`/`.got` stay 0x120/0x4c on hello with each of
  `-z now`, `-z lazy`, `--pic-veneer`, `-z noseparate-code`, `--no-rosegment`,
  `--hash-style=sysv`; `--help` lists no ARM PLT option (only `--target2`, `--wrap`,
  `--pic-veneer`, `--nmagic`). lld's ARM PLT entry is fixed 16 B (12 B code + d4d4d4d4 trap
  padding) + 4 B `.got.plt` slot, header 32 B + 3 reserved GOT words.

## Next step (resume 3, 2026-10-03)
Steps 1-2 DONE. Steps: (1) confirm lld golden test + all goldens pass (`cargo test -p symdev-elf2e32 --offline`,
CARGO_TARGET_DIR=~/src/rl-scratch/target); (2) hello +7 B exidx thunk: try link order / lld
options, record each; (3) prove hello/async/leave probe/ui with stubs in EKA2L1 + bwrap no-GCCE
+ 15 examples; (4) rebase on main, experiment 112 in backlog; (5) gates.
