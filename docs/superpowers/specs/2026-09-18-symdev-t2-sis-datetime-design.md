# T2: SIS date and time (`SisDate`, `SisTime`, `SisDateTime`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 21.

## 1. Goal

Value types that encode the frozen hello type-8 stamp: type 6 date, type 7 time, type 8 as those two fields concatenated. Default tests never spawn Wine, never inflate, never call a clock, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not “now” / type 14

Experiment 21 pins the bytes on the 2026-09-17 hello.sis. North-star later normalizes SIS creation time before byte-compare; this slice only encodes a recorded stamp. Type-14 SISInfo compose stays next.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisDate`, `SisTime`, `SisDateTime` with methods. No Wine paths. No time crate. |
| Date | Type `6`. Payload: `u16` year LE, `u8` month **0-based**, `u8` day. Hello: year `2026`, month `8`, day `17`. |
| Time | Type `7`. Payload: three `u8` hour, minute, second (length 3; `SisField` pads). Hello: `15`, `18`, `24`. |
| DateTime | Type `8`. Payload: `date.field().bytes()` then `time.field().bytes()`. No extra prefix. |
| Clock | Do not read the host clock. Tests pass the recorded numbers. |
| Tests | Pinned experiment 21 bytes. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl SisDate {
    pub const KIND: u32 = 6;
    pub fn new(year: u16, month: u8, day: u8) -> Self;
    pub fn payload(&self) -> [u8; 4];
    pub fn field(&self) -> SisField;
}

pub struct SisTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl SisTime {
    pub const KIND: u32 = 7;
    pub fn new(hour: u8, minute: u8, second: u8) -> Self;
    pub fn payload(&self) -> [u8; 3];
    pub fn field(&self) -> SisField;
}

pub struct SisDateTime {
    pub date: SisDate,
    pub time: SisTime,
}

impl SisDateTime {
    pub const KIND: u32 = 8;
    pub fn new(date: SisDate, time: SisTime) -> Self;
    pub fn field(&self) -> SisField;
}
```

`SisDate::payload`: year LE, then month, then day.

`SisTime::payload`: `[hour, minute, second]`.

`SisDateTime::field`: `SisField::new(Self::KIND, [date.field().bytes(), time.field().bytes()].concat())`.

Pinned:

- `SisDate::new(2026, 8, 17).payload()` equals `ea 07 08 11`.
- `SisTime::new(15, 18, 24).payload()` equals `0f 12 18`.
- `SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)).field().bytes()` equals

`08 00 00 00 18 00 00 00 06 00 00 00 04 00 00 00 ea 07 08 11 07 00 00 00 03 00 00 00 0f 12 18 00`

## 5. Non-goals

Generating current time; type-14 compose; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5; chrono/`time` crates.
