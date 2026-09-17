# T1-shadow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Public `uidcrc` API is `UidCrc` methods. Compare `UidCrc::line` to Wine `uidcrc.exe` stdout after stripping CR/LF, without spawning Wine in default tests.

**Architecture:** Rewrite `crates/symdev-build/src/uidcrc.rs` around `UidCrc`. Export the type from `lib.rs`. Drop public free functions.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-17-symdev-t1-shadow-design.md](../specs/2026-09-17-symdev-t1-shadow-design.md)
- Prefer types + methods. No new public free functions in this slice.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not wire into `package` or clap.
- Never `-fPIC`. No E52 claim. Work on branch `t1-shadow`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Modify: `crates/symdev-build/src/uidcrc.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `pub use uidcrc::UidCrc` only

---

### Task 1: `UidCrc` + Wine stdout match

**Files:** `uidcrc.rs`, `lib.rs` exports.

**Interfaces:** `UidCrc::new`, `checked`, `bytes`, `line`, `wine_args`, `normalize_stdout`, `matches_wine` as in the spec.

- [ ] **Step 1: Rewrite tests to the type** (existing T1 goldens + shadow cases). They must fail until methods exist.

```rust
fn hello() -> UidCrc {
    UidCrc::new(0x1000_007a, 0x1000_39ce, 0xe79e_4cf9)
}

#[test]
fn uid_checked_matches_experiment_13_goldens() {
    let cases = [
        (0x1000_007a, 0x1000_39ce, 0xe79e_4cf9, 0x5dcf_194e),
        (0, 0, 0, 0),
        (0x1000_007a, 0, 0, 0x045a_c39e),
        (0xffff_ffff, 0xffff_ffff, 0xffff_ffff, 0x97df_97df),
        (0x1000_007a, 0x1000_39ce, 0, 0x98a7_d260),
        (0x1234_5678, 0x9abc_def0, 0x1111_1111, 0x3a5f_ebb7),
    ];
    for (u1, u2, u3, checked) in cases {
        assert_eq!(UidCrc::new(u1, u2, u3).checked(), checked, "{u1:#x} {u2:#x} {u3:#x}");
    }
}

#[test]
fn uidcrc_bytes_match_recorded_hello_file() {
    let want = [
        0x7a, 0x00, 0x00, 0x10, 0xce, 0x39, 0x00, 0x10, 0xf9, 0x4c, 0x9e, 0xe7, 0x4e, 0x19,
        0xcf, 0x5d,
    ];
    assert_eq!(hello().bytes(), want);
}

#[test]
fn uidcrc_line_matches_experiment_13_stdout() {
    assert_eq!(hello().line(), "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e");
}

#[test]
fn uidcrc_args_match_recorded_usage() {
    let wine = Path::new("/usr/bin/wine");
    let exe = Path::new("/sdk/epoc32/tools/uidcrc.exe");
    assert_eq!(
        hello().wine_args(wine, exe, None),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/uidcrc.exe",
            "0x1000007a",
            "0x100039ce",
            "0xe79e4cf9",
        ]
    );
    assert_eq!(
        hello().wine_args(wine, exe, Some("out.uid")),
        [
            "/usr/bin/wine",
            "/sdk/epoc32/tools/uidcrc.exe",
            "0x1000007a",
            "0x100039ce",
            "0xe79e4cf9",
            "out.uid",
        ]
    );
}

#[test]
fn normalize_strips_crlf() {
    assert_eq!(
        UidCrc::normalize_stdout(b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n"),
        "0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e"
    );
}

#[test]
fn uidcrc_matches_wine_hello_crlf() {
    let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x5dcf194e\r\n";
    assert!(hello().matches_wine(out));
}

#[test]
fn uidcrc_matches_wine_rejects_wrong_checked() {
    let out = b"0x1000007a 0x100039ce 0xe79e4cf9 0x00000000\n";
    assert!(!hello().matches_wine(out));
}
```

- [ ] **Step 2:** `cargo test -p symdev-build uidcrc --offline` FAIL (`UidCrc` missing / free functions still the only API)
- [ ] **Step 3:** Implement `UidCrc` methods. Move T1 algorithm onto the type. `normalize_stdout` + `matches_wine` as specified. Remove public free functions. Export `UidCrc` from `lib.rs`.
- [ ] **Step 4:** PASS `cargo test -p symdev-build uidcrc --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Wrap uidcrc in UidCrc and compare Wine CRLF stdout.`

---

## Self-review

- No public free functions in `uidcrc.rs`. No Wine spawn. No clap. No package wiring. No makekeys.
