# T2 SIS Compressed Prefix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisCompressed` encodes the type-3 12-byte prefix plus opaque bytes and wraps as a `SisField` of kind 3.

**Architecture:** New type in `crates/symdev-build/src/sis_compressed.rs`. Reuse `SisField` for the TLV frame. No inflate. No Wine. Export `SisCompressed` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-compressed-design.md](../specs/2026-09-18-symdev-t2-sis-compressed-design.md)
- Types + methods. `SisCompressed` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-compressed`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_compressed.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_compressed; pub use sis_compressed::SisCompressed`

---

### Task 1: `SisCompressed`

**Files:** `sis_compressed.rs`, `lib.rs` exports.

**Interfaces:** `SisCompressed::KIND`, `DEFLATE`, `new`, `header_bytes`, `bytes`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_sis_compressed_prefix_matches_experiment_16() {
    assert_eq!(
        SisCompressed::new(548, vec![]).header_bytes(),
        [1, 0, 0, 0, 0x24, 2, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn hello_sisx_compressed_prefix_matches_experiment_16() {
    assert_eq!(
        SisCompressed::new(1868, vec![]).header_bytes(),
        [1, 0, 0, 0, 0x4c, 7, 0, 0, 0, 0, 0, 0]
    );
}

#[test]
fn compressed_field_header_is_type_3_length_295() {
    let f = SisCompressed::new(548, vec![0; 283]).field();
    assert_eq!(f.header_bytes(), [3, 0, 0, 0, 0x27, 1, 0, 0]);
    assert_eq!(SisCompressed::KIND, 3);
    assert_eq!(SisCompressed::DEFLATE, 1);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_compressed --offline` FAIL (`SisCompressed` missing)
- [ ] **Step 3:** Implement `SisCompressed` as specified. `new` sets `algorithm = DEFLATE` and `reserved = 0`. `field()` is `SisField::new(Self::KIND, self.bytes())`. No inflate. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_compressed --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode the SIS compressed-field 12-byte prefix.`

---

## Self-review

- No Wine on `SisCompressed`. No inflate. No package wiring. No `.sis` files in git. No new crates.
