# 87 — `async`/`await` on the active scheduler (2026-09-21)

`asyncdemo.exe`, 21 659 bytes, is `symbian-rs/examples/async`. It awaits `RTimer`s
through `symbian_async`: a single-threaded executor whose `CActive` lives in
`shims/common/symrs_active.cpp` and whose wakers are ordinary `Arc`s. Through
`symdev test --emulator`:

```
asyncdemo: 15 passed
```

**The criterion of step 73, measured inside the emulator** and written to
`E:\symdev\async73\measured.txt` with `symbian_std::time::Instant`:

| What | Measured |
|---|---|
| one 300 ms sleep | **312 ms**, 328 on a second run |
| two 300 ms sleeps **awaited together** | **312 ms** |
| the same two, one after the other | **625 ms** |
| a race between 100 ms and 20 s | **109 ms** |
| twenty install/run/uninstall rounds of a scheduler | 312 ms |

312 against 625 is the whole claim: both requests were outstanding at once, which is
what a `CActiveScheduler` is for and what a blocking `User::WaitForRequest` cannot do.
The 15.625 ms system tick (experiment 85) is why 300 reads as 312. The race figure is
the cancellation path — the loser's `RTimer` was cancelled through `CActive::Cancel`
rather than waited out.

Also proved from inside the emulator, each as its own case:

- **Order.** Of a concurrent 400 ms and 100 ms pair, the 100 ms one records itself
  first: `[100, 400]`.
- **The joined shape.** A `spawn`ed task sleeping 100 ms completes while the root
  future sleeps 300 ms, on the scheduler `block_on` installed — which is the same code
  path an Avkon application takes onto CONE's.
- **The refusals**, all four observed rather than argued: `spawn` with no scheduler is
  `KErrNotReady (-18)`; a nested `block_on` is `KErrInUse (-14)` instead of a second
  `CActiveScheduler::Install` panic; `symbian_core::net::blocking` under a scheduler is
  `KErrInUse (-14)` instead of consuming an active object's completion; a sleep longer
  than `RTimer::After`'s `TInt` of microseconds is `KErrOverflow (-9)`.

No network, no peer and no clock setting: one `RTimer` handle is the whole of what this
image needs from the platform beyond `euser`.
