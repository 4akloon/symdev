# WIP: step 73 — Rust async/await on Symbian Active Objects

Task: a single-threaded executor on `CActiveScheduler` (own or join), `TRequestStatus` -> `Waker`, `RTimer` sleep, and an example where two 300 ms timers awaited concurrently finish in ~300 ms.

## Findings

- `TRequestStatus`, `User_WaitForRequest`, `RFastLock`, `RSemaphore` all live in `symbian-sys/src/thread.rs`; `esock/mod.rs` re-exports the first two. Do not redeclare.
- `symbian-core/src/net/request.rs::blocking` is the synchronous twin: one private status per call, documented reason (a thread's request semaphore is shared between outstanding requests).
- `symrs_shim.h` rule: a file belongs in `shims/common/` only when it needs `TRAP` or (step 75) a C++ subclass with virtuals. `CActive` is exactly the second case.
- `avkon-rust-spec` §1.3: CONE installs `CCoeScheduler` and `CCoeEnv` is a `CActive` on it -> a GUI app must never `Install`/`Start`/`Stop`; the executor must be able to just `CActiveScheduler::Add`.
- Experiment 78 member ABI: non-virtual member = AAPCS with `this` as argument 0; sret displaces `this`. Experiment 80: `codegen-units=16`/`lto=false` profile keeps unused archive members out.
- Experiment 85: `Instant` = `User::TickCount` + `UserHal::TickPeriod`, resolution 15.625 ms, so a 300 ms measurement quantises to +-15.625 ms; `Instant::now()` returns `io::Result`.

## Decisions

- Waker is `Arc<W>` where `W`'s only state is an `AtomicU32` flag, so `Wake`'s `Send + Sync` is honest rather than an `unsafe impl`; the executor scans flags instead of keeping a shared ready queue.
- One driver: the shim's `RunL` records the completion and drains the executor. `block_on` additionally installs a scheduler and uses `Start`/`Stop`; the joined form does neither.

## Dead ends

## Next step

- Read the governing docs (design spec §6a/§7/§8/§11, avkon-rust-spec active scheduler, eka2-concurrency, experiments 78/80/84) and `symbian-core/src/net/`.
- SDK headers are extended-ASCII: `grep` needs `-a` or it silently prints nothing. Cost me four searches.
- `class RTimer : public RHandleBase` — `e32std.h` line 3159: `CreateLocal()` -> TInt, `After(TRequestStatus&, TTimeIntervalMicroSeconds32)`, `Cancel()`, all void/TInt, no trailing L, so no shim. `TTimeIntervalMicroSeconds32` is `TTimeIntervalBase` = one `TInt`, inline ctors only, so trivially copyable.
- `e32base.h` 1580: `CActive` — `RunL()`/`DoCancel()` pure virtual, `iStatus` is a public member, `SetActive()` and the ctor are protected -> a subclass is the only way in, which is rule 3 of `symrs_shim.h`. `CActiveScheduler` (2828) has static `Install/Add/Start/Stop/Current/RunIfReady`.
- euser.dso exports all of them (`_ZN6RTimer5AfterER14TRequestStatus27TTimeIntervalMicroSeconds32`, `_ZN16CActiveScheduler3AddEP7CActive`, ...).
- The C++ shim is auto-globbed: `RustSdk::shim_sources` takes every `shims/common/*.cpp`, so a new file needs no build change. `RustSdk::LIBRARIES` needs no new DSO either — `CActive`/`CActiveScheduler`/`RTimer` are all euser.
- Only `symbian-macros` has host tests in `symbian-rs`; these crates are verified by their example in the emulator, so that example is the test.

## Plan (decided, 2026-09-21)

1. `shims/common/symrs_active.cpp`: `CSymRsActive : CActive` forwarding `RunL`/`DoCancel` to a two-slot vtable of function pointers with an opaque context, plus `symrs_scheduler_*` (the one TRAP is `CActiveScheduler::Start`).
2. `symbian-sys`: `src/active.rs` (the shim entries + `CActiveScheduler::Current`, which is a plain static and needs no shim) and `RTimer` in `src/time.rs`.
3. `symbian-async`: `Request<S: Source>` (a boxed state the `CActive` points at, generic over what issues and cancels — so step 74's sockets implement `Source` and reuse the whole thing), `Sleep` over `RTimer`, `join`/`race`, a process-wide single-threaded `Executor` bound to one `CActiveScheduler` (checked, a different thread has a different scheduler), `block_on` (owns a scheduler) and `spawn` (joins one).
4. The waker is `Arc<TaskWaker>` whose only state is an `AtomicU32` flag, so `Wake`'s `Send + Sync` is true; the driving is done by the request's `RunL`, which is on the scheduler's thread by construction.
5. Mixing guard: `symbian_core` counts outstanding executor requests, and `blocking()` refuses while any exist on a thread that has a scheduler installed.
- ABI probe (recorded GCCE argv, `objdump -dr`): `t->After(*s, TTimeIntervalMicroSeconds32(us))` is `push {r4,lr}; bl _ZN6RTimer5After...` with **no register shuffle** — `this`, the status and the interval stay in r0/r1/r2. Sizes from the same probe: `RTimer` 4, `TTimeIntervalMicroSeconds32` 4, `CActive` 28, `TRequestStatus` 8.
- `symrs_active.cpp` compiles with that argv; undefined symbols are euser's `CActiveScheduler`/`CActive`/`User::LeaveIfError`, drtaeabi's `__cxa_*` and `_ZdlPvj` from `scppnwdl.dso` — all already on the link line, so no new DSO.
