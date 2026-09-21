# 88 — `thread_local!` over Symbian's one kernel call (2026-09-21)

`tlsdemo.exe`, 16 272 bytes, is `symbian-rs/examples/tls`. `symdev test --emulator` on
it reports **45 passed**, exit 0: fifteen cases on the platform's own
`UserSvr::Dll*Tls`, twenty-seven on `symbian_std::thread_local!` above it, and three
that measure what one access costs.

**There is no `Dll` class in this SDK.** `grep -rn 'class Dll\b|Dll::Tls' epoc32/include/`
matches one *comment*; `e32std.h` does not contain the string `Tls`; no import library
of this SDK exports a `_ZN3Dll…` symbol. The whole TLS surface is five `@internalAll`
statics of `class UserSvr` in `e32svr.h` lines 39–43, and `symbian-sys::tls` calls them
directly — they take scalars, so no C++ shim comes into it.

What the probe established, rather than assumed:

| Question | Answer, observed in EKA2L1 |
|---|---|
| How many slots does an EXE get? | **Many.** 64 distinct handles held at once, no set failure. Not the "one slot per program" the design spec feared. |
| Keyed by what? | A `TInt` **the caller chooses**. `aHandle` is what `Dll::Tls` would have filled in with the calling DLL's code segment handle; an EXE has none and passes its own constant. |
| Per thread or per process? | **Per thread.** A worker read null where the creator had stored a value, its own store was visible to itself, and the creator's slot was untouched after the join. |
| Do the uid overloads mix with the plain ones? | **No**, in either direction — both cross reads came back null. The SDK uses the one-argument pair, which round-trips. |

The design that follows: **one** slot for the whole program
(`SYMBIAN_STD_TLS_HANDLE`), holding a per-thread table of
`(key, value, dropper)`, keyed by each `LocalKey` static's own **address** — so a key
needs no registration and therefore no atomic. Not one slot per key, although the
platform would allow it, because the kernel will not enumerate a thread's slots and
`Drop` at thread exit needs that list, and because the handle is unobserved on
hardware and one assumption is better than N.

Measured, emulator only:

| Operation | ns |
|---|---|
| one `thread_local!` access (`with`, value already initialised) | **100** |
| the bare `UserSvr::DllTls` kernel call under it | 53 |
| `AtomicU32::fetch_add` beside it | 158 |

So a thread-local is the **cheap** way to hold per-thread state on this device, not
the expensive one: every atomic here is a `Wait`/`Signal` pair on a process-wide
`RFastLock` (experiment 80), and one kernel call is less than two.

Destructors **run**, at the end of every thread `symbian_std::thread::spawn` created:
the worker's value was dropped when it ended and the creator's survived. The main
thread has no hook below `symbian-std`, so it calls
`thread::drop_thread_locals()` itself; step 77's `lang_start` is where that call
belongs. After a sweep the slot holds a sentinel, so a later access is `KErrDied` and
not a fresh value nothing would ever drop — which is `std`'s contract.

The value lives in a `Box` on the **one** process heap: `spawn` switches a worker onto
the creator's allocator as its first instruction (experiment 80), so a thread-local
created on one thread and dropped on another goes back to the heap it came from.

Sizes, against a baseline built from main's own `symbian-rs` in the same tree:
`hello` 3 187, `hello-raw` 752, `alloc` 4 474, `shim` 4 474, `files` 10 552, `time`
20 583, `net` 13 322 — all unchanged. The one that moved is **`atomics` 11 582 →
11 719, +137**: the only program besides this one that spawns a thread, and what it
pays for is the thread-exit sweep in `spawn`'s trampoline.

Experiment record: `docs/research/experiment-backlog.md` §88.
