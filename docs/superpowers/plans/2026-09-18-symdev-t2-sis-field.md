# T2 SIS Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisField` encodes one SIS type+length field with 4-byte payload padding, matching experiment 15 goldens.

**Architecture:** New type in `crates/symdev-build/src/sis_field.rs`. `SisUid` stays the 16-byte prefix; `SisField` is the TLV unit; `SisTools` stays Wine. Export `SisField` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-field-design.md](../specs/2026-09-18-symdev-t2-sis-field-design.md)
- Types + methods. `SisField` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-field`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_field.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_field; pub use sis_field::SisField`

---

### Task 1: `SisField`

**Files:** `sis_field.rs`, `lib.rs` exports.

**Interfaces:** `SisField::CONTROLLER`, `new`, `header_bytes`, `bytes` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_sis_controller_header_matches_experiment_15() {
    let f = SisField::new(SisField::CONTROLLER, vec![0; 3976]);
    assert_eq!(f.header_bytes(), [0x0c, 0, 0, 0, 0x88, 0x0f, 0, 0]);
}

#[test]
fn hello_sisx_controller_header_matches_experiment_15() {
    let f = SisField::new(SisField::CONTROLLER, vec![0; 5148]);
    assert_eq!(f.header_bytes(), [0x0c, 0, 0, 0, 0x1c, 0x14, 0, 0]);
}

#[test]
fn two_byte_field_pads_to_four() {
    let f = SisField::new(34, vec![0x5c, 0x9e]);
    assert_eq!(
        f.bytes(),
        [0x22, 0, 0, 0, 2, 0, 0, 0, 0x5c, 0x9e, 0, 0]
    );
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_field --offline` FAIL (`SisField` missing)
- [ ] **Step 3:** Implement `SisField` as specified. Header is LE `kind` + LE `payload.len() as u32`. `bytes` appends payload then zero pad `(4 - (len % 4)) % 4`. No Wine. No nested parse.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_field --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-length fields with four-byte padding.`

---

## Self-review

- No Wine on `SisField`. No package wiring. No `.sis` files in git. No compressed controller body.
