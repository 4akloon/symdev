# T2: SIS info (`SisInfo`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-datetime-design.md](2026-09-18-symdev-t2-sis-datetime-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 22.

## 1. Goal

A value type that composes the frozen hello type-14 block from existing T2 leaves. Default tests never spawn Wine, never inflate, never read a clock, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 13 / inflate

Experiments 17–21 already pin the six child encodings. Experiment 22 pins their order inside type 14 and two extra zero bytes inside the unpadded length. Type-13 siblings, in-crate inflate, checksums, and `package` stay later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisInfo` with methods. No Wine paths. No clock. |
| Kind | Type `14`. |
| Children | In order: `uid: SisPkgUid`, `vendor: SisString`, `names: SisArray`, `vendor_names: SisArray`, `version: SisVersion`, `datetime: SisDateTime`. |
| Payload | Concatenate each child's `field().bytes()`, then two extra `0x00` bytes as recorded (hello children total 148; unpadded length is 150). Those two zeros are **not** `SisField` padding. |
| Stamp | Tests pass the experiment-21 datetime (`SisDate::new(2026, 8, 17)`, `SisTime::new(15, 18, 24)`). Do not read the host clock. Month is 0-based. |
| Tests | Pinned experiment 22 `field().bytes()`. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisInfo` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interface

```rust
pub struct SisInfo {
    pub uid: SisPkgUid,
    pub vendor: SisString,
    pub names: SisArray,
    pub vendor_names: SisArray,
    pub version: SisVersion,
    pub datetime: SisDateTime,
}

impl SisInfo {
    pub const KIND: u32 = 14;
    pub fn new(
        uid: SisPkgUid,
        vendor: SisString,
        names: SisArray,
        vendor_names: SisArray,
        version: SisVersion,
        datetime: SisDateTime,
    ) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`payload`: `[uid, vendor, names, vendor_names, version, datetime].map(|x| x.field().bytes()).concat()` then `[0, 0]`.

`field`: `SisField::new(Self::KIND, self.payload())`.

Hello constructor for tests:

```rust
SisInfo::new(
    SisPkgUid::new(0xe79e_4cf9),
    SisString::new("Vendor"),
    SisArray::new(vec![SisString::new("hello").field()]),
    SisArray::new(vec![SisString::new("Vendor-EN").field()]),
    SisVersion::new(1, 0, 24),
    SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
)
```

Pinned `field().bytes()` (160 bytes: header `0e 00 00 00 96 00 00 00`, 150-byte payload, two pad zeros):

```
0e 00 00 00 96 00 00 00
09 00 00 00 04 00 00 00 f9 4c 9e e7
01 00 00 00 0c 00 00 00 56 00 65 00 6e 00 64 00 6f 00 72 00
02 00 00 00 14 00 00 00 01 00 00 00 0a 00 00 00 68 00 65 00 6c 00 6c 00 6f 00 00 00
02 00 00 00 1c 00 00 00 01 00 00 00 12 00 00 00 56 00 65 00 6e 00 64 00 6f 00 72 00 2d 00 45 00 4e 00 00 00
04 00 00 00 0c 00 00 00 01 00 00 00 00 00 00 00 18 00 00 00
08 00 00 00 18 00 00 00 06 00 00 00 04 00 00 00 ea 07 08 11 07 00 00 00 03 00 00 00 0f 12 18 00
00 00
00 00
```

(The first `00 00` after datetime is inside length 150; the last `00 00` is `SisField` pad.)

## 5. Non-goals

Type 13 compose; types 16/15/17/19/28/40; in-tree inflate; checksums 34/35; type 30 data; generating current time; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5; chrono/`time` crates.
