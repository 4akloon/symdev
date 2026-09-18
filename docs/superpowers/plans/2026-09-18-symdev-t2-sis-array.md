# T2 SIS Array Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisArray` encodes type-2 as concatenated child `SisField::bytes()`.

**Architecture:** New type in `crates/symdev-build/src/sis_array.rs`. Reuse `SisField` and `SisString`. No Wine. Export `SisArray` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-array-design.md](../specs/2026-09-18-symdev-t2-sis-array-design.md)
- Types + methods. `SisArray` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-array`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_array.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_array; pub use sis_array::SisArray`

---

### Task 1: `SisArray`

**Files:** `sis_array.rs`, `lib.rs` exports.

**Interfaces:** `SisArray::KIND`, `new`, `payload`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_name_array_matches_experiment_20() {
    let f = SisArray::new(vec![SisString::new("hello").field()]).field();
    assert_eq!(
        f.bytes(),
        [
            2, 0, 0, 0, 0x14, 0, 0, 0, 1, 0, 0, 0, 0x0a, 0, 0, 0, 0x68, 0, 0x65, 0, 0x6c, 0,
            0x6c, 0, 0x6f, 0, 0, 0
        ]
    );
    assert_eq!(SisArray::KIND, 2);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_array --offline` FAIL (`SisArray` missing)
- [ ] **Step 3:** Implement `SisArray` as specified. `payload` concatenates `item.bytes()`. `field()` is `SisField::new(Self::KIND, self.payload())`. No count prefix. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_array --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-2 arrays as concatenated child fields.`

---

## Self-review

- No Wine on `SisArray`. No count prefix. No package wiring. No `.sis` files in git.
