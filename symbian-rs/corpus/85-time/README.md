# 85 — `symbian_std::time`: a monotonic clock and a wall clock (2026-09-20)

`timedemo.exe`, 20 583 bytes, is `symbian-rs/examples/time`. `symdev test --emulator`
on it reports **29 passed**, exit 0. Almost every name in the source is `std`'s —
`Duration`, `Instant`, `SystemTime`, `UNIX_EPOCH`, `duration_since`, `elapsed`,
`checked_add` — and the Symbian-shaped types appear only where `std` has no word for
the thing.

Which clock is which, and why:

| | Counter | Period | Wraps after |
|---|---|---|---|
| `Instant` | `User::TickCount` | `UserHal::TickPeriod`, **15 625 µs** measured | 776.7 days |
| `SystemTime` | `TTime::UniversalTime` | **1 µs** measured | ±292 471 years |

`User::NTickCount` is finer (1 000 µs measured) and is **not** used: its period is the
HAL attribute `ENanoTickPeriod`, `hal.dll` is not on this SDK's link line, and euser's
own `UserSvr::HalGet` answers `KErrNotSupported` for every attribute inside EKA2L1 —
so 1 kHz would be an emulator measurement standing in for a device fact.

What the run showed:

- A 1 000 ms `User::After` measures **1 000 ms** and a 500 ms one **500 ms** (64 and
  32 ticks exactly); a 100 ms one measures 93.75 or 109.375 ms, because `User::After`
  rounds to a tick boundary. Stated tolerance ±32.25 ms — two ticks and a millisecond.
- 20 000 instants over 218–234 ms: **0 backwards**, and the counter moved 14–15 times, so
  the check is not the vacuous one the first version of this example ran.
- **A device clock change**: `User::SetUTCTime` moved the wall clock a full hour,
  `SystemTime` moved with it and the `Instant` measured 0 across it.
- **The epoch is 62 168 256 000 000 000 µs**, established from euser's own calendar and
  not computed: Symbian is Julian before 1600 and Gregorian from 1600, so proleptic
  Gregorian arithmetic is 12 days wrong.
- The emulator's clock is the host's, within 50–78 ms including teardown.

Sizes unchanged by this step, measured against a baseline built from main's own
`symbian-rs`: `hello` 3 187, `hello-raw` 752, `alloc` 4 474, `shim` 4 474, `files`
10 552, `atomics` 11 582. Of `time`'s own 20 583, **7 556 bytes are
`compiler_builtins` and 6 124 of those are soft float that nothing in the image
calls**: `Duration` needs u64 division, `u64_div_rem` pulls the one
`compiler_builtins` object, and the float routines come with it. `files` and
`atomics` have 0 `compiler_builtins` symbols; `time` has 22.

Experiment record: `docs/research/experiment-backlog.md` §85.
