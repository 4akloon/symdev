# T2: SIS field array (`SisArray`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track without stopping; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md); [2026-09-18-symdev-t2-sis-string-design.md](2026-09-18-symdev-t2-sis-string-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 20.

## 1. Goal

A `SisArray` value type that encodes a SIS type-2 field whose payload is the concatenation of child `SisField::bytes()`. Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not inflate / SISInfo

Experiment 20 pins type 2 as concatenated nested TLVs (the hello names array). Composing a full type-14 SISInfo is a later slice.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisArray { items: Vec<SisField> }` with methods. No Wine paths. |
| Payload | `items.iter().flat_map(|f| f.bytes())`. No element-count prefix. Children already include their own 4-byte padding. |
| Field | `field()` returns `SisField::new(Self::KIND, self.payload())` with `KIND = 2`. |
| Tests | Pinned hello names array from experiment 20. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisArray` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisArray {
    pub items: Vec<SisField>,
}

impl SisArray {
    pub const KIND: u32 = 2;

    pub fn new(items: Vec<SisField>) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

Pinned: `SisArray::new(vec![SisString::new("hello").field()]).field().bytes()` equals

`02 00 00 00 14 00 00 00 01 00 00 00 0a 00 00 00 68 00 65 00 6c 00 6c 00 6f 00 00 00`

(type 2, length 20, then the type-1 hello field including pad).

## 5. Non-goals

Full SISInfo; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
