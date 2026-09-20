# Experiment 72 (wip): concurrency primitives on Symbian OS 9.3 / ARMv5TE

Task: survey what atomics and blocking primitives Symbian OS 9.3 on ARMv5TE actually offers, so a later slice can decide what `core::sync::atomic`, `Mutex`, `Once` and threads can honestly be built on.

## Findings

### 1. euser.dso / headers
- No `e32atomics.h` in `~/sdk/S60_3rd_FP2/epoc32/include/` (only glib's `stdapis/glib-2.0/glib/gatomic.h`). The `__e32_atomic_*` family is 9.4+/Symbian^3; `nm -D euser.dso | grep -c '__sync|__atomic|__e32_'` = 0.
- euser.dso DOES export four atomics, declared in `e32std.h` under the comment `// Atomic operations` (lines 4518-4522): `_ZN4User9LockedIncERi`, `_ZN4User9LockedDecERi`, `_ZN4User7SafeIncERi`, `_ZN4User7SafeDecERi` — all `IMPORT_C static TInt f(TInt& aValue)`.
- Blocking primitives exported: RFastLock (CreateLocal/Wait/Signal), RMutex (CreateLocal/CreateGlobal/OpenGlobal/Open/Wait/Signal/IsHeld), RSemaphore (CreateLocal/CreateGlobal/Open*/Wait/Wait(timeout)/Signal/Signal(n)), RCriticalSection (CreateLocal/Wait/Signal/Close), RCondVar (CreateLocal/CreateGlobal/Wait(RMutex)/TimedWait/Signal/Broadcast), RThread::Create x2.
- `nm -D euser.dso` warns `string table [6] is corrupt` but lists 2229 symbols; names carry the `@@euser{000a0000}[100039e5].dll` version suffix.


### 2. GCCE 12.1.0 probes (`/tmp/claude-1000/atomics-work/`)
- Observed GCCE argv (`-O2 -march=armv5t -mapcs -mthumb-interwork -mthumb -msoft-float`), all four of `-march=armv5t|armv5te` x `-mthumb|ARM` give the SAME answer:
  - **relaxed** load/store of 8/16/32 bits: inline `ldr`/`str`, no libcall, no barrier.
  - **acquire load / release store / seq_cst load / seq_cst store** 8/16/32: inline `ldr`/`str` **plus `bl __sync_synchronize`** (undefined).
  - **64-bit** load/store at any ordering: libcall `__atomic_load_8` / `__atomic_store_8`.
  - **every RMW** (`fetch_add`, `exchange`, `compare_exchange`) at 8/16/32/64 and at every ordering: libcall `__atomic_fetch_add_N` / `__atomic_exchange_N` / `__atomic_compare_exchange_N`; `__sync_*` builtins give `__sync_fetch_and_add_4`, `__sync_val_compare_and_swap_4`.
  - GCC never emits `SWP`/`SWPB` for these.
- **Nothing on the recorded link line defines any of them.** Checked with `nm`: `libgcc.a`, `libsupc++.a`, `usrt2_2.lib` (12 members, 18 defined symbols), `euser.dso`, `drtaeabi.dso`, `scppnwdl.dso`, `drtrvct2_2.dso`, `dfpaeabi.dso` — zero hits for `__atomic_*`/`__sync_*`.
- **Link proof:** an `int E32Main()` doing `__atomic_fetch_add`/`__atomic_compare_exchange_n`/seq_cst load on a `volatile u32`, linked with the recorded `link.rs` argv, fails:
  `undefined reference to '__atomic_fetch_add_4' / '__atomic_compare_exchange_4' / '__sync_synchronize' (x2)`. So even an *acquire load* is a link failure today.


### 3. Rust / LLVM (nightly-2026-09-19, rustc 1.100.0-nightly)
- Scratch copy of the target JSON with `max-atomic-width: 32`, `atomic-cas: true`. `core`/`alloc`/`compiler_builtins` build fine with `-Zbuild-std`; the probe crate compiles.
- **LLVM is stricter than GCC:** it lowers EVERY atomic op to a libcall, including a RELAXED load and store. `librprobe.a`'s undefined list is exactly `__atomic_load_4`, `__atomic_store_4`, `__atomic_exchange_1`, `__atomic_fetch_add_4`, `__atomic_compare_exchange_4`. Disassembly confirms `probe_load_relaxed` is `bl __atomic_load_4` with `r1 = 0` (memorder).
- `compiler_builtins` (the build-std member) defines NONE of them.
- Linking that archive with the recorded `link.rs` argv fails with seven `undefined reference to '__atomic_*'` lines. -> raising `max-atomic-width` today just converts a compile error into a link error.
- At `max-atomic-width: 0` the types do not exist at all: `no AtomicU32/AtomicU8/AtomicUsize in sync::atomic`, and `cannot find sync in alloc` (no `Arc`). At 32/true `alloc::sync::Arc` exists.
- **Decision: do NOT change the target JSON in this experiment.** Evidence: nothing on the link line defines the libcalls, so the change would only move the failure later.


### 4. Observed on EKA2L1 (C++ probes built with `symdev build/package/run`, `/tmp/claude-1000/atomics-work/`)
Return values are the OLD value in every case; the predicate differs:
- `User::LockedInc(v)`: 0->1 r=0, 5->6 r=5, -1->0 r=-1. Unconditional increment, returns old.
- `User::LockedDec(v)`: 0->-1 r=0, 5->4 r=5, 1->0 r=1. Unconditional decrement, returns old.
- `User::SafeInc(v)`:  0->0 r=0, 5->6 r=5, -1->-1 r=-1, -5->-5 r=-5. Increments ONLY when the old value is > 0.
- `User::SafeDec(v)`:  0->0 r=0, 5->4 r=5, 1->0 r=1, -5->-5 r=-5, -1->-1 r=-1. Decrements ONLY when the old value is > 0.
- **Atomicity observed.** Two threads (`RThread::Create` + `Logon`/`Resume`), 20000 iterations each, each iteration doing `LockedInc(gShared)` and a hand-rolled `x = gPlain; User::After(0) every 64; gPlain = x+1`: result `LockedInc=40000 plain=20000 lost=20000`. The plain read-modify-write lost exactly half its updates while `LockedInc` lost none. Without the forced yield neither lost anything (EKA2L1 did not preempt the tight loop), so the yield is what makes the test meaningful.
- Creation: `RFastLock/RMutex/RSemaphore/RCriticalSection/RCondVar::CreateLocal` all returned `KErrNone`; handles 196610 / 262147 / 327684 (a kernel object each). `sizeof`: `RFastLock` 8, `RMutex` 4, `RCriticalSection` 8.
- `RMutex`: `IsHeld()` 0 before `Wait`, 1 after; **`Wait` twice from the owning thread returns** -> recursive/re-entrant.
- `RFastLock`: `Wait` twice from the same thread **blocks forever** (the probe never printed past the second `Wait`; run killed on timeout) -> NOT recursive; it is a counting semaphore initialised to 1.
- `RSemaphore::Wait(50000)` on an empty semaphore returns **-33 = KErrTimedOut**; with a token available it returns 0. This is the only try-lock/timeout-shaped primitive in the set (`RFastLock`/`RMutex` have no timed wait).
- `RCriticalSection::IsBlocked()` is 1 inside the critical section.
- **Emulator caveats:** `RCondVar::CreateLocal()` returns `KErrNone` but `Handle()` is 0 - EKA2L1 quirk or a real handle-less object, UNKNOWN. After a worker `RThread` has run and exited, the main thread takes an `Access violation reading address 0x8000A4` at the next few instructions on every probe; single-threaded probes never fault. Emulator artefact, UNKNOWN on a device.
- Operational note: EKA2L1 refuses to install a package whose executable is already on drive E (`Installation of SIS failed` after `Installation done!`), so each probe run needs a fresh app name AND a fresh UID3.

## Decisions

## Dead ends

## Next step

- Prove (or refute) that a lock-backed `__atomic_*` shim makes Rust atomics link and run; then write the deliverable.
