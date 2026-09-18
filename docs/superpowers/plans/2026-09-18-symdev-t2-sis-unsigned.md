# T2 SIS Unsigned Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisUnsigned::bytes()` equals frozen hello.sis: UID + type-12 field of live checksums 34/35, zlib type-3 controller, type-30 data.

**Architecture:** New type in `crates/symdev-build/src/sis/unsigned.rs`. Reuse `SisUid`, `SisChecksum34`/`35::of`, `SisCompressed`, `SisData`, `SisEncode`. One zlib crate if a setting matches; else pin type-3 zlib opaque. No Wine. Export from `sis/mod.rs` and crate root.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. At most one zlib crate (`flate2` or `miniz_oxide`).

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-unsigned-design.md](../specs/2026-09-18-symdev-t2-sis-unsigned-design.md)
- Types + methods. This type must not take Wine paths. Do not read a clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg` / `.exe`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-unsigned` in an isolated git worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config. Do not amend.
- Do not copy MakeSIS C. Name is not `SisFile`. `sis/mod.rs` and crate-root `lib.rs` diffs are append-only exports.

## File structure

- Create: `crates/symdev-build/src/sis/unsigned.rs`
- Create: `crates/symdev-build/src/sis/testdata/hello_sis.hex` (hex of the 4000-byte file; not a `.sis`)
- Modify: `crates/symdev-build/src/sis/compressed.rs` — zlib helper on `SisCompressed` if a crate setting matches
- Modify: `crates/symdev-build/src/sis/mod.rs` — add `mod unsigned;` and `pub use unsigned::SisUnsigned`
- Modify: `crates/symdev-build/src/lib.rs` — append `SisUnsigned` to the `sis` re-export list
- Modify: `crates/symdev-build/Cargo.toml` — one zlib crate if needed
- Modify: `docs/research/experiment-backlog.md` — append experiment 34 only

---

### Task 1: `SisUnsigned`

**Files:** as above.

**Interfaces:** as in the spec.

- Consumes: `SisUid`, `SisField`/`SisEncode`, `SisChecksum34`, `SisChecksum35`, `SisCompressed`, `SisController`, `SisData`
- Produces: `SisUnsigned::new` / `payload` / `field` / `bytes` (KIND 12)

- [ ] **Step 1: Failing tests** — dump frozen `$HOME/src/symdev-experiment-5/hello.sis` into hex testdata. Unit tests in `unsigned.rs` for full-file equality, live checksums, and KIND 12.

- [ ] **Step 2: Run test to verify it fails** (module missing)

- [ ] **Step 3: Minimal implementation** — compose children; compress type-13 with a matching zlib setting or pin opaque zlib; checksums via `of`.

- [ ] **Step 4: `cargo test --workspace --offline`**

- [ ] **Step 5: Commit**
