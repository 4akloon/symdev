# T2 SIS DateTime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisDate` / `SisTime` / `SisDateTime` encode the frozen hello type-8 stamp.

**Architecture:** New types in `crates/symdev-build/src/sis_datetime.rs`. Reuse `SisField` for TLV and padding. No clock. No Wine. Export all three from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-datetime-design.md](../specs/2026-09-18-symdev-t2-sis-datetime-design.md)
- Types + methods. These types must not take Wine paths and must not read the host clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-datetime`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_datetime.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_datetime; pub use sis_datetime::{SisDate, SisDateTime, SisTime}`

---

### Task 1: `SisDate` / `SisTime` / `SisDateTime`

**Files:** `sis_datetime.rs`, `lib.rs` exports.

**Interfaces:** as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_date_payload_matches_experiment_21() {
    assert_eq!(SisDate::new(2026, 8, 17).payload(), [0xea, 0x07, 0x08, 0x11]);
    assert_eq!(SisDate::KIND, 6);
}

#[test]
fn hello_time_payload_matches_experiment_21() {
    assert_eq!(SisTime::new(15, 18, 24).payload(), [0x0f, 0x12, 0x18]);
    assert_eq!(SisTime::KIND, 7);
}

#[test]
fn hello_datetime_field_matches_experiment_21() {
    let f = SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)).field();
    assert_eq!(
        f.bytes(),
        [
            8, 0, 0, 0, 0x18, 0, 0, 0, 6, 0, 0, 0, 4, 0, 0, 0, 0xea, 0x07, 0x08, 0x11, 7, 0, 0,
            0, 3, 0, 0, 0, 0x0f, 0x12, 0x18, 0
        ]
    );
    assert_eq!(SisDateTime::KIND, 8);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_datetime --offline` FAIL (types missing)
- [ ] **Step 3:** Implement the three types as specified. Date payload is year LE + month + day. Time payload is three bytes. DateTime payload concatenates `date.field().bytes()` and `time.field().bytes()`. No clock. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_datetime --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-8 date and time from the hello stamp.`

---

## Self-review

- No Wine. No clock. No time crate. No package wiring. No `.sis` files in git. Month is 0-based as recorded.
