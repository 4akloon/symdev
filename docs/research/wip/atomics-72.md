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

## Decisions

## Dead ends

## Next step

- Rust side: raise max-atomic-width to 32 in a scratch copy of the target JSON and see what rustc/LLVM emits.
