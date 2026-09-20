# 80 — atomics, Arc, Mutex, Once and threads on a CPU with no atomic instruction (2026-09-20)

`atomicsdemo.exe`, 11 582 bytes, is `symbian-rs/examples/atomics`. ARMv5TE has no
`LDREX`/`STREX`, so every `core::sync::atomic` operation in this image is a call into
the SDK's compiler-runtime archive and a kernel `Wait`/`Signal` pair on one process-wide
`RFastLock`. `symdev test --emulator`, 23 cases, all passing:

```
ok   no atomic lock before the first atomic
ok   one atomic operation creates the lock
ok   and it is a real kernel handle
ok   fetch_add returns the old value
ok   compare_exchange takes on a match
ok   and reports what was there on a miss
ok   Arc clones / and counts strong references / and drops one
ok   the static Mutex locks
ok   try_lock on a held Mutex times out
ok   a thread returns its value
ok   fetch_add from two threads loses nothing: 4000 of 4000
ok   a load-then-store beside it does lose updates: 2000 of 4000
ok   a static Mutex counts every increment: 4000 of 4000
ok   an Arc<Mutex<_>> counts every increment: 4000 of 4000
ok   Once ran exactly once for both threads: 1 runs
ok   the heap still allocates after a thread has exited
atomicsdemo: 23 passed
```

The two counts that matter are the pair. **4000 of 4000** is 2 000 `fetch_add`s from
each of two threads with nothing lost. **2000 of 4000** is a load, a yield and a store
on a second counter in the same loop, losing exactly half — the control that says the
threads really did interleave and that the first number is a property of the shim, not
of a test that never raced.

Three things in this image were not true before it:

- **`Arc` exists.** At `max-atomic-width: 0` `alloc::sync` is not compiled at all. The
  target now says `32` and `atomic-cas: true`, which is only correct because the
  archive is on the link line.
- **The lock is created by `User::LockedInc`**, the one atomic euser exports, on first
  use — `no atomic lock before the first atomic` then `one atomic operation creates the
  lock` is that bootstrap being observed from inside the program.
- **A thread can exit without taking the heap with it.** The worker is created with a
  heap of its own and switches to the creator's as its first instruction; created the
  other way round, the creator's next allocation faults (experiment 80, the access
  violation experiment 72 recorded as unexplained).

Sizes after this step: `hello` **3 187** and `hello-raw` **752** unchanged, `shim`
**4 475** unchanged, `alloc` 4 320 → **4 474** and `files` 10 423 → **10 552**. The two
that moved are the two that allocate: +154 and +129 bytes for the lock the heap takes
once a program has a second thread. The atomics archive itself costs nothing to a
program that does not use it.

Experiment record: `docs/research/experiment-backlog.md` §80.
