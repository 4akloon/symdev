# T2: SIS words (`SisWords`, `SisWords16`, `SisWords19`, `SisU32`)

Date: 2026-09-18
Status: approved for SDD (user: independent T2 slice; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-languages-design.md](2026-09-18-symdev-t2-sis-languages-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiments 25–27.

## 1. Goal

Value types that encode three leftover hello type-13 siblings: type 16 and type 19 each wrap a type-2 whose payload is concatenated little-endian `u32` words (not `SisArray`), and type 40 is one little-endian `u32`. Default tests never spawn Wine, never inflate, and never read `.sis` / `.pkg` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not languages / product list / type 13

Experiment 23 already pinned type 11 / 15 (`&EN` id `1`). Type 16’s word `0x21` is not that language id and is not a language table. Type 17 (product list) is a parallel slice; this slice does not touch `sis/product.rs`. Type-13 compose stays later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisWords`, `SisWords16`, `SisWords19`, `SisU32` with methods. No Wine paths. |
| Words | Type `2`. Payload: each `u32` as `to_le_bytes()`, concatenated. **Not** `SisArray` (that concatenates `SisField::bytes()`). Hello names used field-concat; these goldens use raw u32 concat. |
| Type 16 | Wraps `words.field().bytes()`. Hello: one word `0x21`. Do not name this Options / Languages. Do not treat `0x21` as the EN language id. |
| Type 19 | Same wrap. Hello: one word `0x14`. Do not invent a C / Wine name. |
| Type 40 | One LE `u32`. Hello: `0`. Not nested in type 2. |
| Tests | Pinned experiment 25–27 bytes. No Wine. Do not commit or read `.sis` / `.sisx` / `.pkg`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. No invented C names or language tables. |
| Merge | `sis/mod.rs` and crate-root `lib.rs` diffs are **append-only** exports. Do not touch `sis/product.rs`. |

## 4. Interfaces

```rust
pub struct SisWords {
    pub values: Vec<u32>,
}

impl SisWords {
    pub const KIND: u32 = 2;
    pub fn new(values: Vec<u32>) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}

pub struct SisWords16 {
    pub words: SisWords,
}

impl SisWords16 {
    pub const KIND: u32 = 16;
    pub fn new(words: SisWords) -> Self;
    pub fn field(&self) -> SisField;
}

pub struct SisWords19 {
    pub words: SisWords,
}

impl SisWords19 {
    pub const KIND: u32 = 19;
    pub fn new(words: SisWords) -> Self;
    pub fn field(&self) -> SisField;
}

pub struct SisU32 {
    pub value: u32,
}

impl SisU32 {
    pub const KIND: u32 = 40;
    pub fn new(value: u32) -> Self;
    pub fn payload(&self) -> [u8; 4];
    pub fn field(&self) -> SisField;
}
```

`SisWords::payload` concatenates `value.to_le_bytes()` for each word.

`SisWords::field` is `SisField::new(Self::KIND, self.payload())`.

`SisWords16::field` is `SisField::new(Self::KIND, self.words.field().bytes())`.

`SisWords19::field` is `SisField::new(Self::KIND, self.words.field().bytes())`.

`SisU32::payload` is `self.value.to_le_bytes()`.

`SisU32::field` is `SisField::new(Self::KIND, self.payload().to_vec())`.

Hello:

- `SisWords::new(vec![0x21]).payload()` equals `21 00 00 00`.
- `SisWords16::new(SisWords::new(vec![0x21])).field().bytes()` equals

`10 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 21 00 00 00`

- `SisWords19::new(SisWords::new(vec![0x14])).field().bytes()` equals

`13 00 00 00 0c 00 00 00 02 00 00 00 04 00 00 00 14 00 00 00`

- `SisU32::new(0).field().bytes()` equals

`28 00 00 00 04 00 00 00 00 00 00 00`

## 5. Non-goals

Type 17 product list; type 13 compose; type 28; language-name / Options tables; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
