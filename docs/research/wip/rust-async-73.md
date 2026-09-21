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
