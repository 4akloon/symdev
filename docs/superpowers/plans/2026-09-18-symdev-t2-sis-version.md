# T2 SIS Version Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisVersion` encodes type-4 major/minor/build as three LE `u32` and wraps as a `SisField`.

**Architecture:** New type in `crates/symdev-build/src/sis_version.rs`. Reuse `SisField`. No Wine. Export `SisVersion` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-version-design.md](../specs/2026-09-18-symdev-t2-sis-version-design.md)
- Types + methods. `SisVersion` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-version`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_version.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_version; pub use sis_version::SisVersion`

---

### Task 1: `SisVersion`

**Files:** `sis_version.rs`, `lib.rs` exports.

**Interfaces:** `SisVersion::KIND`, `new`, `payload`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_pkg_version_payload_matches_experiment_18() {
    assert_eq!(
        SisVersion::new(1, 0, 24).payload(),
        [1, 0, 0, 0, 0, 0, 0, 0, 0x18, 0, 0, 0]
    );
}

#[test]
fn version_field_is_type_4_length_12() {
    let f = SisVersion::new(1, 0, 24).field();
    assert_eq!(f.header_bytes(), [4, 0, 0, 0, 0x0c, 0, 0, 0]);
    assert_eq!(SisVersion::KIND, 4);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_version --offline` FAIL (`SisVersion` missing)
- [ ] **Step 3:** Implement `SisVersion` as specified. `payload` is LE `major`, `minor`, `build`. `field()` is `SisField::new(Self::KIND, self.payload().to_vec())`. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_version --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-4 versions as three little-endian words.`

---

## Self-review

- No Wine on `SisVersion`. No package wiring. No `.sis` files in git.
