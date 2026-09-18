# T2: SIS files (`SisWord41`, `SisHash`, `SisFile`, `SisFiles`)

Date: 2026-09-18
Status: approved for SDD (user: continue T2; type 28 then type 13; do not fake type 28 as opaque bytes).

Cites: [2026-09-18-symdev-t2-sis-words-design.md](2026-09-18-symdev-t2-sis-words-design.md); [2026-09-18-symdev-t2-sis-products-design.md](2026-09-18-symdev-t2-sis-products-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiments 29–30.

## 1. Goal

Value types that encode the frozen hello type-28 file list from nested TLVs already visible in the inflated type-13 blob: one type-24 file, then two type-2 raw u32 words. Default tests never spawn Wine, never inflate, and never read `.sis` / `.pkg` / `.exe` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not opaque bytes / type 13

A previous walk of type 28 was incomplete (called header `29` “type 29”; called type 25 “16 bytes”). Experiments 29–30 re-dump the same frozen inflate. Type 13 compose stays the next slice and must call these types, not pin type 28 as a byte blob.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisWord41`, `SisHash`, `SisFile`, `SisFiles` with methods. No Wine paths. |
| Path | Reuse `SisString`. Hello dest is `!:\sys\bin\hello.exe` (UTF-16-LE, no NUL). Two further type-1 fields are empty strings (`n=0`). |
| Type 41 | One LE `u32`, KIND `41`. Hello: `0x000be000`. Do not invent a C/capability name. |
| Type 25 | KIND `25`. Payload is three LE `u32` then 20 bytes. Hello words `1`, `0x25`, `0x14`; digest is SHA-1 of frozen `hello.exe` (observed by hashing that file, not from C headers). Do not treat the payload as nested TLVs. |
| Type 24 leftover | After the last empty string, five LE `u32` with **no** nested KIND: `3588`, `0`, `3588`, `0`, `0`. `3588` (`0x0e04`) matches experiment-6 `hello.exe` size. Do **not** walk `04 0e 00 00 00 00 00 00` as type `3588` length 0. |
| Type 24 | KIND `24`. Payload concatenates child `field().bytes()` then those five u32s. Hello unpadded `n=136`, field 144 bytes. |
| Type 28 | KIND `28`. Like `SisInfo`: concatenate `SisArray` of type-24 fields, then two `SisWords` (`0x0d` and `0x1a`). Those trailing type-2 payloads are raw u32 arrays, so reuse `SisWords`. Hello unpadded `n=176`, field 184 bytes. |
| Tests | Pinned experiment 29–30 bytes. No Wine. Do not commit or read `.sis` / `.sisx` / `.pkg` / `.exe`. |
| Package | Do not call these types from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. No invented C names. |
| Parallel | Type 13 compose is the next slice. Do not add an inflate crate. |

## 4. Interfaces

```rust
pub struct SisWord41 {
    pub value: u32,
}

impl SisWord41 {
    pub const KIND: u32 = 41;
    pub fn new(value: u32) -> Self;
    pub fn payload(&self) -> [u8; 4];
    pub fn field(&self) -> SisField;
}

pub struct SisHash {
    pub words: [u32; 3],
    pub digest: Vec<u8>,
}

impl SisHash {
    pub const KIND: u32 = 25;
    pub fn new(words: [u32; 3], digest: impl Into<Vec<u8>>) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}

pub struct SisFile {
    pub dest: SisString,
    pub empty: SisString,
    pub word41: SisWord41,
    pub hash: SisHash,
    pub empty2: SisString,
    pub tail: [u32; 5],
}

impl SisFile {
    pub const KIND: u32 = 24;
    pub fn new(
        dest: SisString,
        empty: SisString,
        word41: SisWord41,
        hash: SisHash,
        empty2: SisString,
        tail: [u32; 5],
    ) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}

pub struct SisFiles {
    pub files: SisArray,
    pub word_d: SisWords,
    pub word_1a: SisWords,
}

impl SisFiles {
    pub const KIND: u32 = 28;
    pub fn new(files: SisArray, word_d: SisWords, word_1a: SisWords) -> Self;
    pub fn payload(&self) -> Vec<u8>;
    pub fn field(&self) -> SisField;
}
```

`SisWord41::payload` is `self.value.to_le_bytes()`.

`SisWord41::field` is `SisField::new(Self::KIND, self.payload().to_vec())`.

`SisHash::payload` concatenates each word as `to_le_bytes()` then `digest`.

`SisHash::field` is `SisField::new(Self::KIND, self.payload())`.

`SisFile::payload` concatenates `dest`, `empty`, `word41`, `hash`, `empty2` each as `field().bytes()`, then each `tail` word as `to_le_bytes()`.

`SisFile::field` is `SisField::new(Self::KIND, self.payload())`.

`SisFiles::payload` concatenates `files.field().bytes()`, `word_d.field().bytes()`, `word_1a.field().bytes()`.

`SisFiles::field` is `SisField::new(Self::KIND, self.payload())`.

Hello constructors for tests:

```rust
fn hello_hash() -> SisHash {
    SisHash::new(
        [1, 0x25, 0x14],
        [
            0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5,
            0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
        ],
    )
}

fn hello_file() -> SisFile {
    SisFile::new(
        SisString::new("!:\\sys\\bin\\hello.exe"),
        SisString::new(""),
        SisWord41::new(0x000b_e000),
        hello_hash(),
        SisString::new(""),
        [3588, 0, 3588, 0, 0],
    )
}

fn hello_files() -> SisFiles {
    SisFiles::new(
        SisArray::new(vec![hello_file().field()]),
        SisWords::new(vec![0x0d]),
        SisWords::new(vec![0x1a]),
    )
}
```

Pinned `SisWord41::new(0x000b_e000).field().bytes()`:

`29 00 00 00 04 00 00 00 00 e0 0b 00`

Pinned `hello_hash().field().bytes()` (40 bytes):

`19 00 00 00 20 00 00 00 01 00 00 00 25 00 00 00 14 00 00 00 3a 23 e7 e7 e6 0e d9 73 54 53 4b 2a 77 e5 65 cd 64 ea 39 70`

Pinned `hello_file().field().bytes()` (144 bytes):

`18 00 00 00 88 00 00 00` then dest string field, empty string field `01 00 00 00 00 00 00 00`, type-41 field, type-25 field, empty string field, then `04 0e 00 00 00 00 00 00 04 0e 00 00 00 00 00 00 00 00 00 00`.

Pinned `hello_files().field().bytes()` (184 bytes):

`1c 00 00 00 b0 00 00 00` + type-2 array `n=144` wrapping the type-24 field + `SisWords::new(vec![0x0d]).field().bytes()` + `SisWords::new(vec![0x1a]).field().bytes()`.

Type-24 payload layout (136 bytes):

1. Type `1` `n=40` dest `!:\sys\bin\hello.exe` (48-byte field).
2. Type `1` `n=0` empty (8-byte field).
3. Type `41` `n=4` payload `00 e0 0b 00`.
4. Type `25` `n=32` three words plus 20-byte digest.
5. Type `1` `n=0` empty (8-byte field).
6. Leftover 20 bytes: five LE u32 `3588, 0, 3588, 0, 0`.

Type-28 payload layout (176 bytes):

1. Type-2 array `n=144` whose payload is exactly the 144-byte type-24 field.
2. Type-2 raw u32 `0x0d` (`SisWords`).
3. Type-2 raw u32 `0x1a` (`SisWords`).

## 5. Non-goals

Type 13 compose; type 30 file data; checksums 34/35; native `signsis`; in-tree inflate; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
