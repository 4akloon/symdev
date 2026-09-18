# T2: SIS languages (`SisLanguage`, `SisLanguages`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-info-design.md](2026-09-18-symdev-t2-sis-info-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 23.

## 1. Goal

Value types that encode the frozen hello type-15 language list: type 11 is one little-endian language word, type 15 wraps a type-2 array of those fields. Default tests never spawn Wine, never inflate, and never read `.sis` / `.pkg` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 16 / type 13

Experiment 23 pins type 11 payload `1` against `hello.pkg` `&EN`. Type 16’s word `0x21` does not match that id and stays later. Type-13 compose stays later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisLanguage`, `SisLanguages` with methods. No Wine paths. |
| Language | Type `11`. Payload: one `u32` LE. Hello: `1`. Do not invent a language-name table. |
| List | Type `15`. Payload: `SisArray::new(languages.map(SisLanguage::field)).field().bytes()`. |
| Tests | Pinned experiment 23 bytes. No Wine. Do not commit or read `.sis` / `.sisx` / `.pkg`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisLanguage {
    pub id: u32,
}

impl SisLanguage {
    pub const KIND: u32 = 11;
    pub fn new(id: u32) -> Self;
    pub fn payload(&self) -> [u8; 4];
    pub fn field(&self) -> SisField;
}

pub struct SisLanguages {
    pub languages: SisArray,
}

impl SisLanguages {
    pub const KIND: u32 = 15;
    pub fn new(languages: SisArray) -> Self;
    pub fn field(&self) -> SisField;
}
```

`SisLanguage::payload` is `id.to_le_bytes()`.

`SisLanguages::field` is `SisField::new(Self::KIND, self.languages.field().bytes())`.

Hello:

- `SisLanguage::new(1).payload()` equals `01 00 00 00`.
- `SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])).field().bytes()` equals

`0f 00 00 00 14 00 00 00 02 00 00 00 0c 00 00 00 0b 00 00 00 04 00 00 00 01 00 00 00`

## 5. Non-goals

Type 16 (`0x21`); type 13 compose; types 17/19/28/40; language-name tables; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
