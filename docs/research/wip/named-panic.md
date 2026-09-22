# named-panic (WIP)

Task: replace `User::Exit(-1)` in the `no_std` `#[panic_handler]` (symbian-runtime) with
`User::Panic(category, reason)` — user's choice «User::Panic з категорією» (parity with C++
on diagnosability). Bind `User::Panic` in symbian-sys, choose category (<= KMaxExitCategoryName)
and reason, decide OOM (exit vs panic), measure cost per no_std example vs main, find reclaimable
levers, observe the panic in the EKA2L1 log (Kernel:trace, restore log-filter), compare with a
C++ `User::Panic` in a scratch copy of docs/research/cpp-parity/hello. Experiment entry in the
backlog (next free number on main: 99). Branch `named-panic`, base main 7231c84. Do not merge.

## Findings
- Binding already exists: symbian-sys/src/euser.rs:26-28 `User_Panic(*const TDesC16, i32) -> !`; `nm -D euser.dso`: `00000a24 T _ZN4User5PanicERK7TDesC16i@@euser{000a0000}[100039e5].dll`. e32std.h:4457 `IMPORT_C static void Panic(const TDesC& aCategory,TInt aReason);`.
- `KMaxExitCategoryName=0x10` (e32const.h:167), `TExitCategoryName = TBuf<KMaxExitCategoryName>` (e32cmn.h:1750): 16 UTF-16 units.
- Precedent: the `std` PAL `abort_internal` (rust-src/overlay/.../pal/symbian/mod.rs:29,59) already panics `User::Panic("RUST", KErrGeneral=-2)`. Other in-tree categories: `symrs-tls` (symbian-std thread/local.rs), libcalls lock.rs PANIC_CATEGORY (12 chars).
- e32panic.h:1661 `EUserLeaveWithoutTrap=175` (USER category) — "User::Leave() called and there is no TRAP frame". The recollection "USER 0" is not what the header says (USER 0 = EInvariantFalse). To observe in emulator.
- Baseline (main 7231c84) .exe: alloc 3876 async 20422 atomics 10713 cleanup 5769 files 11088 hello 2567 hello-raw 808 locale 8486 net 12419 notes 14648 query 19372 shim 4520 spawnee 3257 time 13523 tls 15098 ui 12958 ui-list 13870 (17 no_std examples; `cleanup` is new since size-levers).
- Today's panic path in hello: rust_begin_unwind 16 B (push fp,lr; mov fp; mvn r0,#0; bl User::Exit), panic_fmt 12 B, panic_bounds_check 12 B. Nothing else.
- eexe.lib's startup (KLitUser "USER", _xxxx_call_user_invariant) already imports BOTH User::Exit and User::Panic into every EXE (nm: U _ZN4User5PanicERK7TDesC16i in base hello and all 17). So User::Panic adds no import.
- V1 (User::Panic("RUST", KErrGeneral)): +8 .text (ldr r0 + literal word) +12 .rodata (Lit16<4> CATEGORY) in every example that has the handler (15); hello-raw, shim 0 (own handler / no runtime). .exe +6..+31 (deflate/alignment), corpus +242.

## Dead ends

## Next step

Measure V2 (reason = Location::line()), then immediate-abort symbol diff for levers.
