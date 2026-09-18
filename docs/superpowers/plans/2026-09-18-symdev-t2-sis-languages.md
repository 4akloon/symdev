# T2 SIS Languages Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisLanguage` / `SisLanguages` encode the frozen hello type-15 English list.

**Architecture:** New types in `crates/symdev-build/src/sis_language.rs`. Reuse `SisField` and `SisArray`. No Wine. Export both from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-languages-design.md](../specs/2026-09-18-symdev-t2-sis-languages-design.md)
- Types + methods. These types must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-languages`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.
- Do not invent a language-name table. Hello language id is `1`. Type 16 stays out.

## File structure

- Create: `crates/symdev-build/src/sis_language.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_language; pub use sis_language::{SisLanguage, SisLanguages}`

---

### Task 1: `SisLanguage` / `SisLanguages`

**Files:** `sis_language.rs`, `lib.rs` exports.

**Interfaces:** as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_language_payload_matches_experiment_23() {
    assert_eq!(SisLanguage::new(1).payload(), [1, 0, 0, 0]);
    assert_eq!(SisLanguage::KIND, 11);
}

#[test]
fn hello_languages_field_matches_experiment_23() {
    let f = SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])).field();
    assert_eq!(
        f.bytes(),
        [
            0x0f, 0, 0, 0, 0x14, 0, 0, 0, 2, 0, 0, 0, 0x0c, 0, 0, 0, 0x0b, 0, 0, 0, 4, 0, 0, 0,
            1, 0, 0, 0
        ]
    );
    assert_eq!(SisLanguages::KIND, 15);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_language --offline` FAIL (types missing)
- [ ] **Step 3:** Implement as specified. Language payload is `id.to_le_bytes()`. Languages field wraps `languages.field().bytes()` as type 15. No language-name table. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_language --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-15 languages from the hello EN id.`

---

## Self-review

- No Wine. No language-name table. No package wiring. No `.sis` files in git. Type 16 not implemented.
