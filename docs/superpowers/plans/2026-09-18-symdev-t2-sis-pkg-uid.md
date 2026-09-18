# T2 SIS Package UID Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisPkgUid` encodes type-9 package UID as one LE `u32` and wraps as a `SisField`.

**Architecture:** New type in `crates/symdev-build/src/sis_pkg_uid.rs`. Distinct from `SisUid`. Reuse `SisField`. No Wine. Export `SisPkgUid` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-pkg-uid-design.md](../specs/2026-09-18-symdev-t2-sis-pkg-uid-design.md)
- Types + methods. `SisPkgUid` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-pkg-uid`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_pkg_uid.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_pkg_uid; pub use sis_pkg_uid::SisPkgUid`

---

### Task 1: `SisPkgUid`

**Files:** `sis_pkg_uid.rs`, `lib.rs` exports.

**Interfaces:** `SisPkgUid::KIND`, `new`, `payload`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_pkg_uid_payload_matches_experiment_19() {
    assert_eq!(SisPkgUid::new(0xe79e_4cf9).payload(), [0xf9, 0x4c, 0x9e, 0xe7]);
}

#[test]
fn pkg_uid_field_is_type_9_length_4() {
    let f = SisPkgUid::new(0xe79e_4cf9).field();
    assert_eq!(f.header_bytes(), [9, 0, 0, 0, 4, 0, 0, 0]);
    assert_eq!(SisPkgUid::KIND, 9);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_pkg_uid --offline` FAIL (`SisPkgUid` missing)
- [ ] **Step 3:** Implement `SisPkgUid` as specified. `payload` is `uid.to_le_bytes()`. `field()` is `SisField::new(Self::KIND, self.payload().to_vec())`. Do not change `SisUid`. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_pkg_uid --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-9 package UIDs as one little-endian word.`

---

## Self-review

- No Wine on `SisPkgUid`. Distinct from `SisUid`. No package wiring. No `.sis` files in git.
