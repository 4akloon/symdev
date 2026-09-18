# T2: SIS controller body (`SisController`)

Date: 2026-09-18
Status: approved for SDD (user: continue T2; type 13 after type 28; type 13 is the inflated blob).

Cites: [2026-09-18-symdev-t2-sis-files-design.md](2026-09-18-symdev-t2-sis-files-design.md); [2026-09-18-symdev-t2-sis-info-design.md](2026-09-18-symdev-t2-sis-info-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 31.

## 1. Goal

A value type that encodes the frozen hello inflated type-13 field by concatenating already-encoded children, including real type-28 `SisFiles` (not opaque bytes). Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 12 / inflate / checksums

Experiment 16’s type-3 zlib inflates to 548 bytes: one type-13 field, unpadded payload `n=540`. Type 12 (`SisField::CONTROLLER`) is the outer SIS wrapper and stays a field constant. This slice does not compress, does not add a zlib crate, and does not encode types 34/35/30.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisController` with methods. No Wine paths. No clock. |
| Kind | Type `13`. Unpadded payload `n=540`. Field 548 bytes (no extra pad: `540 % 4 == 0`). |
| Name | `SisController` is KIND 13. Outer type 12 remains `SisField::CONTROLLER`. |
| Children | In order: `SisInfo`, `SisWords16`, `SisLanguages`, `SisProducts`, `SisWords19`, `SisFiles`, `SisU32(0)`. Concatenate each child's `field().bytes()`. No extra trailing zeros. |
| Type 28 | Reuse `SisFiles` from the files slice. Do not substitute a byte blob. |
| Tests | Pinned experiment 31 `field().bytes()` equals the 548-byte inflated hello blob, copied into the test as a const array. No Wine. Do not commit or read `.sis` / `.sisx`. Do not generate “now”. |
| Package | Do not call `SisController` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |
| Out of slice | Checksums 34/35, type 30 data, native signsis, type-3 compress, inflate crate. |

## 4. Interface

```rust
pub struct SisController {
    pub info: SisInfo,
    pub words16: SisWords16,
    pub languages: SisLanguages,
    pub products: SisProducts,
    pub words19: SisWords19,
    pub files: SisFiles,
    pub trailer: SisU32,
}

impl SisController {
    pub const KIND: u32 = 13;
    pub fn new(
        info: SisInfo,
        words16: SisWords16,
        languages: SisLanguages,
        products: SisProducts,
        words19: SisWords19,
        files: SisFiles,
        trailer: SisU32,
    ) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`payload`: concatenate `[info, words16, languages, products, words19, files, trailer].map(|x| x.field().bytes())`.

`field`: `SisField::new(Self::KIND, self.payload())`.

Hello constructor for tests uses the already-pinned child constructors (info experiment 22 datetime `2026-08-17` 0-based month / `15:18:24`; files experiment 29–30; products experiment 28; words `0x21` / `0x14`; languages id `1`; trailer `SisU32::new(0)`).

Pinned header: `0d 00 00 00 1c 02 00 00` (type 13, length 540). Full 548-byte `field().bytes()` is the experiment-31 const in the unit test (copied from the host inflate; not read from disk at test time).

Child occupied sizes inside the payload: 160 + 20 + 28 + 116 + 20 + 184 + 12 = 540.

## 5. Non-goals

Type 12 outer controller; type-3 zlib; types 34/35 checksums; type 30 data; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
