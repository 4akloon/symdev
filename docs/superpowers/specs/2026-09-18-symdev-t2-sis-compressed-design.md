# T2: SIS compressed prefix (`SisCompressed`)

Date: 2026-09-18
Status: approved for SDD (user: continue T-track; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 16.

## 1. Goal

A `SisCompressed` value type that encodes the 12-byte type-3 payload prefix (algorithm, uncompressed size, reserved 0) plus opaque compressed bytes. Wrap as a `SisField` of kind 3. Default tests never spawn Wine, never inflate, and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not inflate / checksums

Experiment 16 pins the prefix and that zlib starts at offset 12. Inflate needs a crate or a later native decoder; type-34/35 checksums did not match a guessed CRC of the obvious blobs. Those stay later slices.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisCompressed { algorithm, uncompressed_size, reserved, data }` with methods. No Wine paths. `SisTools` stays the host process. |
| Algorithm | Associated constant `DEFLATE = 1` as recorded. |
| Reserved | `0` as recorded on hello SIS and SISX. Do not invent a name beyond `reserved`. |
| Data | Opaque bytes after the 12-byte prefix. Do not inflate in this slice. No new crates. |
| Field | `field()` returns `SisField::new(Self::KIND, self.bytes())` with `KIND = 3`. |
| Tests | Pinned 12-byte prefixes and type-3 field length 295 for dummy zlib length 283. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisCompressed` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |

## 4. Interfaces

```rust
pub struct SisCompressed {
    pub algorithm: u32,
    pub uncompressed_size: u32,
    pub reserved: u32,
    pub data: Vec<u8>,
}

impl SisCompressed {
    pub const KIND: u32 = 3;
    pub const DEFLATE: u32 = 1;

    pub fn new(uncompressed_size: u32, data: Vec<u8>) -> Self;
    pub fn header_bytes(&self) -> [u8; 12];
    pub fn bytes(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`new`: `algorithm = DEFLATE`, `reserved = 0`, plus the two arguments.

`header_bytes`: little-endian `algorithm`, `uncompressed_size`, `reserved`.

`bytes`: `header_bytes` then `data` (no TLV padding; `SisField` pads).

`field`: `SisField::new(Self::KIND, self.bytes())`.

Pinned:

- `SisCompressed::new(548, vec![]).header_bytes()` equals `01 00 00 00 24 02 00 00 00 00 00 00` (hello SIS).
- `SisCompressed::new(1868, vec![]).header_bytes()` equals `01 00 00 00 4c 07 00 00 00 00 00 00` (hello SISX).
- `SisCompressed::new(548, vec![0; 283]).field().header_bytes()` equals `03 00 00 00 27 01 00 00` (type 3, length 295). Dummy `data` is only for length.

## 5. Non-goals

Inflate; type-34/35 checksums; type-30 data; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; new crates.
