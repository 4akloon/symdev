# named-panic (WIP)

Task: replace `User::Exit(-1)` in the `no_std` `#[panic_handler]` (symbian-runtime) with
`User::Panic(category, reason)` — user's choice «User::Panic з категорією» (parity with C++
on diagnosability). Bind `User::Panic` in symbian-sys, choose category (<= KMaxExitCategoryName)
and reason, decide OOM (exit vs panic), measure cost per no_std example vs main, find reclaimable
levers, observe the panic in the EKA2L1 log (Kernel:trace, restore log-filter), compare with a
C++ `User::Panic` in a scratch copy of docs/research/cpp-parity/hello. Experiment entry in the
backlog (99 was taken on main; this is 100). Branch `named-panic`, base main 7231c84. Do not merge.

## Findings
- Binding already exists: symbian-sys/src/euser.rs:26-28 `User_Panic(*const TDesC16, i32) -> !`; `nm -D euser.dso`: `00000a24 T _ZN4User5PanicERK7TDesC16i@@euser{000a0000}[100039e5].dll`. e32std.h:4457 `IMPORT_C static void Panic(const TDesC& aCategory,TInt aReason);`.
- `KMaxExitCategoryName=0x10` (e32const.h:167), `TExitCategoryName = TBuf<KMaxExitCategoryName>` (e32cmn.h:1750): 16 UTF-16 units.
- Precedent: the `std` PAL `abort_internal` (rust-src/overlay/.../pal/symbian/mod.rs:29,59) already panics `User::Panic("RUST", KErrGeneral=-2)`. Other in-tree categories: `symrs-tls` (symbian-std thread/local.rs), libcalls lock.rs PANIC_CATEGORY (12 chars).
- e32panic.h:1661 `EUserLeaveWithoutTrap=175` (USER category) — "User::Leave() called and there is no TRAP frame". The recollection "USER 0" is not what the header says (USER 0 = EInvariantFalse). To observe in emulator.
- Baseline (main 7231c84) .exe: alloc 3876 async 20422 atomics 10713 cleanup 5769 files 11088 hello 2567 hello-raw 808 locale 8486 net 12419 notes 14648 query 19372 shim 4520 spawnee 3257 time 13523 tls 15098 ui 12958 ui-list 13870 (17 no_std examples; `cleanup` is new since size-levers).
- Today's panic path in hello: rust_begin_unwind 16 B (push fp,lr; mov fp; mvn r0,#0; bl User::Exit), panic_fmt 12 B, panic_bounds_check 12 B. Nothing else.
- eexe.lib's startup (KLitUser "USER", _xxxx_call_user_invariant) already imports BOTH User::Exit and User::Panic into every EXE (nm: U _ZN4User5PanicERK7TDesC16i in base hello and all 17). So User::Panic adds no import.
- V1 (User::Panic("RUST", KErrGeneral)): +8 .text (ldr r0 + literal word) +12 .rodata (Lit16<4> CATEGORY) in every example that has the handler (15); hello-raw, shim 0 (own handler / no runtime). .exe +6..+31 (deflate/alignment), corpus +242.

- V2 reason = `info.location().line()`: corpus .exe +3 895 vs V1 (.text +972, .rodata +5 928: file-path strings + 16-B Location per site); async +672, query +351, tls +318, time +315, hello +7.
- V2 + `-Zlocation-detail=line` (file redacted): corpus +1 568 vs V1 (async +460, tls +167, query +142, hello +7). Still a line with no file.
- Lever: #[cold]+#[inline(never)] on the handler: 0 bytes in all 17 (handler already out of line).
- Panic stubs already take no Arguments: core::panicking::panic_fmt is 12 B (push; mov fp; bl rust_begin_unwind); call sites pass nothing. What survives is cold *selection* logic that ends in different noreturn stubs, e.g. core::str::slice_error_fail_rt 428 B (notes, query, ui).
- immediate-abort vs V1 (nightly-2026-09-19, this main): corpus -2 768 (.text -3 632); notes -473, query -450, ui -424, async -263, hello -31. Also removes symbian_alloc::oom: under immediate-abort alloc::handle_alloc_error calls ct_error -> panic! -> trap (alloc.rs:655-661), so OOM traps too.
- Lever: immediate-abort + -Cllvm-args=-trap-func=symrs_rust_trap: rustc ignores it (0 calls to the hook, `udf #65006` stays in notes): dead end.
- Lever: patched core (7 `intrinsics::abort()` in core/src/panicking.rs -> extern "C" symrs_rust_abort -> User::Panic RUST -2) + immediate-abort: corpus -776 vs V1 (notes -251, query -269, ui -238, others within ±50). Keeps category but OOM becomes RUST -2 and needs a core patch. The other ~2 KB of immediate-abort is the trap itself: `udf` keeps functions leaf, a `bl` needs push{fp,lr} (frame-pointer=always).
- Exception-handler route (User::SetExceptionHandler + immediate-abort): EKA2L1 never dispatches a CPU fault to a user handler (~/src/EKA2L1/src/emu/kernel/src/kernel.cpp:221-249 always kills KERN-EXEC 3), so unobservable here: not tried.
- hello-raw and shim: 0 change — no panic site reaches the handler, gc'd.
- OBSERVED (EKA2L1, Kernel:trace, examples/panic default path, index out of bounds): `T .../kernel/src/thread.cpp:542 [Kernel]: Thread Main panicked with category: RUST and exit code: -2`. Emulator process exited by itself afterwards; config.yml restored byte-identical.
- panicdemo.exe 2 011 B; uid3 0xe00006a8 (a0..a3 taken on this and other branches).
- Scratchpad dir is shared with other agents' files; mine are in scratchpad/np/.

- OBSERVED: examples/panic OOM path (vec! of 576 MiB, then 9 MiB) -> `Access violation reading address 0x8000A4 in thread Main` + `Thread Main terminated peacefully with category: KERN-EXEC and exit code: 3`. Probes: try_reserve + User::Exit(44) -> same; User::Exit(44) with no alloc -> same; User::Exit(45) first thing in main (no fs) -> same (0x7000A4). So **User::Exit called while the thread's CTrapCleanup is installed dies KERN-EXEC 3 in EKA2L1**; hello's normal return (cleanup dropped first, then eexe's User::Exit) logs `Thread Main forcefully killed with category: None and exit code: 0`.
- C++ scratch (copy of cpp-parity/hello, uid 0xe00006a9, np/cpp): PANIC_NOW -> `Thread Main panicked with category: CPP and exit code: 42`; EXIT_WITH_CLEANUP (CTrapCleanup::New(); User::Exit(45)) -> access violation 0x7000A4 + KERN-EXEC 3 (same as Rust); EXIT_NO_CLEANUP -> `forcefully killed with category: None and exit code: 46`; LEAVE_NO_TRAP (CTrapCleanup + User::LeaveNoMemory()) -> `panicked with category: E32USER-CBase and exit code: 65` (EClnLevelUnderflow, e32panic.h) — not USER 0, not USER 175; LEAVE_TRAPPED (TRAPD, delete cleanup, return err) -> `forcefully killed with category: None and exit code: -4`.
- C++ size: baseline 802 B .exe (.text 232 .rodata 76); + one `if (note.Length() > 60) User::Panic(_LIT "CPP", 42)`: 831 B (+29), .text +24, .rodata +12 (the _LIT), imports unchanged (eexe already imports User::Panic).
- spawnee's doc blames "EKA2L1 cannot spawn an image with a writable data section" for a KERN-EXEC 3 "reading its own heap base + 0xA4"; spawnee calls User_Exit inside main with the cleanup installed — same signature as above. Hypothesis only, not checked.

- OBSERVED final code: examples/panic default -> `Thread Main panicked with category: RUST and exit code: -2`; with E:\\symdev\\panic\\oom -> `Thread Main panicked with category: RUST and exit code: -4`. Old handler (User::Exit(-1)) on the same example -> access violation 0x8000A4 + `terminated peacefully with category: KERN-EXEC and exit code: 3`: under #[symbian_std::main] the old panic path never showed -1 in the emulator.

- Rebased onto main aae58bb (coordinator: experiment 99 taken -> this is 100; locale now 12 031). Final vs aae58bb, .exe: alloc 3876->3896 async 20423->20451 atomics 10720->10723 cleanup 5767->5783 files 11091->11102 hello 2567->2584 hello-raw 808 = locale 12031->12045 net 12423->12434 notes 14649->14670 query 19374->19392 shim 4520 = spawnee 3261->3274 time 13525->13561 tls 15095->15096 ui 12950->12975 ui-list 13869->13907; corpus +272; .text +16 (+8 handler, +8 alloc_error 24 B replacing oom 16 B; hello +8, no OOM path), .rodata +12..16 (CATEGORY). panic example 2 010.
- symdev test --emulator after rebase: async 15, atomics 23, cleanup 2, files 26, locale 7, notes 3, query 4, time 29, tls 45, ui 3, ui-list 6, net 22 passed (peers 18974/18975).
- Backlog 99's note "C++ baseline's thread also ends KERN-EXEC 3 terminated peacefully after writing its report" is plausibly the same User::Exit-under-CTrapCleanup behaviour; not checked.

## Decisions
- Category `RUST` (same as std PAL abort_internal, 4 of 16 units). Reason KErrGeneral (-2), same as std. Line-number reason rejected: +1.5..3.9 KB corpus for a line without a file.
- Handler + alloc handler moved to symbian-runtime/src/panic.rs.
- OOM -> User::Panic("RUST", KErrNoMemory=-4): Exit(-4) from inside main is KERN-EXEC 3 here (C++ too); C++'s -4 exit exists only after a top-level TRAPD returns, which abort-on-OOM cannot do; an untrapped C++ leave is itself a panic. Distinct reason (-4 vs -2), same category. symbian_alloc::oom deleted (no other user).
- No lever applied: immediate-abort loses the category; patched core -776 is not worth a core patch + OOM loss.

## Dead ends
- `-Cllvm-args=-trap-func`: ignored by rustc.
- Wrong toolchain src (1.98.1 stable) for a core patch: symbian-rs pins nightly-2026-09-19.

## Next step

std-hello/std-net tests running; then gates, report examples with symdev test --emulator, gates, backlog entry 99.
