# T2: SIS checksums (`SisChecksum34`, `SisChecksum35`)

Date: 2026-09-18
Status: approved for SDD (user: T2 slice; derive type 34/35 from frozen hello.sis; two KIND wrappers).

Cites: [2026-09-18-symdev-t2-sis-compressed-design.md](2026-09-18-symdev-t2-sis-compressed-design.md); [2026-09-18-symdev-t2-sis-controller-design.md](2026-09-18-symdev-t2-sis-controller-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 32.

## 1. Goal

Value types that encode the frozen hello type-12 children type 34 and type 35: each is a two-byte checksum wrapping `[u8; 2]`. When the inner type-3 or type-30 field bytes are constructed, `of` recomputes the same two bytes. Default tests never spawn Wine, never inflate, and never read `.sis` / `.sisx` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 12 / type 30

Experiment 15 already pinned these as the first children of outer type 12 (`n=2`, payloads `5c 9e` and `64 03`). Guessing a CRC of “obvious” blobs failed earlier. Experiment 32 derives the byte step from the frozen files: EPOC CRC16 init 0 (same step as experiment 13 `UidCrc`) over the **padded** type-3 field bytes → type 34, and over the **padded** type-30 field bytes → type 35. Confirmed on hello.sis and hello.sisx (type 34 changes with the controller; type 35 stays `64 03` because type 30 is identical). Outer type 12 compose and type-30 data encoding stay later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | Two types `SisChecksum34` and `SisChecksum35` with `const KIND`, matching `SisWords16` / `SisWords19`. No Wine paths. |
| Value | `[u8; 2]` little-endian CRC16. Hello SIS: type 34 `5c 9e`, type 35 `64 03`. |
| Algorithm | EPOC CRC16, init `0`, same byte step as experiment 13. Input is `inner.bytes()` (kind, length, payload, `SisField` padding). Type 34 inner is the type-3 field; type 35 inner is the type-30 field. Do not CRC payload-only, zlib-only, inflated type 13, or the file after UID. |
| `new` | Pin a recorded two-byte value (unblocks type 12 compose without a type-30 encoder). |
| `of` | Compute from a constructed inner `SisField`. |
| Tests | Pinned experiment 32 field bytes. Type 34 `of` uses `SisCompressed` plus the hello zlib bytes copied into the test (not `include_bytes!` of `.sis`). Type 35 hello value is pinned; `of` is the same CRC. No Wine. Do not commit or read `.sis` / `.sisx`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Derived from frozen bytes plus the already-known EPOC CRC16 step. Do not invent a C / `CS_*` name. |
| Merge | `sis/mod.rs` and crate-root `lib.rs` diffs are **append-only** exports. |

## 4. Interfaces

```rust
pub struct SisChecksum34 {
    pub value: [u8; 2],
}

impl SisChecksum34 {
    pub const KIND: u32 = 34;
    pub fn new(value: [u8; 2]) -> Self;
    pub fn of(inner: &SisField) -> Self;
    pub fn payload(&self) -> [u8; 2];
    pub fn field(&self) -> SisField;
}

pub struct SisChecksum35 {
    pub value: [u8; 2],
}

impl SisChecksum35 {
    pub const KIND: u32 = 35;
    pub fn new(value: [u8; 2]) -> Self;
    pub fn of(inner: &SisField) -> Self;
    pub fn payload(&self) -> [u8; 2];
    pub fn field(&self) -> SisField;
}
```

`payload` returns `self.value`.

`field` is `SisField::new(Self::KIND, self.payload().to_vec())`. Length 2 is padded to 4 by `SisField`.

`of`: run the experiment-13 CRC16 byte step over `inner.bytes()`, store the `u16` as little-endian `[u8; 2]`.

Pinned:

- `SisChecksum34::new([0x5c, 0x9e]).field().bytes()` equals `22 00 00 00 02 00 00 00 5c 9e 00 00`
- `SisChecksum35::new([0x64, 0x03]).field().bytes()` equals `23 00 00 00 02 00 00 00 64 03 00 00`
- `SisChecksum34::of(&SisCompressed::new(548, hello_type3_zlib).field()).value` equals `[0x5c, 0x9e]`

Hello type-3 zlib (283 bytes) is the experiment-32 const in the unit test, copied from frozen `hello.sis` type-3 payload offset 12; not read from disk at test time.

## 5. Non-goals

Outer type 12 compose; type-30 data encoder; inflate crate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5; new crates.
