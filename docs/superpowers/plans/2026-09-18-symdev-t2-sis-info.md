# T2 SIS Info Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisInfo` composes the frozen hello type-14 block from existing T2 leaves.

**Architecture:** New type in `crates/symdev-build/src/sis_info.rs`. Reuse `SisField` for TLV and padding. No clock. No Wine. Export `SisInfo` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-info-design.md](../specs/2026-09-18-symdev-t2-sis-info-design.md)
- Types + methods. `SisInfo` must not take Wine paths and must not read the host clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-info`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.
- Month is 0-based (`SisDate::new(2026, 8, 17)`). Payload ends with two extra zeros inside the unpadded length, then `SisField` pads.

## File structure

- Create: `crates/symdev-build/src/sis_info.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_info; pub use sis_info::SisInfo`

---

### Task 1: `SisInfo`

**Files:** `sis_info.rs`, `lib.rs` exports.

**Interfaces:** `SisInfo::KIND`, `new`, `payload`, `field` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
fn hello_info() -> SisInfo {
    SisInfo::new(
        SisPkgUid::new(0xe79e_4cf9),
        SisString::new("Vendor"),
        SisArray::new(vec![SisString::new("hello").field()]),
        SisArray::new(vec![SisString::new("Vendor-EN").field()]),
        SisVersion::new(1, 0, 24),
        SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
    )
}

#[test]
fn hello_info_payload_is_148_children_plus_two_zeros() {
    let p = hello_info().payload();
    assert_eq!(p.len(), 150);
    assert_eq!(&p[148..], &[0, 0]);
}

#[test]
fn hello_info_field_matches_experiment_22() {
    assert_eq!(
        hello_info().field().bytes(),
        [
            0x0e, 0x00, 0x00, 0x00, 0x96, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x04, 0x00,
            0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00,
            0x56, 0x00, 0x65, 0x00, 0x6e, 0x00, 0x64, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0a, 0x00, 0x00, 0x00,
            0x68, 0x00, 0x65, 0x00, 0x6c, 0x00, 0x6c, 0x00, 0x6f, 0x00, 0x00, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00,
            0x56, 0x00, 0x65, 0x00, 0x6e, 0x00, 0x64, 0x00, 0x6f, 0x00, 0x72, 0x00, 0x2d, 0x00,
            0x45, 0x00, 0x4e, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x08, 0x00,
            0x00, 0x00, 0x18, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
            0xea, 0x07, 0x08, 0x11, 0x07, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x0f, 0x12,
            0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
    );
    assert_eq!(SisInfo::KIND, 14);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_info --offline` FAIL (type missing)
- [ ] **Step 3:** Implement `SisInfo` as specified. Payload concatenates child `field().bytes()` then `[0, 0]`. `field()` is `SisField::new(Self::KIND, self.payload())`. No clock. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_info --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-14 info from existing T2 leaves.`

---

## Self-review

- No Wine. No clock. No inflate. No package wiring. No `.sis` files in git. Two inner zeros are inside the unpadded length; `SisField` supplies the outer pad.
