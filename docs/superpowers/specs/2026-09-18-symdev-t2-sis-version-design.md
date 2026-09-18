# T2: SIS version triple (`SisVersion`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track without stopping; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 18.

## 1. Goal

A `SisVersion` value type that encodes a SIS type-4 version as three little-endian `u32` (major, minor, build) and wraps as a `SisField`. Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not arrays / UID / inflate

Experiment 18 pins type 4 to the experiment-7 `.pkg` triple `1,0,24`. Type-2 arrays, type-9 package UID, and in-tree inflate stay later slices.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisVersion { major, minor, build }` with methods. No Wine paths. `SisTools` stays the host process. |
| Payload | Three little-endian `u32` in that order. Length 12, already 4-byte aligned. |
| Field | `field()` returns `SisField::new(Self::KIND, self.payload().to_vec())` with `KIND = 4`. |
| Tests | Pinned hello `1,0,24` bytes from experiment 18. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisVersion` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisVersion {
    pub major: u32,
    pub minor: u32,
    pub build: u32,
}

impl SisVersion {
    pub const KIND: u32 = 4;

    pub fn new(major: u32, minor: u32, build: u32) -> Self;
    pub fn payload(&self) -> [u8; 12];
    pub fn field(&self) -> SisField;
}
```

`payload`: LE `major`, `minor`, `build`.

`field`: `SisField::new(Self::KIND, self.payload().to_vec())`.

Pinned: `SisVersion::new(1, 0, 24).payload()` equals `01 00 00 00 00 00 00 00 18 00 00 00`.
`field().header_bytes()` equals `04 00 00 00 0c 00 00 00`.

## 5. Non-goals

Type-2 arrays; type-9 UID; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
