# T2: SIS products (`SisProducts`)

Date: 2026-09-18
Status: approved for SDD (user: independent T2 slice; types + methods; value types do not know Wine).

Cites: [2026-09-18-symdev-t2-sis-product-design.md](2026-09-18-symdev-t2-sis-product-design.md); [experiment-backlog.md](../../research/experiment-backlog.md) experiment 28.

## 1. Goal

A value type that encodes the frozen hello type-17 product list: a type-2 array of one type-18 `SisProduct`, then a trailing type-2 field whose payload is LE `u32` `0x12`. Default tests never spawn Wine, never inflate, and never read `.sis` / `.pkg` from disk. Still no clap verb. `symdev package` still uses Wine `SisTools`.

## 2. Why this, not type 16 / 19 / 40

Experiment 28 pins type 17 from the same inflated type-13 as experiment 24. Types 16/19/40 and `SisWords` belong to a parallel slice. Type-13 compose stays later.

## 3. Locked decisions

| # | Decision |
|---|---|
| Shape | `SisProducts` with methods. No Wine paths. |
| Kind | Type `17`. Field is 116 bytes; unpadded payload `n=108`. |
| Products | Reuse `SisArray` of one hello `SisProduct` field. Reuse `SisProduct` in `sis/product.rs`. Do not invent a second version. |
| Trailing word | Concatenate `02 00 00 00 04 00 00 00 12 00 00 00` in `payload()` (or a tiny private helper). Do not create `SisWords`. |
| Tests | Pinned experiment 28 bytes. No Wine. Do not commit or read `.sis` / `.sisx` / `.pkg`. |
| Package | Do not call this type from `SisPackage` in this slice. |
| Sources | Do not copy MakeSIS C. Goldens only. |
| Parallel | `sis/mod.rs` and `lib.rs` diffs are append-only. Do not rewrite experiment backlog sections 1–27. |

## 4. Interfaces

```rust
pub struct SisProducts {
    pub products: SisArray,
}

impl SisProducts {
    pub const KIND: u32 = 17;
    pub fn new(products: SisArray) -> Self;
    pub fn payload(&self) -> Vec<u8>; // products.field().bytes() + [2,0,0,0, 4,0,0,0, 0x12,0,0,0]
    pub fn field(&self) -> SisField;
}
```

`SisProducts::payload` is `products.field().bytes()` then those 12 trailing bytes.

`SisProducts::field` is `SisField::new(Self::KIND, self.payload())`.

Hello constructor for tests (same as `sis/product.rs`):

```rust
SisProduct::new(
    SisPkgUid::new(0x1027_52ae),
    SisProductVersion::new(SisVersion::new(0, 0, 0)),
    SisArray::new(vec![SisString::new("S60ProductID").field()]),
)
```

Hello `SisProducts` is `SisProducts::new(SisArray::new(vec![that_product.field()]))`.

Pinned `field().bytes()` (116 bytes):

`11 00 00 00 6c 00 00 00 02 00 00 00 58 00 00 00 12 00 00 00 50 00 00 00 09 00 00 00 04 00 00 00 ae 52 27 10 05 00 00 00 14 00 00 00 04 00 00 00 0c 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 02 00 00 00 20 00 00 00 01 00 00 00 18 00 00 00 53 00 36 00 30 00 50 00 72 00 6f 00 64 00 75 00 63 00 74 00 49 00 44 00 02 00 00 00 04 00 00 00 12 00 00 00`

Payload layout:

1. Type-2 array `n=88` whose payload is exactly the 88-byte type-18 `SisProduct` field.
2. Type-2 field of length 4 whose payload is LE `u32` `0x12`.

Month/clock are irrelevant.

## 5. Non-goals

`SisWords`; types 16/19/40; type 13 compose; a second version word; in-tree inflate; native `signsis` / `makekeys`; wiring into `package`; clap; spawning Wine; E52 claim; T5 / M3 / M5.
