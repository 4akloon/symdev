# T2 SIS String Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisString` encodes type-1 UTF-16-LE strings and wraps as a `SisField`.

**Architecture:** New type in `crates/symdev-build/src/sis_string.rs`. Reuse `SisField` for TLV + padding. No inflate. No Wine. Export `SisString` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-string-design.md](../specs/2026-09-18-symdev-t2-sis-string-design.md)
- Types + methods. `SisString` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-string`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_string.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_string; pub use sis_string::SisString`

---

### Task 1: `SisString`

**Files:** `sis_string.rs`, `lib.rs` exports.

**Interfaces:** `SisString::KIND`, `new`, `payload`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn vendor_payload_is_utf16_le_without_nul() {
    assert_eq!(
        SisString::new("Vendor").payload(),
        [0x56, 0, 0x65, 0, 0x6e, 0, 0x64, 0, 0x6f, 0, 0x72, 0]
    );
}

#[test]
fn hello_field_pads_odd_utf16_length() {
    assert_eq!(
        SisString::new("hello").field().bytes(),
        [
            1, 0, 0, 0, 0x0a, 0, 0, 0, 0x68, 0, 0x65, 0, 0x6c, 0, 0x6c, 0, 0x6f, 0, 0, 0
        ]
    );
    assert_eq!(SisString::KIND, 1);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_string --offline` FAIL (`SisString` missing)
- [ ] **Step 3:** Implement `SisString` as specified. `payload` from `encode_utf16` as LE `u16`s. `field()` is `SisField::new(Self::KIND, self.payload())`. No inflate. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_string --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-1 strings as UTF-16-LE.`

---

## Self-review

- No Wine on `SisString`. No inflate. No package wiring. No `.sis` files in git. No new crates.
