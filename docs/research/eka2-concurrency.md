# Concurrency primitives on Symbian OS 9.3 / ARMv5TE

Experiment 72. What the platform actually offers for atomics, locks and threads, so a later
slice can decide honestly what `core::sync::atomic`, `Mutex`, `Once` and threads are built on.

Every claim below is either **observed** on this host (a `nm` output, a compiler probe, a run
in EKA2L1) or marked **UNKNOWN** with what would settle it. Nothing here is recollection.
Scratch: `/tmp/claude-1000/atomics-work/` (outside git). SDK: `~/sdk/S60_3rd_FP2`.

## 0. The short answer

ARMv5TE has no `LDREX`/`STREX`, so no compiler can generate a lock-free atomic
read-modify-write inline. Symbian 9.3 does **not** fill the gap: `euser.dso` exports four
`TInt`-only counters and nothing else, there is no `e32atomics.h`, and nothing on the link
line defines a single `__atomic_*` or `__sync_*` libcall. Today any `AtomicUsize` in a Rust
crate is a link error.

A **process-wide lock backing the libatomic entry points** closes the gap completely: with a
~110-line `__atomic_*` shim over one `RFastLock`, Rust's `AtomicU32::fetch_add` and
`compare_exchange` link through the recorded line and give exact results across two threads
in the emulator. That is the recommended route, and it is what should carry
`max-atomic-width: 32` / `atomic-cas: true` into the target JSON — in the same change, not
before.

## 1. What `euser.dso` exports

`nm -D ~/sdk/S60_3rd_FP2/epoc32/release/armv5/lib/euser.dso` (2229 symbols; binutils warns
`string table [6] is corrupt` but lists them all; every name carries the
`@@euser{000a0000}[100039e5].dll` version suffix).

| Mangled export | Header signature (`e32std.h`, under the comment `// Atomic operations`, lines 4518–4522) |
|---|---|
| `_ZN4User9LockedIncERi` | `IMPORT_C static TInt LockedInc(TInt& aValue);` |
| `_ZN4User9LockedDecERi` | `IMPORT_C static TInt LockedDec(TInt& aValue);` |
| `_ZN4User7SafeIncERi`  | `IMPORT_C static TInt SafeInc(TInt& aValue);` |
| `_ZN4User7SafeDecERi`  | `IMPORT_C static TInt SafeDec(TInt& aValue);` |

That is the whole documented atomic surface of 9.3.

- **There is no `e32atomics.h` in this SDK.** The only file matching `*atomic*` under
  `~/sdk/S60_3rd_FP2` is glib's `epoc32/include/stdapis/glib-2.0/glib/gatomic.h`, which
  declares glib's own `g_atomic_*` as `IMPORT_C` functions of a DLL — i.e. glib's Symbian
  port also had to put its atomics behind a call, not inline.
- **No `__e32_atomic_*` family**: `nm -D euser.dso | grep -cE '__sync|__atomic|__e32_'` = 0.
  That family is Symbian^3 / 9.4+; it does not exist here.
- **Memory ordering: the headers claim nothing.** The four declarations carry no doxygen and
  no ordering wording anywhere in `epoc32/include`. What ordering they imply is
  **UNKNOWN**. What would settle it: the Symbian Foundation source of
  `e32/euser/maths/um_norm.cpp` / the `LockedInc` implementation, or a disassembly of a real
  device `euser.dll` (no ROM on this host).

### Observed behaviour of the four (EKA2L1, single thread)

Every one returns the **old** value. The predicate differs:

| Call | 0 | 5 | 1 | −1 | −5 |
|---|---|---|---|---|---|
| `LockedInc(v)` | 0→1, r=0 | 5→6, r=5 | — | −1→0, r=−1 | — |
| `LockedDec(v)` | 0→−1, r=0 | 5→4, r=5 | 1→0, r=1 | — | — |
| `SafeInc(v)`  | 0→0, r=0 | 5→6, r=5 | — | −1→−1, r=−1 | −5→−5, r=−5 |
| `SafeDec(v)`  | 0→0, r=0 | 5→4, r=5 | 1→0, r=1 | — | −5→−5, r=−5 |

So `Locked*` is an unconditional ±1 returning the old value; `Safe*` does the ±1 **only when
the old value is strictly positive** — reference-count semantics, matching the SDK's own use
in `f32fsys.h:231` (`inline TInt Inc() {return(User::SafeInc(iAccessCount));}`).

### Atomicity, observed

Two threads (`RThread::Create` + `Logon`/`Resume`), 20000 iterations each. Each iteration
does `User::LockedInc(gShared)` and, beside it, a hand-rolled `x = gPlain;` … `gPlain = x+1;`
with a `User::After(0)` every 64th iteration so the emulator actually reschedules inside the
window:

```
2x20000: LockedInc=40000 plain=20000 lost=20000
```

`LockedInc` lost nothing; the plain read-modify-write lost exactly half its updates. Without
the forced yield both came out exact — EKA2L1 does not preempt a tight loop — so the yield is
what makes the test mean anything. **UNKNOWN:** the same on real hardware.

## 2. What the compiler does on its own (GCCE 12.1.0)

Probes compiled with the observed GCCE argv from
`crates/symdev-build/src/driver/compile.rs` (`-O2 -fexceptions -march=armv5t -mapcs
-mthumb-interwork -mthumb -msoft-float …`). All four of
`-march=armv5t|armv5te` × `-mthumb`|ARM give the same answer, and GCC never emits `SWP`/`SWPB`:

| Operation | 8 / 16 / 32 bit | 64 bit |
|---|---|---|
| relaxed load / store | **inline** `ldr` / `str`, no barrier | libcall `__atomic_load_8` / `__atomic_store_8` |
| acquire load, release store, seq_cst load/store | inline `ldr`/`str` **plus `bl __sync_synchronize`** | libcall |
| `exchange`, `fetch_add`, `fetch_sub`, `fetch_and/or/xor/nand` | libcall `__atomic_<op>_N` | libcall |
| `compare_exchange` | libcall `__atomic_compare_exchange_N` | libcall |
| `__sync_*` builtins | libcall `__sync_fetch_and_add_4`, `__sync_val_compare_and_swap_4` | libcall |

### Nothing on the link line defines them

Checked with `nm` against every archive and DSO of the recorded link
(`crates/symdev-build/src/driver/link.rs`): `libgcc.a`, `libsupc++.a`,
`epoc32/release/armv5/urel/usrt2_2.lib` (12 members, 18 defined symbols), `euser.dso`,
`dfpaeabi.dso`, `drtaeabi.dso`, `scppnwdl.dso`, `drtrvct2_2.dso`. **Zero hits** for any
`__atomic_*` or `__sync_*` name.

Proven by an actual link, not just by `nm`. An `int E32Main()` doing a `fetch_add`, a
`compare_exchange_n` and a seq_cst load on a `volatile` 32-bit global, linked with the
recorded argv:

```
main.cpp:(.text+0x10): undefined reference to `__atomic_fetch_add_4'
main.cpp:(.text+0x1e): undefined reference to `__atomic_compare_exchange_4'
main.cpp:(.text+0x22): undefined reference to `__sync_synchronize'
main.cpp:(.text+0x28): undefined reference to `__sync_synchronize'
```

Note the last two: on this line even an **acquire load** fails to link.

### The libcall ABI, read off the emitted code

Identical for GCC 12.1 and LLVM:

| Symbol | Arguments |
|---|---|
| `__atomic_load_N` | `(ptr, memorder)` |
| `__atomic_store_N` | `(ptr, value, memorder)` |
| `__atomic_exchange_N` | `(ptr, value, memorder)` → old |
| `__atomic_fetch_<op>_N` | `(ptr, value, memorder)` → old |
| `__atomic_compare_exchange_N` | `(ptr, expected_ptr, desired, success_memorder, failure_memorder)` → bool |
| `__sync_synchronize` | `()` |

`compare_exchange` takes **five** arguments: the `weak` flag of the builtin is not passed to
the library routine, although GCC's *builtin declaration* of the same name has six. memorder
values seen: 0 relaxed, 2 acquire, 3 release, 4 acq_rel, 5 seq_cst.

## 3. The same question for Rust

Pinned `nightly-2026-09-19` (rustc 1.100.0-nightly), `-Zbuild-std=core,alloc`,
`-Zjson-target-spec`, a scratch copy of `symbian-rs/targets/arm-symbian-e32.json` with
`max-atomic-width: 32` and `atomic-cas: true`.

- `core`, `alloc` and `compiler_builtins` build without complaint; the probe crate compiles.
- **LLVM is stricter than GCC: every atomic operation becomes a libcall, including a
  *relaxed* load and store.** `probe_load_relaxed` disassembles to `bl __atomic_load_4` with
  `r1 = 0`. The archive's complete undefined list is `__atomic_load_4`, `__atomic_store_4`,
  `__atomic_exchange_1`, `__atomic_fetch_add_4`, `__atomic_compare_exchange_4`.
- **`compiler_builtins` defines none of them** (its member in the archive has no
  `__atomic_*`/`__sync_*` symbol at all, defined or undefined).
- Linking that archive with the recorded argv produces seven
  `undefined reference to '__atomic_*'` errors.
- At the target's current `max-atomic-width: 0` the types simply do not exist:
  `no AtomicU32/AtomicU8/AtomicUsize in sync::atomic`, and `cannot find sync in alloc` —
  **no `Arc`**. With 32/true, `alloc::sync::Arc` exists.

So raising the width alone converts a compile error into a link error. It is not a fix by
itself; it is a fix exactly when the libcalls are defined.

### With a lock-backed shim, Rust atomics link and run

`shim.cpp` (`/tmp/claude-1000/atomics-work/shim.cpp`, ~110 lines): one file-static
`RFastLock`, an `atomic_shim_init()`, an RAII guard, and
`__atomic_{load,store,exchange,fetch_add,fetch_sub,fetch_and,fetch_or,fetch_xor,fetch_nand,compare_exchange}_{1,2,4,8}`
plus `__sync_synchronize`. Each definition needs an `__asm__("__atomic_…")` label: defining
the name directly gives `error: … ambiguates built-in declaration`.

Linked through `symdev build` (MMP `LIBRARY librprobe.a`, the Rust archive dropped into
`build/`, which symdev already puts on `-L`), the resulting 5423-byte EXE ran in EKA2L1:

```
init=0 store11 ld=11 lda=11 fadd=11 cas=12 now=30 swap8=0 usize=0
create=0 rust fetch_add 2x20000 = 40000
```

Every single-threaded result is correct, and Rust's `AtomicU32::fetch_add(1, SeqCst)` called
20000 times from each of two threads gives **exactly 40000**. The equivalent C++ probe adds
`compare_exchange`: a CAS loop from two threads won exactly 400 times out of 400, while the
hand-rolled plain RMW beside it lost half its updates.

**Hazard:** the shim degrades to non-atomic before `atomic_shim_init()` has run. In the real
slice the init must happen in the `_Z7E32Mainv` prologue, before any Rust code, or the shim
must fault loudly instead of silently returning a torn value.

**UNKNOWN:** whether `__sync_synchronize` needs to be more than a compiler barrier. ARMv5TE
has no `DMB` (ARMv6K and later), and the E52 is single-core, so a compiler barrier is all
that can be emitted — but that the OS never migrates a thread across a second core on this
device is not something this host can observe.

## 4. Blocking primitives

From `e32cmn.h` / `e32std.h`, with the runtime facts observed in EKA2L1. All of them are
handles onto kernel objects (`RHandleBase`), all creation calls return a `TInt` error code
and **none of them leave**; `Wait`/`Signal`/`Close` return `void` except where noted.

| Type | Creation | Scope | `sizeof` | Contention | Recursive? |
|---|---|---|---|---|---|
| `RFastLock` | `CreateLocal(TOwnerType)` only | process-local | 8 (`iHandle` + `iCount`) | header: "a layer over a standard semaphore, and only calls into the kernel side if there is contention" | **No** — a second `Wait` from the owning thread blocks forever (observed) |
| `RSemaphore` | `CreateLocal(count)`, `CreateGlobal(name,count)`, `OpenGlobal`, `Open(RMessagePtr2,…)` | local or **system-wide** by name | 4 | blocks in the kernel | counting, so N `Wait`s need N `Signal`s |
| `RMutex` | `CreateLocal()`, `CreateGlobal(name)`, `OpenGlobal`, `Open` | local or **system-wide** | 4 | blocks in the kernel; `IsHeld()` 0→1 across `Wait` (observed) | **Yes** — `Wait` twice from the owner returns (observed) |
| `RCriticalSection` | `CreateLocal()` only | process-local | 8 (`iHandle` + `iBlocked`) | privately a semaphore; `IsBlocked()` is 1 inside the section (observed) | UNKNOWN (not probed) |
| `RCondVar` | `CreateLocal()`, `CreateGlobal(name)`, `OpenGlobal`, `Open` | local or system-wide | 4 | `Wait(RMutex&)`, `TimedWait(RMutex&, us)`, `Signal()`, `Broadcast()` — all return `TInt` | n/a |

- **Timeouts / try-lock.** Only `RSemaphore::Wait(TInt aTimeoutMicroseconds)` and
  `RCondVar::TimedWait` take one. Observed: `Wait(50000)` on an empty semaphore returns
  **−33 = `KErrTimedOut`** (`e32err.h`); with a token available it returns 0. `RFastLock` and
  `RMutex` have **no** timed or trying variant, so a Rust `Mutex::try_lock` must be built on
  `RSemaphore` (or on the shim's atomics), not on `RMutex`.
- **Uncontended cost**, 100 000 iterations, `User::FastCounter()` at 32768 Hz
  (`HAL::Get(EFastCounterFrequency)`), inside EKA2L1 — **ratios only, this is not device
  timing**:

  | Loop body | ticks / 100 000 | relative |
  |---|---|---|
  | `g = g + 1` (plain, `volatile`) | 7 | 1× |
  | `User::LockedInc(i)` | 238 | 34× |
  | `RMutex::Wait` + `Signal` | 349 | 50× |
  | `RFastLock::Wait` + `Signal` | 523 | 75× |
  | shim `__atomic_fetch_add` (lock + RMW) | 641 | 92× |

  Note that in EKA2L1 `RMutex` is *cheaper* than `RFastLock`, which contradicts the header's
  claim that `RFastLock` stays out of the kernel when uncontended. Almost certainly an
  emulator artefact (its `RFastLock` fast path is unlikely to be modelled). **UNKNOWN** on a
  device; measure there before picking one on cost.
- **Priority inheritance** on `RMutex`: not observed, **UNKNOWN**.
- `RThread::Create(const TDesC& aName, TThreadFunction, TInt aStackSize, TInt aHeapMinSize,
  TInt aHeapMaxSize, TAny* aPtr, TOwnerType)` and the `RAllocator*` overload — a thread
  always gets an explicit stack size and either its own heap or a shared allocator. Joining
  is `Logon(TRequestStatus&)` + `User::WaitForRequest`. Observed working in EKA2L1
  (`create=0`, `ExitReason()` 0).

### Emulator caveats to carry forward

- After a worker `RThread` has run and exited, the main thread takes an
  `Access violation reading address 0x8000A4` within the next few instructions, on every
  probe that creates a thread; single-threaded probes never fault. The probe's own output is
  produced first, so the data above is intact. Cause unidentified — EKA2L1 or a real
  teardown bug, **UNKNOWN**.
- `RCondVar::CreateLocal()` returns `KErrNone` but `Handle()` is 0, unlike every other
  primitive (196610 / 262147 / 327684). Suspect EKA2L1 does not implement `RCondVar`;
  **UNKNOWN**, and a `Wait`/`Signal` round trip was not probed.
- EKA2L1 refuses to install a package whose executable is already on drive E — it logs
  `Installation done!` and then the front end prints `Installation of SIS failed`, and the
  `--run` never happens. Each probe run needs a fresh app name **and** a fresh UID3.

## 5. The conclusion a later slice needs

Availability, today (no shim) and with the shim of §3:

| Type | load | store | swap | `fetch_add` | `compare_exchange` |
|---|---|---|---|---|---|
| `AtomicBool`, `AtomicU8`/`I8` | lock | lock | lock | lock | lock |
| `AtomicU16`/`I16` | lock | lock | lock | lock | lock |
| `AtomicU32`/`I32`, `AtomicUsize`/`Isize`, `AtomicPtr` | lock | lock | lock | lock (±1 also via `User::LockedInc/Dec`) | lock |
| `AtomicU64`/`I64` | lock | lock | lock | lock | lock |

"lock" = available only through the `__atomic_*` shim over an `RFastLock`; **nothing in this
table is available inline, and only `TInt` ±1 is available as a euser call.** Without the
shim every cell is *not available*: a compile error at `max-atomic-width: 0`, a link error
above it.

Two refinements worth knowing:

- From **C++/GCC** a relaxed 8/16/32-bit load or store is genuinely inline, and only the
  barrier and the RMWs are libcalls. From **Rust/LLVM** even a relaxed load is a libcall, so
  on the Rust side there is no inline case at all.
- `User::LockedInc`/`LockedDec` are a real, observed-atomic 32-bit signed ±1 with no lock.
  A future `__atomic_fetch_add_4` could special-case ±1 through them; `SafeInc`/`SafeDec` are
  a ready-made "increment only if positive", which is exactly `Arc`'s upgrade-from-`Weak`
  step. Neither gives CAS, so the lock is still needed for the general case.

### Recommendation

- **`max-atomic-width`: 32, `atomic-cas`: true — but only in the change that also lands the
  shim in the link.** The evidence that the change is correct is §3: with the shim on the
  line, a `max-atomic-width: 32` / `atomic-cas: true` build of `core` links through the
  recorded argv and `AtomicU32::fetch_add` from two threads gives exactly 40000. The
  evidence that it must not land *earlier* is also §3: without the shim the same build
  produces seven undefined `__atomic_*` references. **The target JSON is therefore left at
  `0` / `false` by this experiment.** Not 64: the shim handles 8-byte fine, but a 64-bit
  atomic on a 32-bit target buys nothing that a lock does not already, and keeping the width
  at the pointer size keeps `AtomicU64` out of dependency code that would silently get a
  lock.
- **`Mutex`** → `RFastLock`. It is process-local (which is all a Rust `Mutex` needs), it is
  the primitive Symbian itself calls "fast", and it is not recursive, which matches Rust's
  contract that relocking from the same thread is a deadlock rather than UB. `try_lock`
  cannot use it — build `try_lock` on `RSemaphore::Wait(0 or small timeout)` (`KErrTimedOut`
  observed) or on the shim's `compare_exchange`. Revisit on real hardware if the cost table
  above reproduces there, since `RMutex` measured cheaper in the emulator.
- **`Once`** → the shim's `AtomicU32` `compare_exchange` for the state word plus one
  `RFastLock` for the waiters, i.e. the ordinary three-state `Once`. A lock-only `Once`
  (take the global lock, check a `bool`) also works and is simpler, but it serialises every
  `Once::call_once` in the process against every atomic in the process, because the shim has
  a single global lock. If that matters, give `Once` its own `RFastLock`.
- **Threads** → `RThread::Create` with an explicit stack size; join with
  `Logon` + `User::WaitForRequest`. No detached/`JoinHandle`-style semantics come for free.
- **`Arc` and `core::sync::atomic` stand or fall with the shim.** Today they do not exist:
  `alloc::sync` is not compiled at all, so `Arc` is unavailable and a large part of the
  ecosystem (anything with an `Arc`, a `OnceLock`, a `lazy_static`, most channel and logging
  crates) cannot be built. With the shim they exist and are correct, but every operation on
  them takes a process-global lock — `Arc::clone` becomes a `Wait`/`Signal` pair, roughly 90×
  a plain increment in the emulator. That is usable for correctness and unsuitable for a hot
  path. A single-threaded program pays this cost for nothing, so the shim should be paired
  with an explicit note that the phone-side SDK is single-threaded by default and that
  `Rc`/`RefCell` are the right tools until a program actually creates an `RThread`.

### What is still UNKNOWN

1. The memory ordering the four euser atomics guarantee — nothing in the headers says.
2. Whether they are atomic on real hardware (observed only in EKA2L1).
3. Whether `__sync_synchronize` must be more than a compiler barrier on the E52.
4. `RMutex` priority inheritance; `RCriticalSection` recursion.
5. Whether `RCondVar` works at all (EKA2L1 gives it handle 0).
6. The real uncontended cost of `RFastLock` vs `RMutex` (the emulator's ordering is
   suspicious).
7. The `Access violation` after a worker thread exits — emulator or real.

Each of 2, 3, 6 and 7 is settled by the same thing: running these probes on a stock E52.
Number 1 is settled by the Symbian Foundation source or a disassembly of a device
`euser.dll`; neither is on this host.
