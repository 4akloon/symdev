# T2: SIS package UID field (`SisPkgUid`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track without stopping; types + methods; value types do not know Wine).

Cites: [2026-09-17-symdev-t2-sis-uid-design.md](2026-09-17-symdev-t2-sis-uid-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 19.

## 1. Goal

A `SisPkgUid` value type that encodes a SIS type-9 package UID as one little-endian `u32` and wraps as a `SisField`. Distinct from `SisUid` (16-byte file header). Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not arrays / inflate / file SisUid

Experiment 19 pins type 9 to hello UID3 `0xe79e4cf9`. `SisUid` remains the SIS file prefix (UID1/UID2/package/checked). Type-2 arrays and in-tree inflate stay later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisPkgUid { uid }` with methods. No Wine paths. Do not put Wine on `SisUid` either. |
| Payload | One little-endian `u32`. Length 4, already 4-byte aligned. |
| Field | `field()` returns `SisField::new(Self::KIND, self.payload().to_vec())` with `KIND = 9`. |
| Tests | Pinned hello `0xe79e4cf9` bytes from experiment 19. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisPkgUid` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisPkgUid {
    pub uid: u32,
}

impl SisPkgUid {
    pub const KIND: u32 = 9;

    pub fn new(uid: u32) -> Self;
    pub fn payload(&self) -> [u8; 4];
    pub fn field(&self) -> SisField;
}
```

`payload`: `uid.to_le_bytes()`.

`field`: `SisField::new(Self::KIND, self.payload().to_vec())`.

Pinned: `SisPkgUid::new(0xe79e_4cf9).payload()` equals `f9 4c 9e e7`.
`field().header_bytes()` equals `09 00 00 00 04 00 00 00`.

## 5. Non-goals

Type-2 arrays; in-tree inflate; rewriting `SisUid`; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
