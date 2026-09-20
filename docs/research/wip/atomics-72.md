# Experiment 72 (wip): concurrency primitives on Symbian OS 9.3 / ARMv5TE

Task: survey what atomics and blocking primitives Symbian OS 9.3 on ARMv5TE actually offers, so a later slice can decide what `core::sync::atomic`, `Mutex`, `Once` and threads can honestly be built on.

## Findings

### 1. euser.dso / headers
- No `e32atomics.h` in `~/sdk/S60_3rd_FP2/epoc32/include/` (only glib's `stdapis/glib-2.0/glib/gatomic.h`). The `__e32_atomic_*` family is 9.4+/Symbian^3; `nm -D euser.dso | grep -c '__sync|__atomic|__e32_'` = 0.
- euser.dso DOES export four atomics, declared in `e32std.h` under the comment `// Atomic operations` (lines 4518-4522): `_ZN4User9LockedIncERi`, `_ZN4User9LockedDecERi`, `_ZN4User7SafeIncERi`, `_ZN4User7SafeDecERi` — all `IMPORT_C static TInt f(TInt& aValue)`.
- Blocking primitives exported: RFastLock (CreateLocal/Wait/Signal), RMutex (CreateLocal/CreateGlobal/OpenGlobal/Open/Wait/Signal/IsHeld), RSemaphore (CreateLocal/CreateGlobal/Open*/Wait/Wait(timeout)/Signal/Signal(n)), RCriticalSection (CreateLocal/Wait/Signal/Close), RCondVar (CreateLocal/CreateGlobal/Wait(RMutex)/TimedWait/Signal/Broadcast), RThread::Create x2.
- `nm -D euser.dso` warns `string table [6] is corrupt` but lists 2229 symbols; names carry the `@@euser{000a0000}[100039e5].dll` version suffix.

## Decisions

## Dead ends

## Next step

- Read header semantics for the four atomics and the lock classes; then GCCE probes.
