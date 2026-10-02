# WIP: rust-lld spike (experiment 109)

Question: can a Rust SDK app be built with no GCCE on the dev machine — prebuilt shims +
GCC runtime archives shipped, final link by `rust-lld -flavor gnu` instead of GNU ld?

Scratch (outside git): `~/src/rust-lld-spike/` (work/ = projects, logs, ELFs).
Branch: `rust-lld-spike` (worktree `~/worktrees/symdev/rust-lld-spike`). No product code changes.

## Status
- [x] 0. Read context (shim header, driver link code, experiments)
- [x] 1. Capture GNU ld argv for console / async / GUI Rust apps
- [x] 2. Archive members pulled in (--trace / -Map), sizes, licences
- [x] 3. Replay with rust-lld; adaptations; readelf comparison
- [x] 4. elf2e32 on lld ELF; E32 compare
- [x] 5. EKA2L1 run + screenshot; leave probe
- [x] 6. Effort estimate; experiment 109 record

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

- elf2e32: product code untouched; scratch fork `~/src/rust-lld-spike/elf2e32-fork` (copy of
  symdev-elf2e32/core/uidcrc) with 5 `SPIKE 109` changes: import addend only for ABS32
  (JUMP_SLOT/GLOB_DAT = S); symbol-less RELATIVE target = linked word; target == code end counts
  as code (lld `.ARM.exidx$$Limit`); missing RW PT_LOAD = empty data at 0x400000; exception
  descriptor looked up in .symtab when not in .dynsym. Its 42 unit tests pass and it gives
  byte-identical E32 (mask 0x14-17, 0x24-2B) for all 4 GNU ELFs. Used via SYMDEV_ELF2E32.
  Wrapper `bin/ld-lld` then needs no post-link patching (LLD_POSTFIX=1 path unused).
- RESULT (lld link + fork elf2e32), all 4 build: E32 .exe gnu/lld: hello 968/1044, async
  18431/18577, shim 4452/4572, ui 10315/10511 (uncompressed 1340/1584, 33948/34684,
  7092/7552, 17092/17936). Import section bytes identical; import words (addend<<16|ordinal)
  identical per DLL in all 4; DT_NEEDED identical in all 4; data section identical; code relocs
  equal except async 211->203 text, ui 124->115 text: GNU makes PLT entries (+ABS32 reloc) for
  the image's own `symrs_app_*`/`_ZdlPvj` called from the Thumb shim; lld -Bsymbolic calls them
  directly. Growth = PLT+GOT: GNU 8 B/import, lld 16 B PLT + 4 B GOT + 44 B fixed (PLT0 32,
  3 reserved GOT words); .ARM.exidx +8..+32 B. Everything else same size.
- Tools: `e32dump.py`, `e32cmp.py`, `relocmap.py`, `e2e.sh` (symdev's EXE elf2e32 argv; rerun
  reproduces symdev's .exe exactly). Outputs in `e32/`, `gnu/`, `lld/`.

- EKA2L1, lld builds (symdev package + run, own PID, kill -9 own PID only; scratch signing
  password `SYMDEV_SIGN_PASSWORD=spike109`, keys generated in scratch build/):
  - hello: log `Trying to display: Hello from Rust SDK (19 chars)`; screen black (console app,
    notifier not drawn) `shots/lld-hello-1.png`.
  - leave probe (shim, `User::LeaveIfError(-1)` in symrs_leave.cpp TRAP): log
    `lld109 mkdirall=0 trapped=-1 bad=0 ensured=0 sign=-42 alive` -> leave thrown, caught, -1
    returned, process continued.
  - ui: `shots/lld-ui-1.png` title Bars, 3 bars, `bars=3 keys=0 cmd=0`, Options/Exit; F1 via
    XSendEvent -> `shots/lld-ui-2.png` Options menu with Rust items (More bars, Fewer bars,
    Reset, Exit) -> DynInitMenuPaneL + trapped AddMenuItemL work.
  - async: `symdev test --emulator` -> 15 passed (CActiveScheduler::Start TRAP, timers).

- GNU runs (rebuilt; ELFs == saved baselines) with the same steps: hello and leave probe give
  the same log lines (`trapped=-1 ... alive`); runshot.py now deletes the old log first (a
  stale log of the other variant could satisfy the wait -- reran gnu-hello/gnu-shim after the
  fix). GUI: F1, F1 (Select "More bars") -> `bars=4 keys=0 cmd=1` in both
  (`shots/{gnu,lld}-ui-cmd-{1,2}.png`); pixel diff GNU vs lld = 46 px in a 7x9 box at
  (548,157) = the status-pane clock minute; hello/shim shots identical (black). async GNU:
  15 passed.
- NEGATIVE control (`lld-neg/`): lld with `--target2=got-rel` and the unfixed shim archive ->
  leave probe never reaches InfoPrint; emulator logs a CPU register dump and exits within ~2 s.
  So the positive `trapped=-1` really depends on the extab TARGET2 encoding.
- XLeaveException typeinfo: both linkers bind the catch's `_ZTI15XLeaveException` to the shim's
  own COMDAT copy (GNU: ABS32 vs its local dynsym entry; lld: RELATIVE); its vtable
  `_ZTVN10__cxxabiv117__class_type_infoE` is imported from drtaeabi in both.

- NO-GCCE LINK (prebuilt set in `~/src/rust-lld-spike/prebuilt/lib`, 90 592 B, 23 619 B .tar.gz):
  `libsymrs.a` 19 322 (6 common shims; objects byte-identical across all 4 projects, the GUI
  build's -DSYMRS_UID3 included), `libsymrs_ui.a` 59 362 (avkon/list/note/query; avkon edited
  in scratch `prebuilt-src/` to read UID3 from `extern "C" char symrs_uid3[]`, link adds
  `--defsym=symrs_uid3=0x<uid3>`; list/note/query objects == per-app ones), both with TARGET2
  -> ABS32 (3 + 7 relocs); `libsupc++.a` 7 736 (del_ops.o, eh_personality.o, --strip-debug),
  `libgcc.a` 4 172 (pr-support.o, _thumb1_case_uqi.o). Closure computed by linking ALL 10 shims
  `--whole-archive` with lld `--why-extract` (`closure/why.txt`): exactly those 4 members
  (_thumb1_case_uqi from symrs_note.o's Thumb switch).
  `nogcce-link.py`: GNU argv with every GCCE path remapped, rust-lld inside
  `bwrap --tmpfs ~/gcc-builds` under strace: all 4 link, 0 accesses to gcc-builds; files opened =
  SDK copies, prebuilt/lib, script, the 2 Rust archives, rust-lld's own libs.
  Fork elf2e32 on those ELFs: E32 IDENTICAL (mask header CRC+time) to the per-app-shim lld
  builds for all 4, the GUI one included (--defsym gives the same literal word).
- readelf -a dumps: `~/src/rust-lld-spike/readelf/{gnu,lld}-<p>.txt`.

- Coordination (lead, after my runs): several agents share ~/.local/share/EKA2L1; every further
  emulator run must be wrapped: `flock ~/.local/share/EKA2L1/.symdev-agent.lock -c '<run>'`,
  short locked section, kill -9 own PID only. All runs above were made before the note.
- Extra: lld-linked `ui-list` (batch copy) renders the Avkon list; Down moves the highlight
  (`shots/lld-uilist-{1,2}.png`).

## Dead ends

## Next step
Done: experiment 109 complete (batch filled in: 15/15 examples link + post-link, import words
identical; notes/query DT_NEEDED keep eikcoctl under GNU only). Verification 2026-10-02: cargo
test --workspace 714 passed / 0 failed, clippy 0 warnings (docs-only branch); nogcce == lld E32
for 4/4; fork-on-GNU == symdev for 4/4; 0 gcc-builds accesses in all 4 straces; none of my
emulator PIDs alive. Branch diff vs merge-base: only experiment-backlog.md and this note.
