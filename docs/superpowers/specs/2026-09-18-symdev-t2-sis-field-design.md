# T2: SIS field TLV (`SisField`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-17-symdev-t2-sis-uid-design.md](2026-09-17-symdev-t2-sis-uid-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 15.

## 1. Goal

A `SisField` value type that encodes one SIS type+length field with 4-byte payload padding, matching experiment 15 goldens. Default tests never spawn Wine and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not full native makesis

Experiment 15 pins the TLV frame around the controller. Compressed controller, file data, checksums, and signatures stay later slices. `SisUid` stays the 16-byte file prefix; `SisField` is the next unit.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisField { kind, payload }` with methods. No Wine paths. `SisTools` stays the host process. |
| Header | Little-endian `u32` type then `u32` length (`payload.len()`, not padded size). |
| Padding | After payload, zero-fill so payload+pad is a multiple of 4: pad = `(4 - (len % 4)) % 4`. Next field at `8 + len + pad`. |
| Controller type | Associated constant `CONTROLLER = 12` as recorded. Do not invent a full type enum this slice. |
| Tests | Pinned outer headers and the 2-byte type-34 field from experiment 15. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisField` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisField {
    pub kind: u32,
    pub payload: Vec<u8>,
}

impl SisField {
    pub const CONTROLLER: u32 = 12;

    pub fn new(kind: u32, payload: Vec<u8>) -> Self;
    pub fn header_bytes(&self) -> [u8; 8];
    pub fn bytes(&self) -> Vec<u8>;
}
```

`header_bytes`: `kind` and `payload.len() as u32`, each little-endian.

`bytes`: `header_bytes` then `payload` then `0u8` padding to a 4-byte boundary.

Pinned:

- `SisField::new(SisField::CONTROLLER, vec![0; 3976]).header_bytes()` equals `0c 00 00 00 88 0f 00 00` (hello SIS).
- `SisField::new(SisField::CONTROLLER, vec![0; 5148]).header_bytes()` equals `0c 00 00 00 1c 14 00 00` (hello SISX). Dummy zero payloads are only for length; do not claim they match the controller body.
- `SisField::new(34, vec![0x5c, 0x9e]).bytes()` equals `22 00 00 00 02 00 00 00 5c 9e 00 00`.

## 5. Non-goals

Nested parse; compressed controller; data checksums; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim.
