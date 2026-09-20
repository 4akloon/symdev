# rust-time-76 — the time half of step 76

Task: give `symbian_std::time` a `std`-shaped `Instant`, `SystemTime`, `UNIX_EPOCH`
and `SystemTimeError` over a Symbian clock, proven in the emulator. TLS is out of scope.

## Findings

## Decisions

## Dead ends

## Next step

- Read the spec §6a and §11/76-77, experiments 78-79, and `symbian-std/src/io/error.rs` + `src/fs/`.

### 2026-09-20, headers

- `e32std.h:4511-4517`: `User::TickCount()->TUint`, `User::NTickCount()->TUint32`,
  `User::FastCounter()->TUint32`, all static, no doc comment in the SDK header.
- `e32std.h:1770` `class TTime`: "microseconds since midnight, January 1st, 0 AD nominal
  Gregorian", one `TInt64 iTime`; `__DECLARE_TEST` (e32def.h:2024) declares only member
  functions, so `sizeof(TTime) == 8` and it is a plain `i64` to Rust.
- `TTime::HomeTime()` / `TTime::UniversalTime()` are **non-static void members** —
  `_ZN5TTime8HomeTimeEv`, `_ZN5TTime13UniversalTimeEv` — so the experiment-78 member ABI
  (`this` in r0) applies and no shim is needed; neither can leave.
- `hal_data.h`: `ESystemTickPeriod` "time between system ticks, in microseconds",
  `ENanoTickPeriod` "time between nanokernel ticks, in microseconds",
  `EFastCounterFrequency`, `EFastCounterCountsUp` — the fast counter may count **down**.
- `e32hal.h:589` `UserHal::TickPeriod(TTimeIntervalMicroSeconds32&)->TInt`, non-leaving.
- Computed Unix epoch offset (proleptic Gregorian, 719 528 days from 0000-01-01 to
  1970-01-01): 62 167 219 200 s = **62_167_219_200_000_000 µs**. To be checked in the
  emulator, not trusted.

### 2026-09-20, measured in EKA2L1 (probe v1, `E:\symdev\time76\probe.txt`)

- `UserHal::TickPeriod` = **15 625 µs** (1/64 s), and `User::TickCount` moved exactly
  **64** across `User::After(1_000_000)` — the stated period and the measured one agree.
- `User::NTickCount` moved **1000** across a 1 s sleep, 101 across 100 ms, 10 across
  10 ms, 1 across 1 ms → **1 000 µs (1 kHz) in EKA2L1**. Wrap at 2^32 ticks = 49.7 days.
- `User::FastCounter` moved 33 337 across 1 s → ~33.3 kHz (~30 µs). Its frequency and
  its *direction* are `hal.dll` attributes, not on this link line.
- 4096 back-to-back reads: nano and system never moved, fast moved 8 times → a read is
  far cheaper than any of these periods.
- **Monotonic**: 32 768 `NTickCount` samples, **0** raw decreases, 0 wrapping decreases.
- `TTime::UniversalTime` granularity: smallest non-zero step **1 µs**.
- `HomeTime - UniversalTime` = **7 200 000 044 µs** ≈ +2 h (the emulator's time zone),
  which is why `SystemTime` must use `UniversalTime`.

### 2026-09-20, the epoch, from euser's own calendar (probe v2, `calendar.txt`)

**The computed proleptic-Gregorian offset was wrong by 12 days.** Asked of
`Time::LeapYearsUpTo`/`IsLeapYear` and `TTime`'s const accessors — real euser ROM code,
not EKA2L1 code:

- `IsLeapYear(100)=1`, `(400)=1`, `(1582)=0`, `(1600)=1`, `(1900)=0`, `(2000)=1`;
  `LeapYearsUpTo(100)=25`, `(400)=100`, `(1600)=400`, `(1970)=490`. So Symbian's
  calendar is **Julian before 1600 and Gregorian from 1600** — that is what "nominal
  Gregorian" in `e32std.h` means. 1970·365 + 490 = **719 540 days**, not 719 528.
- `TTime(62_168_256_000_000_000)` decodes as day 1 of the year, day 0 of the month,
  weekday 3 (= `EThursday`), 31 days in the month → **1970-01-01, a Thursday**. Correct.
- The 719 528-day candidate decodes as day 354, weekday 5 → 20 December, 12 days early.
- Cross-check: `UniversalTime` now decodes as day 263, day-of-month 19, weekday 6
  (Sunday), 30 days in the month = **2026-09-20, Sunday**. Correct.
- **`UNIX_EPOCH` = 62_168_256_000_000_000 µs**, established from euser, and EKA2L1's
  clock conversion agrees with it.

### 2026-09-20, the HAL route is closed (probe v3, `hal.txt`)

`hal.dll` is not on this SDK's link line, but euser exports
`_ZN7UserSvr6HalGetEiPv` — `UserSvr::HalGet(TInt, TAny*)` — which is what `HAL::Get`
calls. Tried with the `HALData::TAttribute` ordinals counted out of `hal_data.h`
(`ESystemTickPeriod` 14, `EMemoryRAM` 15, `ENanoTickPeriod` 92,
`EFastCounterFrequency` 93, `EFastCounterCountsUp` 94):

**every one returns KErrNotSupported (-5)** in EKA2L1, including attribute 14, whose
value `UserHal::TickPeriod` hands over quite happily. So the whole `HalGet` path is
unimplemented here and the nanokernel tick period is **not readable** on this platform.

**Decision: `Instant` is `User::TickCount` + `UserHal::TickPeriod`.** It is the only
counter whose period the platform will state through a call that works, so the
tick→`Duration` factor is read at run time on whatever board this lands on instead of
being an emulator measurement hard-coded as a device fact. The price is resolution:
15.625 ms instead of `NTickCount`'s 1 ms. The gain, besides honesty, is the wrap
window: 2^32 × 15.625 ms = **776.7 days (2.13 years)** against `NTickCount`'s 49.7 days.

**Decision: `Instant::now()` returns `io::Result<Instant>`.** `UserHal::TickPeriod`
returns a `TInt` and this crate may not panic, so the one std signature that has to
give is `now()`; `elapsed`, `duration_since`, `checked_*`, `Add` and `Sub` all keep
std's exact shapes. `Instant` carries the period it was taken with (8 bytes).

**Decision: no `Ord`/`PartialOrd` for `Instant`.** A 32-bit wrapping counter has no
total order; RFC 1982 serial comparison is not transitive, so `Ord` would be a lie.
`checked_duration_since` returning `None` is the honest local comparison, and `std`
already has it.

### 2026-09-20, the example passes and the sizes

- `symdev test --emulator` on `examples/time`: **29 passed**, exit 0. A 1 000 ms sleep
  measures 1 000 ms and a 500 ms one 500 ms exactly (64 and 32 ticks); a 100 ms sleep
  measures 93.75 or 109.375 ms — `User::After` rounds to a tick boundary — inside the
  stated ±32.25 ms (two ticks plus a millisecond).
- 20 000 instants over 218 ms: **0 backwards**, and the counter really moved **14**
  times. The first version of this test took its 20 000 samples inside a single tick
  and proved nothing, so "the counter moved" is now a case of its own.
- **A clock change is proven**: `User::SetUTCTime` works in EKA2L1, `SystemTime`
  jumped the full hour and the `Instant` measured 0 across it. Step 76's criterion.
- **Host skew**: the emulator's last `SystemTime` reading landed **50 ms and 51 ms**
  before the host's clock at the moment `symdev test` returned, on two back-to-back
  runs — and those 50 ms are the report write plus the emulator's shutdown, so the
  true skew is smaller and does not drift. EKA2L1's wall clock is the host's.
- Sizes: `hello` 3 187, `hello-raw` 752, `alloc` 4 320, `shim` **4 474**, `files`
  10 423, `time` **20 479**. Every one unchanged by this work; `shim` measures 4 474
  here with the change stashed as well, so the backlog's 4 475 is older, not this.
- **`time` costs 6.5 kB of soft float it never calls.** `Duration` arithmetic needs
  u64 division, `u64_div_rem` pulls the one `compiler_builtins` object, and that
  object brings `__adddf3`/`__divdf3`/`__muldf3` and their `sf` siblings with it —
  nothing in the image calls them but each other. Experiment 77's one-codegen-unit
  problem again. `files` has 0 `compiler_builtins` symbols, `time` has 22.
- Using `Duration::as_micros`/`as_millis` (`u128`) in the example cost a further
  **1 081 bytes**: 21 560 → 20 479 after switching to `as_secs`/`subsec_micros`.
