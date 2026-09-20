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
