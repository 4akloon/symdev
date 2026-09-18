# T2 SIS Products Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisProducts` encodes the frozen hello type-17 product list.

**Architecture:** New type in `crates/symdev-build/src/sis/products.rs`. Reuse `SisField`, `SisArray`, and existing `SisProduct`. Trailing type-2 word is 12 concatenated bytes, not `SisWords`. No Wine. Export from `sis/mod.rs` and crate root with append-only diffs.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-products-design.md](../specs/2026-09-18-symdev-t2-sis-products-design.md)
- Types + methods. This type must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-products` in an isolated git worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.
- Reuse `SisProduct`. Do not invent a second version. Do not create `SisWords`. Do not implement types 16/19/40.
- `sis/mod.rs` and `lib.rs` diffs are append-only.

## File structure

- Create: `crates/symdev-build/src/sis/products.rs`
- Modify: `crates/symdev-build/src/sis/mod.rs` — append `mod products;` and `pub use products::SisProducts`
- Modify: `crates/symdev-build/src/lib.rs` — append `SisProducts` to the `sis` re-export list

---

### Task 1: `SisProducts`

**Files:** `sis/products.rs`, `sis/mod.rs` exports, `lib.rs` re-exports.

**Interfaces:** as in the spec.

- [ ] **Step 1: Failing tests**

```rust
fn hello_product() -> SisProduct {
    SisProduct::new(
        SisPkgUid::new(0x1027_52ae),
        SisProductVersion::new(SisVersion::new(0, 0, 0)),
        SisArray::new(vec![SisString::new("S60ProductID").field()]),
    )
}

#[test]
fn hello_products_field_matches_experiment_28() {
    let f = SisProducts::new(SisArray::new(vec![hello_product().field()])).field();
    assert_eq!(
        f.bytes(),
        [
            0x11, 0x00, 0x00, 0x00, 0x6c, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x58, 0x00,
            0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0xae, 0x52, 0x27, 0x10, 0x05, 0x00, 0x00, 0x00, 0x14, 0x00,
            0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00,
            0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x53, 0x00, 0x36, 0x00,
            0x30, 0x00, 0x50, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x64, 0x00, 0x75, 0x00, 0x63, 0x00,
            0x74, 0x00, 0x49, 0x00, 0x44, 0x00, 0x02, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0x12, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(SisProducts::KIND, 17);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis::products --offline` FAIL (type missing)
- [ ] **Step 3:** Implement as specified. `payload()` is `products.field().bytes()` plus `[2, 0, 0, 0, 4, 0, 0, 0, 0x12, 0, 0, 0]`. Reuse `SisProduct`. No `SisWords`. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis::products --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-17 products from the hello S60 list.`

---

## Self-review

- No Wine. No SisWords. No types 16/19/40. No second version. No package wiring. No `.sis` files in git. Reuses existing `SisProduct`.
