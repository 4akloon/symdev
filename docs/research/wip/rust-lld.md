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

## Dead ends
- lld options for an 8-byte PLT: none. `.plt`/`.got` stay 0x120/0x4c on hello with each of
  `-z now`, `-z lazy`, `--pic-veneer`, `-z noseparate-code`, `--no-rosegment`,
  `--hash-style=sysv`; `--help` lists no ARM PLT option (only `--target2`, `--wrap`,
  `--pic-veneer`, `--nmagic`). lld's ARM PLT entry is fixed 16 B (12 B code + d4d4d4d4 trap
  padding) + 4 B `.got.plt` slot, header 32 B + 3 reserved GOT words.

## Next step
Part B: investigate lld options for an 8-byte PLT (record dead ends), then the --wrap stub
mechanism (ImportStubs: tiny ELF REL writer in symdev-elf2e32, two-pass link: first lld
link → JUMP_SLOT symbols → stubs → relink with --wrap).
