# T2 SIS Product Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisProductVersion` / `SisProduct` encode the frozen hello type-18 S60 product.

**Architecture:** New types in `crates/symdev-build/src/sis_product.rs`. Reuse `SisField`, `SisVersion`, `SisPkgUid`, `SisArray`, `SisString`. No Wine. Export both from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-product-design.md](../specs/2026-09-18-symdev-t2-sis-product-design.md)
- Types + methods. These types must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-product`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.
- Type 5 wraps one `SisVersion` field. Do not invent a second (to) version. Type 17 stays out. Reuse `SisPkgUid` for `0x102752AE`.

## File structure

- Create: `crates/symdev-build/src/sis_product.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_product; pub use sis_product::{SisProduct, SisProductVersion}`

---

### Task 1: `SisProductVersion` / `SisProduct`

**Files:** `sis_product.rs`, `lib.rs` exports.

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
fn hello_product_version_field_matches_experiment_24() {
    let f = SisProductVersion::new(SisVersion::new(0, 0, 0)).field();
    assert_eq!(
        f.bytes(),
        [
            0x05, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(SisProductVersion::KIND, 5);
}

#[test]
fn hello_product_field_matches_experiment_24() {
    assert_eq!(
        hello_product().field().bytes(),
        [
            0x12, 0x00, 0x00, 0x00, 0x50, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x00, 0xae, 0x52, 0x27, 0x10, 0x05, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00,
            0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x53, 0x00, 0x36, 0x00, 0x30, 0x00,
            0x50, 0x00, 0x72, 0x00, 0x6f, 0x00, 0x64, 0x00, 0x75, 0x00, 0x63, 0x00, 0x74, 0x00,
            0x49, 0x00, 0x44, 0x00,
        ]
    );
    assert_eq!(SisProduct::KIND, 18);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_product --offline` FAIL (types missing)
- [ ] **Step 3:** Implement as specified. Type 5 wraps `version.field().bytes()`. Type 18 concatenates uid, version wrap, and names `field().bytes()`. Reuse `SisPkgUid`. No second version. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_product --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-18 product from the hello S60 line.`

---

## Self-review

- No Wine. No type 17. No second version. No package wiring. No `.sis` files in git. UID is `SisPkgUid`, not a new type.
