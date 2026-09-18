# T2: SIS product (`SisProductVersion`, `SisProduct`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-languages-design.md](2026-09-18-symdev-t2-sis-languages-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 24.

## 1. Goal

Value types that encode the frozen hello type-18 product: type 5 wraps one type-4 version `0,0,0`; type 18 concatenates pkg UID, that wrap, and a names array `S60ProductID`. Default tests never spawn Wine, never inflate, and never read `.sis` / `.pkg` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 17 / type 13

Experiment 24 pins the pkg line `[0x102752AE], 0, 0, 0, {"S60ProductID"}` as one type-18 field. Type 17 wraps an array of these plus a type-2 word `0x12` and stays next. Do not invent a second (to) version: the golden has one nested type 4.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisProductVersion`, `SisProduct` with methods. No Wine paths. |
| Version wrap | Type `5`. Payload: `version.field().bytes()`. Hello: `SisVersion::new(0, 0, 0)`. |
| Product | Type `18`. Payload: `uid.field().bytes()` then `version.field().bytes()` then `names.field().bytes()`. |
| UID | Reuse `SisPkgUid` (type 9). Hello: `0x102752AE`. |
| Names | Reuse `SisArray` of `SisString::new("S60ProductID")`. |
| Tests | Pinned experiment 24 bytes. No Wine. Do not commit or read `.sis` / `.sisx` / `.pkg`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisProductVersion {
    pub version: SisVersion,
}

impl SisProductVersion {
    pub const KIND: u32 = 5;
    pub fn new(version: SisVersion) -> Self;
    pub fn field(&self) -> SisField;
}

pub struct SisProduct {
    pub uid: SisPkgUid,
    pub version: SisProductVersion,
    pub names: SisArray,
}

impl SisProduct {
    pub const KIND: u32 = 18;
    pub fn new(uid: SisPkgUid, version: SisProductVersion, names: SisArray) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`SisProductVersion::field` is `SisField::new(Self::KIND, self.version.field().bytes())`.

`SisProduct::payload` concatenates `uid`, `version`, and `names` `.field().bytes()`.

`SisProduct::field` is `SisField::new(Self::KIND, self.payload())`.

Hello constructor for tests:

```rust
SisProduct::new(
    SisPkgUid::new(0x1027_52ae),
    SisProductVersion::new(SisVersion::new(0, 0, 0)),
    SisArray::new(vec![SisString::new("S60ProductID").field()]),
)
```

Pinned `SisProductVersion::new(SisVersion::new(0, 0, 0)).field().bytes()`:

`05 00 00 00 14 00 00 00 04 00 00 00 0c 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00`

Pinned `SisProduct` `field().bytes()` (88 bytes):

`12 00 00 00 50 00 00 00 09 00 00 00 04 00 00 00 ae 52 27 10 05 00 00 00 14 00 00 00 04 00 00 00 0c 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 20 00 00 00 01 00 00 00 18 00 00 00 53 00 36 00 30 00 50 00 72 00 6f 00 64 00 75 00 63 00 74 00 49 00 44 00`

## 5. Non-goals

Type 17 list; type 16 (`0x21`); types 19/28/40; type 13 compose; a second version word; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
