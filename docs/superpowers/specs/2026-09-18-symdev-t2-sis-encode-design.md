# T2: SIS encode trait (`SisEncode`)

Date: 2026-09-18
Status: approved for SDD (user: one trait for the repeated KIND + payload + field shape).

Cites: [2026-09-18-symdev-t2-sis-field-design.md](2026-09-18-symdev-t2-sis-field-design.md).

## 1. Goal

One trait for SIS value types that already share `KIND`, `payload()`, and `field()` as `SisField::new(Self::KIND, payload)`. Goldens and constructors stay the same.

## 2. Locked decisions

| # | Decision |
|---|---|
| Shape | One trait. No `SisArray<T>`, no extra encode traits, no async. |
| `field()` | Default method. Delete inherent `field()` on implementors. |
| Wrappers | Types that only concatenated children gain `payload()` from that concat so the default `field()` works. |
| Not in trait | `SisField` itself. `SisPackage` / `SisTools`. `SisUid` (16-byte prefix, no KIND/field). Checksums 34/35 and type 30 stay other worktrees. |
| `[u8; N]` | Inherent `payload()` may stay; trait impl `to_vec()`. |
| Tests | Existing goldens still pass. Optional: one type’s `T::KIND` matches `field().kind`. |

## 3. Interface

```rust
pub trait SisEncode {
    const KIND: u32;
    fn payload(&self) -> Vec<u8>;
    fn field(&self) -> SisField {
        SisField::new(Self::KIND, self.payload())
    }
}
```

Trait lives next to `SisField` and is exported from `sis/mod.rs` and crate root.
