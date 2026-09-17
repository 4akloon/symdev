# T2 SIS UID Header Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisUid` writes the 16-byte SIS UID header that matches experiment-14 hello goldens, via `UidCrc`.

**Architecture:** New type in `crates/symdev-build/src/sis_uid.rs`. `SisUid` is the SIS identity; `UidCrc` computes the checksum; `SisTools` stays Wine. Export `SisUid` from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-17-symdev-t2-sis-uid-design.md](../specs/2026-09-17-symdev-t2-sis-uid-design.md)
- Types + methods. `SisUid` must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not wire into `package` or clap. Do not commit `.sis` / `.sisx`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-uid`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/sis_uid.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod sis_uid; pub use sis_uid::SisUid`

---

### Task 1: `SisUid`

**Files:** `sis_uid.rs`, `lib.rs` exports.

**Interfaces:** `SisUid::UID1`, `UID2`, `new`, `crc`, `bytes` as in the spec.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn hello_sis_uid_bytes_match_experiment_14() {
    let want = [
        0x7a, 0x1a, 0x20, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0x04, 0x00,
        0xb4, 0x5d,
    ];
    assert_eq!(SisUid::new(0xe79e_4cf9).bytes(), want);
}

#[test]
fn hello_sis_uid_checked_matches_experiment_14() {
    assert_eq!(SisUid::new(0xe79e_4cf9).crc().checked(), 0x5db4_0004);
}

#[test]
fn sis_uid_constants_are_recorded() {
    assert_eq!(SisUid::UID1, 0x1020_1a7a);
    assert_eq!(SisUid::UID2, 0);
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis_uid --offline` FAIL (`SisUid` missing)
- [ ] **Step 3:** Implement `SisUid` as specified. `crc()` returns `UidCrc::new(Self::UID1, Self::UID2, self.package)`. `bytes()` delegates to `self.crc().bytes()`. No Wine. No SIS body.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis_uid --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Match Wine makesis SIS UID headers with SisUid.`

---

## Self-review

- No Wine on `SisUid`. No package wiring. No `.sis` files in git. No native SIS body.
