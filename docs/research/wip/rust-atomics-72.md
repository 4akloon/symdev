# WIP: step 72 — real atomics, Arc, Mutex, Once on ARMv5TE (no LDREX/STREX)

Task: ship `shims/common/symrs_atomic.cpp` over an `RFastLock`, raise the target to
`max-atomic-width: 32` + `atomic-cas: true` in the same commit, add `symbian_std::sync`
(`Mutex`, `MutexGuard`, `Once`, re-export `Arc`) and `symbian_std::thread`
(`spawn`/`join` over `RThread`), and an `examples/atomics` that proves the counts through
`symdev test --emulator`. Settle the post-thread-exit access violation at 0x8000A4.

## Findings
- Baseline E32 sizes reproduced on this worktree before any change: hello 3187, hello-raw 752, alloc 4320, shim 4475, files 10423 — exactly the figures in the brief.
- Exact undefined set at `max-atomic-width: 32` / `atomic-cas: true`, read with `nm -u` over the staticlib of a probe that touches every op at every width: 31 symbols = `__atomic_{load,store,exchange,fetch_add,fetch_sub,fetch_and,fetch_or,fetch_xor,fetch_nand,compare_exchange}_{1,2,4}` plus `__sync_synchronize`. No `_8` at any point, so width 32 really does keep 64-bit atomics out.
- `fetch_max`/`fetch_min` and `compare_exchange_weak` emit no libcall of their own: they lower to a `__atomic_compare_exchange_N` loop.
- The static constructor DOES run before `E32Main`: the linker script provides `SHT$$INIT_ARRAY$$Base`/`Limit` around `.init_array` (no KEEP, but `--gc-sections` keeps it anyway), and usrt2_2.lib's `__cpp_initialize__aeabi_` walks exactly that range. Traced with `User::InfoPrint` from inside the constructor.
- **First real bug, and it is ours, not the emulator's.** `__attribute__((constructor))` put `gAtomicLockInit` in `.init_array[0]`, and GCC put the translation unit's own C++ static initialiser `_GLOBAL__sub_I___atomic_load_1` in `.init_array[1]`. The second one runs `RFastLock`'s inline constructor over `gAtomicLock` and re-zeroes `iHandle` AFTER `CreateLocal` already succeeded. Observed: in the constructor `rc=0 h=196610`, in `E32Main` `h=0`, same address 0x400008 both times, while a plain `.bss` sentinel written by the same constructor survived intact. The shim's guard then panicked (silently, as an untrapped Symbian panic does here) on the first atomic.
- Fix: make the creation a static object defined *after* `gAtomicLock` in the same translation unit, so C++'s own within-TU declaration order guarantees the ordering, instead of racing `.init_array` slots.
- With the ordering fixed there is exactly one `.init_array` entry, `handle=196610` reaches `E32Main`, and store/load/fetch_add/compare_exchange and `Arc` clone+drop all give the right answers in EKA2L1.

## Decisions

## Dead ends

## Next step
- Threads: reproduce the survey's access violation after a worker thread exits and find its cause.
