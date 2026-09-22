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

## Decisions
- Category `RUST` (same as std PAL abort_internal, 4 of 16 units). Reason KErrGeneral (-2), same as std. Line-number reason rejected: +1.5..3.9 KB corpus for a line without a file.
- Handler + alloc handler moved to symbian-runtime/src/panic.rs.
- No lever applied: immediate-abort loses the category; patched core -776 is not worth a core patch + OOM loss.

## Dead ends
- `-Cllvm-args=-trap-func`: ignored by rustc.
- Wrong toolchain src (1.98.1 stable) for a core patch: symbian-rs pins nightly-2026-09-19.

## Next step

OOM run of examples/panic (create ~/.local/share/EKA2L1/data/drives/e/symdev/panic/oom), then C++ scratch (User::Panic cost + LeaveNoMemory without TRAP).
