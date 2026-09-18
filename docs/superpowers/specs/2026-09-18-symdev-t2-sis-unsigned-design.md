# T2: unsigned SIS file (`SisUnsigned`)

Date: 2026-09-18
Status: approved for SDD (user: compose full unsigned SIS matching frozen hello.sis; name is not `SisFile`).

Cites: [2026-09-18-symdev-t2-sis-controller-design.md](2026-09-18-symdev-t2-sis-controller-design.md); [2026-09-18-symdev-t2-sis-data-design.md](2026-09-18-symdev-t2-sis-data-design.md); [2026-09-18-symdev-t2-sis-checksum-design.md](2026-09-18-symdev-t2-sis-checksum-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 34.

## 1. Goal

A value type whose `bytes()` is a complete unsigned SIS: 16-byte `SisUid` plus a `SisField` of KIND 12 (`SisField::CONTROLLER`) whose payload is the concatenation of checksum 34, checksum 35, compressed controller (type 3), and data (type 30) field bytes, in that recorded order. Hello.sis (4000 bytes) must byte-equal that output when the compressor matches. Default tests never spawn Wine and never read `.sis` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not `SisFile` / `SisPackage`

`SisFile` is already KIND 24. Type 12 is the outer controller wrapper, not type 13. This slice only concatenates already-encoded children plus live 34/35 checksums. Do not wire `SisPackage` or clap.

## 3. Locked decisions

| # | Decision |
|---|---|
| Name | `SisUnsigned`. Not `SisFile`. Not `SisRoot`. |
| Shape | Methods. No Wine paths. No clock. Implements `SisEncode` with `KIND = SisField::CONTROLLER` (12). |
| `bytes()` | `uid.bytes()` then `field().bytes()`. |
| Payload | `[checksum34.field(), checksum35.field(), compressed.field(), data.field()].concat()`. Checksums from `SisChecksum34::of(&compressed.field())` and `SisChecksum35::of(&data.field())` — always live, not pinned literals in `payload()`. |
| Type 3 | Reuse `SisCompressed` (alg=1 zlib). Compress the type-13 `SisController::field().bytes()` (548 bytes). |
| Compressor | `flate2` with `default-features = false`, `features = ["zlib"]` (system `libz` via `libz-sys`), `Compression::new(6)`. That matches the frozen 283-byte zlib and Python `zlib.compress` default. `flate2` default `rust_backend` (miniz_oxide) and `zlib-rs` at level 6 did **not** match; do not pin opaque zlib. |
| Type 30 | Reuse `SisData` as already encoded. |
| Tests | Pinned experiment 34: `bytes()` equals dumped hello.sis hex testdata. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call `SisUnsigned` from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |
| Merge | `sis/mod.rs` and crate-root `lib.rs` diffs are **append-only** exports. |

## 4. Interface

```rust
pub struct SisUnsigned {
    pub uid: SisUid,
    pub compressed: SisCompressed,
    pub data: SisData,
}

impl SisUnsigned {
    pub const KIND: u32 = SisField::CONTROLLER;
    pub fn new(uid: SisUid, compressed: SisCompressed, data: SisData) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn bytes(&self) -> Vec<u8>;
}

impl SisEncode for SisUnsigned {
    const KIND: u32 = SisUnsigned::KIND;
    fn payload(&self) -> Vec<u8>;
}
```

`payload`: live `SisChecksum34::of` / `SisChecksum35::of` then the two inner `field().bytes()`.

`field` (trait default): `SisField::new(Self::KIND, self.payload())`.

`bytes`: `uid.bytes()` concatenated with `field().bytes()`.

Hello constructor for tests: `SisUid::new(0xe79e4cf9)`, `SisCompressed` over the already-pinned hello `SisController` field bytes, `SisData` as experiment 33. Full `bytes()` equals the 4000-byte experiment-34 hex (`include_str`, not a `.sis`).

Pinned outer header after UID: `0c 00 00 00 88 0f 00 00` (type 12, length 3976). Children occupied: 12 + 12 + 304 + 3648 = 3976. File size `16 + 8 + 3976 = 4000`.

## 5. Non-goals

Native `signsis` / `makekeys`; SISX; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
