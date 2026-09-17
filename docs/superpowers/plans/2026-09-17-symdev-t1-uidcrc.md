# T1 `uidcrc` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Clean-room `uidcrc` matching experiment 13 goldens, plus recorded Wine argv. No Wine in default tests. No clap verb.

**Architecture:** `crates/symdev-build/src/uidcrc.rs`. Pure functions + argv helper. Not wired into `package`.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-17-symdev-t1-uidcrc-design.md](../specs/2026-09-17-symdev-t1-uidcrc-design.md)
- Do not copy `uidcrc` C sources. Do not invent argv.
- Goldens from experiment 13 only.
- Never `-fPIC`. No E52 claim. Branch `t1-uidcrc`.
- TDD. Author `Yevhenii Aleksiuk` `<alexiuk.genius@gmail.com>`. Do not write git config.
- Default tests do not spawn Wine.

## File structure

- Create: `crates/symdev-build/src/uidcrc.rs`
- Modify: `crates/symdev-build/src/lib.rs` — `mod uidcrc;` and re-exports

---

### Task 1: `uid_checked` / bytes / line

**Files:** Create `uidcrc.rs`; export from `lib.rs`.

**Interfaces:** `uid_checked`, `uidcrc_bytes`, `uidcrc_line` as in the spec.

- [ ] **Step 1: Failing tests** using the six experiment-13 triples; hello bytes `7a000010ce390010f94c9ee74e19cf5d`
- [ ] **Step 2–4:** TDD
- [ ] **Step 5: Commit** `Match Wine uidcrc.exe checked UIDs from experiment 13 goldens.`

---

### Task 2: Wine argv helper

**Files:** Extend `uidcrc.rs`.

**Interface:** `uidcrc_args(wine, uidcrc, uid1, uid2, uid3, outfile: Option<&str>) -> Vec<String>`

Without outfile: wine, exe, three `0x%08x` tokens.
With outfile: those plus the filename.

Fixture paths `/usr/bin/wine` and `/sdk/epoc32/tools/uidcrc.exe` (not `/home/genius`).

- [ ] **Step 1–4:** TDD
- [ ] **Step 5: Commit** `Copy recorded Wine uidcrc argv.`
