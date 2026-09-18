# T2: SIS data (`SisData`, `SisData31`, `SisData32`)

Date: 2026-09-18
Status: approved for SDD (user: T2 type 30 from frozen hello.sis; experiment 33; 32 reserved for checksums).

Cites: [2026-09-18-symdev-t2-sis-controller-design.md](2026-09-18-symdev-t2-sis-controller-design.md); [2026-09-18-symdev-t2-sis-compressed-design.md](2026-09-18-symdev-t2-sis-compressed-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 33.

## 1. Goal

Value types that encode the frozen hello type-30 field that follows the compressed type-3 controller: a type-2 array of one type-31, which is a type-2 array of one type-32, which wraps a type-3 field whose 12-byte prefix is algorithm / uncompressed size / reserved and whose data bytes are raw `hello.exe`. Default tests never spawn Wine, never inflate, and never read `.sis` / `.exe` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not checksums / type 12

Experiment 15 placed type 30 after type 3 inside the outer type-12 controller. Experiment 31 encoded the inflated type-13 body only. This slice encodes the file-bytes sibling. Types 34/35 stay on another branch (experiment 32 reserved). Do not add a zlib crate: the hello type-32 payload is not zlib.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisData32`, `SisData31`, `SisData` with methods. No Wine paths. No clock. |
| Names | KIND numbers only. Do not invent MakeSIS C names. `SisData` is type 30. |
| Type 2 | Reuse `SisArray` (concatenated child `SisField::bytes()`). Not `SisWords`. The nested type-2 payloads start with a TLV header (`1f` / `20`). |
| Type 32 | KIND `32`. Payload is exactly one `SisCompressed::field().bytes()`. |
| Type 3 inside 32 | Reuse `SisCompressed` prefix layout: little-endian `algorithm`, `uncompressed_size`, `reserved`, then `data`. Hello: algorithm `0` (not `SisCompressed::DEFLATE`), uncompressed size `3588`, reserved `0`, `data` is raw experiment-6 `hello.exe` (starts `7a 00 00 10`). Do not call `SisCompressed::new` (that sets algorithm 1). Construct the public fields. Do not inflate. |
| Type 31 | KIND `31`. Payload is one `SisArray` of the type-32 field. |
| Type 30 | KIND `30`. Payload is one `SisArray` of the type-31 field. Hello unpadded `n=3640`, field 3648 bytes (`3640 % 4 == 0`). |
| Tests | Pinned experiment 33. Pin type-30 header + length + SHA-1 of the 3588-byte data payload (same 20 bytes as experiment 29 `SisHash` digest) and head/tail of that payload. Also `field().bytes()` equals the dumped type-30 field via `include_str` hex testdata. No Wine. Do not commit or read `.sis` / `.sisx` / `.exe`. No sha1/zlib crate. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |
| Parallel | Experiment **32** is reserved for checksums 34/35 on another branch. Do not encode 34/35 here. |

## 4. Interfaces

```rust
pub struct SisData32 {
    pub compressed: SisCompressed,
}

impl SisData32 {
    pub const KIND: u32 = 32;
    pub fn new(compressed: SisCompressed) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}

pub struct SisData31 {
    pub items: SisArray,
}

impl SisData31 {
    pub const KIND: u32 = 31;
    pub fn new(items: SisArray) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}

pub struct SisData {
    pub items: SisArray,
}

impl SisData {
    pub const KIND: u32 = 30;
    pub fn new(items: SisArray) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`SisData32::payload` is `self.compressed.field().bytes()`.

`SisData32::field` is `SisField::new(Self::KIND, self.payload())`.

`SisData31::payload` / `SisData::payload` are `self.items.field().bytes()`.

`SisData31::field` / `SisData::field` are `SisField::new(Self::KIND, self.payload())`.

Hello constructor for tests (file bytes from the hex dump, offset 60, length 3588 — not `std::fs` of `hello.exe`):

```rust
fn hello_compressed() -> SisCompressed {
    SisCompressed {
        algorithm: 0,
        uncompressed_size: 3588,
        reserved: 0,
        data: hello_exe_bytes(),
    }
}

fn hello_data() -> SisData {
    SisData::new(SisArray::new(vec![SisData31::new(SisArray::new(vec![
        SisData32::new(hello_compressed()).field(),
    ]))
    .field()]))
}
```

Pinned headers (little-endian):

| Field | Header |
|---|---|
| type 30 `n=3640` (`0x0e38`) | `1e 00 00 00 38 0e 00 00` |
| type 2 `n=3632` (`0x0e30`) | `02 00 00 00 30 0e 00 00` |
| type 31 `n=3624` (`0x0e28`) | `1f 00 00 00 28 0e 00 00` |
| type 2 `n=3616` (`0x0e20`) | `02 00 00 00 20 0e 00 00` |
| type 32 `n=3608` (`0x0e18`) | `20 00 00 00 18 0e 00 00` |
| type 3 `n=3600` (`0x0e10`) | `03 00 00 00 10 0e 00 00` |
| compressed prefix | `00 00 00 00 04 0e 00 00 00 00 00 00` (alg `0`, uncomp `3588`, reserved `0`) |

Type-30 field layout (3648 bytes):

| off | bytes | what |
|---|---|---|
| 0 | 8 | type 30 header |
| 8 | 8 | type 2 header |
| 16 | 8 | type 31 header |
| 24 | 8 | type 2 header |
| 32 | 8 | type 32 header |
| 40 | 8 | type 3 header |
| 48 | 12 | compressed prefix |
| 60 | 3588 | raw `hello.exe` |

Pinned data-payload SHA-1 (host `hashlib.sha1` of frozen `hello.exe` / of bytes `[60..]`): `3a23e7e7e60ed97354534b2a77e565cd64ea3970` (same 20 bytes as experiment 29). Head `7a 00 00 10 00 00 00 00 f9 4c 9e e7 b0 08 32 c1`. Tail `dc 95 8a 46 a2 45 c4 8c 39 38 35 bb 91 10 7f ff`.

Full type-30 `field().bytes()` is the experiment-33 hex in `crates/symdev-build/src/sis/testdata/hello_type30.hex` (`include_str`, not a `.sis`).

## 5. Non-goals

Checksums 34/35 (experiment 32); outer type 12 compose; native `signsis` / `makekeys`; in-tree inflate or a zlib/sha1 crate; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
