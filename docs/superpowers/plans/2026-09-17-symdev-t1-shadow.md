# T1-shadow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compare native `uidcrc_line` to Wine `uidcrc.exe` stdout after stripping CR/LF, without spawning Wine in default tests.

**Architecture:** Extend `crates/symdev-build/src/uidcrc.rs`. Reuse `uidcrc_line`. Export the two new functions from `lib.rs`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-17-symdev-t1-shadow-design.md](../specs/2026-09-17-symdev-t1-shadow-design.md)
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not wire into `package` or clap.
- Never `-fPIC`. No E52 claim. Work on branch `t1-shadow`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Modify: `crates/symdev-build/src/uidcrc.rs`
- Modify: `crates/symdev-build/src/lib.rs` — export `normalize_uidcrc_stdout`, `uidcrc_matches_wine`

---

### Task 1: normalize + match

**Files:** `uidcrc.rs`, `lib.rs` exports.

**Interfaces:**
- `pub fn normalize_uidcrc_stdout(bytes: &[u8]) -> String`
- `pub fn uidcrc_matches_wine(uid1: u32, uid2: u32, uid3: u32, wine_stdout: &[u8]) -> bool`

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn normalize_strips_crlf() {
    assert_eq!(
        normalize_uidcrc_stdout(b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n"),
        "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e"
    );
}

#[test]
fn uidcrc_matches_wine_hello_crlf() {
    let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n";
    assert!(uidcrc_matches_wine(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9, out));
}

#[test]
fn uidcrc_matches_wine_rejects_wrong_checked() {
    let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x00000000\n";
    assert!(!uidcrc_matches_wine(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9, out));
}
```

- [ ] **Step 2:** `cargo test -p symdev-build uidcrc_matches --offline` FAIL (functions missing)
- [ ] **Step 3:** Minimal impl: lossy UTF-8, trim `\r` `\n` space tab at ends, compare to `uidcrc_line`
- [ ] **Step 4:** PASS `cargo test -p symdev-build uidcrc --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Compare native uidcrc lines to CRLF Wine stdout.`

---

## Self-review

- No Wine spawn. No clap. No package wiring. No makekeys.
