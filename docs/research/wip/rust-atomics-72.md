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
- **The access violation is settled, and it is not about code at all.** The address in the survey, 0x8000A4, is a HEAP CELL, not `.text` + 0xA4: in this worktree the process heap sits at 0x800000 (Rust probe) or 0x700000 (C++ probe) and `User::Alloc` hands back base + 0xa0. The fault is the main thread's next allocation after a worker thread exits, reading the heap chunk's own base.
- It is specific to `RThread::Create(name, fn, stack, RAllocator* aHeap, ptr, owner)` — the overload that shares the creating thread's heap. With `RThread::Create(name, fn, stack, heapMin, heapMax, ptr, owner)`, where the worker gets its own heap, the main thread allocates happily after the join and the program runs to the end. `RAllocator::Open()` on the shared heap before the create does NOT help.
- Reproduced in **pure Symbian C++ with no Rust in the picture** (scratchpad `threadheap/`, one `#define SHARE_THE_HEAP` flipping it): `repro alloc while it runs = 7340192` then `repro joined, exit type = 0` then `Access violation reading address 0x700000 in thread Main`. With the define at 0 the same program prints `repro alloc after join`, `repro alloc after close` and `repro still alive`.
- EKA2L1 hand-writes the thread entry routine (`src/emu/kernel/src/libmanager.cpp`, `thread_entry_routine_`): given an allocator it calls euser's heap-switch export, given sizes it calls the chunk-heap export and then switches. It bypasses the SDK's own thread heap setup, which is where a supplied allocator's reference counting would live. No device here, so "the emulator is wrong and a phone is right" stays unproven — but the fault is certainly not ours.
- **The way out, measured:** create the worker with the own-heap overload (so nothing kills a heap the creator is using) and have the worker call `User::SwitchAllocator(main_heap)` as its first instruction. Observed: the worker's allocator becomes 0x800000, the creator's heap; 400 interleaved alloc/free pairs on each thread with forced yields all succeed; and after the join the main thread allocates again, including a 16 KB block that walks the free list, and the program runs to the end.
- Whether the process heap is internally locked is not observable from outside (`RAllocator`'s flags are protected and `RHeap` is not even declared in this SDK), so the allocator takes a lock of its own once a thread has been spawned rather than trusting that test.

## Decisions

## Dead ends

## Added to the slice by the owner, mid-task (do before finishing)
- C++ in the shim only where `TRAP` is unavoidable. Port `symrs_atomic.cpp` and `symrs_cstring.cpp` to Rust; leave `symrs_f32.cpp` and `symrs_leave.cpp` (the two `TRAP`s) in C++ and say so in `symrs_shim.h`.
- Traps to plan for: (1) the `__atomic_*` bodies must not emit an atomic or they call themselves — use `read_volatile`/`write_volatile` and check the object for `bl __atomic_*` inside our own functions; (2) initialise the lock with `User::LockedInc` as a once (`if LockedInc(&claim) == 0 { create } else { spin }`) instead of a C++ static constructor; (3) signatures must match LLVM exactly, wrong arity links and corrupts at run time.
- Verify with the same example and the same counts; report sizes before and after.

## Next step
- Threads: reproduce the survey's access violation after a worker thread exits and find its cause.
