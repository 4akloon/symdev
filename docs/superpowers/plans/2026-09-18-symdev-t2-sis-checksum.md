# T2 SIS Checksums Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisChecksum34` and `SisChecksum35` encode the frozen hello type-34/35 two-byte fields and recompute them with EPOC CRC16 over padded inner field bytes.

**Architecture:** New types in `crates/symdev-build/src/sis/checksum.rs`. `new` pins `[u8; 2]`. `of` CRCs `inner.bytes()`. No Wine. Export from `sis/mod.rs` and crate root.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-checksum-design.md](../specs/2026-09-18-symdev-t2-sis-checksum-design.md)
- Types + methods. These types must not take Wine paths. Do not read a clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-checksums` in an isolated git worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config. Do not amend.
- Do not copy MakeSIS C. Two types with `const KIND`. Algorithm is EPOC CRC16 init 0 over padded type-3 / type-30 field bytes.
- `sis/mod.rs` and crate-root `lib.rs` diffs are append-only exports.

## File structure

- Create: `crates/symdev-build/src/sis/checksum.rs`
- Modify: `crates/symdev-build/src/sis/mod.rs` — add `mod checksum;` and `pub use checksum::{SisChecksum34, SisChecksum35}`
- Modify: `crates/symdev-build/src/lib.rs` — add `SisChecksum34, SisChecksum35` to the `sis` re-export list
- Modify: `docs/research/experiment-backlog.md` — append experiment 32 only

---

### Task 1: `SisChecksum34` / `SisChecksum35`

**Files:** `sis/checksum.rs`, `sis/mod.rs` exports, `lib.rs` re-exports, experiment 32.

**Interfaces:** as in the spec.

- Consumes: `SisField`, `SisCompressed` (tests only)
- Produces: `SisChecksum34::new` / `of` / `payload` / `field` (KIND 34); `SisChecksum35::new` / `of` / `payload` / `field` (KIND 35)

- [ ] **Step 1: Failing test** — unit tests `hello_checksum34_field_matches_experiment_32`, `hello_checksum35_field_matches_experiment_32`, and `hello_checksum34_of_type3_field_matches_experiment_32`.

```rust
#[test]
fn hello_checksum34_field_matches_experiment_32() {
    let f = SisChecksum34::new([0x5c, 0x9e]).field();
    assert_eq!(
        f.bytes(),
        [0x22, 0, 0, 0, 2, 0, 0, 0, 0x5c, 0x9e, 0, 0]
    );
    assert_eq!(SisChecksum34::KIND, 34);
}

#[test]
fn hello_checksum35_field_matches_experiment_32() {
    let f = SisChecksum35::new([0x64, 0x03]).field();
    assert_eq!(
        f.bytes(),
        [0x23, 0, 0, 0, 2, 0, 0, 0, 0x64, 0x03, 0, 0]
    );
    assert_eq!(SisChecksum35::KIND, 35);
}

#[test]
fn hello_checksum34_of_type3_field_matches_experiment_32() {
    let inner = SisCompressed::new(548, hello_type3_zlib().to_vec()).field();
    assert_eq!(inner.bytes().len(), 304);
    assert_eq!(SisChecksum34::of(&inner).value, [0x5c, 0x9e]);
}
```

`hello_type3_zlib` is the 283-byte zlib stream copied from experiment 32 (frozen `hello.sis` type-3 payload from offset 12). Do not `include_bytes!` a `.sis`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --offline --manifest-path crates/symdev-build/Cargo.toml -- sis::checksum`
Expected: FAIL because `checksum` module is missing.

- [ ] **Step 3: Write minimal implementation**

`of` uses the experiment-13 CRC16 byte step on `inner.bytes()`, then `to_le_bytes()`. `field()` is `SisField::new(Self::KIND, self.payload().to_vec())`. No Wine. No type 12. No type 30 encoder.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --workspace --offline`
Expected: PASS, including the three new tests.

- [ ] **Step 5: Commit**

```bash
git add crates/symdev-build/src/sis/checksum.rs crates/symdev-build/src/sis/mod.rs crates/symdev-build/src/lib.rs
git commit -m "Encode SIS type-34 and type-35 checksums from hello goldens."
```

## Self-review

- Spec coverage: two KIND wrappers, `new`/`of`/`field`, hello goldens, algorithm from experiment 32, append-only exports, no MakeSIS C, no `.sis` in git.
- No placeholders.
- Names `SisChecksum34` / `SisChecksum35` / `KIND` / `of` match the spec.

## Notes

- KIND 34 and 35, not 12. Reuse `SisCompressed` only in the type-34 `of` test. No type 30 encoder. No inflate crate. No package wiring. No `.sis` in git.
