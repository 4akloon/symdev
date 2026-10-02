# WIP: rust-lld spike (experiment 109)

Question: can a Rust SDK app be built with no GCCE on the dev machine — prebuilt shims +
GCC runtime archives shipped, final link by `rust-lld -flavor gnu` instead of GNU ld?

Scratch (outside git): `~/src/rust-lld-spike/` (work/ = projects, logs, ELFs).
Branch: `rust-lld-spike` (worktree `~/worktrees/symdev/rust-lld-spike`). No product code changes.

## Status
- [ ] 0. Read context (shim header, driver link code, experiments)
- [ ] 1. Capture GNU ld argv for console / async / GUI Rust apps
- [ ] 2. Archive members pulled in (--trace / -Map), sizes, licences
- [ ] 3. Replay with rust-lld; adaptations; readelf comparison
- [ ] 4. elf2e32 on lld ELF; E32 compare
- [ ] 5. EKA2L1 run + screenshot; leave probe
- [ ] 6. Effort estimate; experiment 109 record

## Facts
- Setup: symdev built from worktree (`target/debug/symdev`); env in `~/src/rust-lld-spike/env.sh`
  (SYMDEV_* as in brief; `SYMDEV_LD` = `bin/ld-log` wrapper logging argv -> real ld; needs
  `SYMDEV_AR` set explicitly because the wrapper's name does not end in `ld`).
- `symdev new --lang rust --template gui` refuses (TODO: Rust supports only console). GUI app =
  copy of `symbian-rs/examples/ui` (uidemo); async = `examples/async`; leave probe =
  `examples/shim` with `leave_if_error(-1)` and tag `lld109` (`copy-example.sh`).
- GNU baselines saved in `~/src/rust-lld-spike/gnu/<p>/`: hello.elf 16456 / .exe 968 (= exp 108),
  asyncdemo 79404/18431, shimdemo 27032/4452, uidemo 76692/10315.
- Argv logs: `logs/gnu-<p>.argv`. GUI adds `-u symrs_app_create` and 5 UI DSOs; else identical.
- Shim compile argv (logs/gxx-ui.argv): recorded GCCE line + `-D__EXE__` + SHIM_OPTIONS; GUI build
  adds `-DSYMRS_UID3=0x<uid3>` to every shim (only `symrs_avkon.cpp` uses it, for `AppDllUid()`).
  => console shim archive is project-independent; avkon shim is not (UID3 baked in).
  `symdev build` already passes `SYMDEV_UID3` to cargo (symbian-std `option_env!`).
- Archive members pulled (GNU -Map): SDK `eexe.lib(uc_exe_.o,uc_exe.o)`, `usrt2_2.lib(callfirstprocessfn,
  dllexp, ucppinit_aeabi)` [SDK files, not GCCE]; from GCCE only `libsupc++.a(eh_personality.o)`
  [for __gxx_personality_v0 of any TRAP], `libsupc++.a(del_ops.o)` [operator delete(void*,uint),
  async+ui], `libgcc.a(pr-support.o)` [__gnu_unwind_frame, from eh_personality]. hello pulls none
  of them (only symrs_cleanup.o).
- GNU ELF layout (symbianelf script): .text@0x8000 + .emb_text + .plt (8-byte Symbian PLT entries
  `ldr pc,[pc,#-4]; .word 0` with R_ARM_GLOB_DAT in .rel.plt) + rodata + exidx in ONE RX PT_LOAD;
  .bss@0x400000; dynamic sections non-alloc (addr 0, tags hold file offsets); DT_TEXTREL.
- Native elf2e32 constraints (crates/symdev-elf2e32): code = first PF_X PT_LOAD; import = any
  dynamic reloc vs undefined sym, slot MUST be inside code (else `TODO ... outside code`), slot word
  read as addend (`addend<<16|ordinal`); local relocs only ABS32/GLOB_DAT/RELATIVE, else TODO error.

- lld 23.1.1 (rust-lld of nightly-2026-09-19). Adaptations found so far, each forced by an error:
  1. drop `--default-symver` (lld: unknown argument; EXE has no exports, verdef unused).
  2. SDK DSOs: lld refuses `SHT_STRTAB ... non-null terminated` -- 438/570 DSOs pad .strtab
     with 1-3 spaces after the last NUL. Scratch copies with padding zeroed (`fix-dso.py`,
     `dso-fixed/`), same size/offsets.
  3. `-z notext` (ABS32 vs local in .text: GCCE/RVCT code is non-PIC; GNU emits TEXTREL too).
  4. linker script `symbian-lld.ld` (PHDRS: RX text = .text .emb_text .plt .got .rodata
     .constdata .init_array .ARM.extab .ARM.exidx; RW data at -Tdata; dynamic tables in a 3rd R
     PT_LOAD at 0x10000000 that elf2e32 ignores) + hidden `Image$$ER_RO$$Base/Limit`,
     `.ARM.exidx$$Base/Limit`, `SHT$$INIT_ARRAY$$Base/Limit` (eexe.lib needs them; GNU's
     symbianelf script defines them).
  5. `--target2=abs` (lld default got-rel; GNU symbianelf + libsupc++ `__symbian__` personality
     read the extab typeinfo word as an absolute pointer). AND lld cannot emit a dynamic reloc
     for TARGET2 vs an imported symbol (`getDynRel` maps only ABS32/TARGET1) -> rewrite
     R_ARM_TARGET2 -> R_ARM_ABS32 in input objects (`fix-target2.py`): usrt2_2.lib
     (callfirstprocessfn.o, 1), our libsymrs.a (each TRAP's catch). libsupc++.a has none.
  6. `-Bsymbolic`: else calls to the image's own global functions (libcalls `__atomic_*`) go
     through PLT/JUMP_SLOT (GNU symbianelf binds them locally).
- lld PLT is GOT-based: 32-byte PLT0 + 16 B/entry + .got.plt slot (R_ARM_JUMP_SLOT), slot
  initialised to PLT0's address (lazy binding). GNU: 8 B/entry `ldr pc,[pc,#-4]` + GLOB_DAT.
- Native elf2e32 refusals / hazards on the lld ELF (product code unchanged):
  a. JUMP_SLOT import slot word = PLT0 address -> taken as import addend: SILENTLY WRONG
     (spike wrapper zeroes the words).
  b. `TODO: ELF without a writable PT_LOAD` -- lld drops zero-size PT_LOADs (wrapper appends an
     empty RW PT_LOAD at -Tdata, like GNU's).
  c. `TODO: ELF without Symbian$$CPP$$Exception$$Descriptor` -- GNU 2.29.1 copies HIDDEN
     symbols into .dynsym as LOCAL; lld never does. Spike: eexe.lib copy with the symbol made
     STV_DEFAULT (`fix-visibility.py`).
  d. `relocation at 0x8154 targets 0x0` -- lld RELATIVE relocs have symbol 0; elf2e32 takes the
     target (code vs data reloc kind) from the symbol value (GNU always names a section sym).

## Dead ends

## Next step
Read context files.
