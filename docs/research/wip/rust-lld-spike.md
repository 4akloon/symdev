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

## Dead ends

## Next step
Read context files.
