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
- [ ] B: lld options investigation
- [ ] B: stub mechanism + ImportStubs type
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

## Dead ends

## Next step
Part B: investigate lld options for an 8-byte PLT (record dead ends), then the --wrap stub
mechanism (ImportStubs: tiny ELF REL writer in symdev-elf2e32, two-pass link: first lld
link → JUMP_SLOT symbols → stubs → relink with --wrap).
