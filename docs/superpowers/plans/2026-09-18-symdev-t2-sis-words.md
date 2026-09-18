# T2 SIS Words Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisWords` / `SisWords16` / `SisWords19` / `SisU32` encode the frozen hello type-16, type-19, and type-40 fields.

**Architecture:** New types in `crates/symdev-build/src/sis/words.rs`. Reuse `SisField`. Type 2 of u32s is raw LE concat, not `SisArray`. No Wine. Append-only exports from `sis/mod.rs` and crate root.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-words-design.md](../specs/2026-09-18-symdev-t2-sis-words-design.md)
- Types + methods. These types must not take Wine paths.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-words` in an isolated worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config. Do not amend. Do not merge to main.
- Do not name these Options/Languages. Type 16’s `0x21` is not the EN language id. Do not invent C names.
- Do not touch `sis/product.rs`. `sis/mod.rs` and `lib.rs` diffs are append-only exports.

## File structure

- Create: `crates/symdev-build/src/sis/words.rs`
- Modify: `crates/symdev-build/src/sis/mod.rs` — append `mod words;` and `pub use words::{SisU32, SisWords, SisWords16, SisWords19}`
- Modify: `crates/symdev-build/src/lib.rs` — append-only re-export of those four names from `sis`

---

### Task 1: `SisWords` / `SisWords16` / `SisWords19` / `SisU32`

**Files:** `sis/words.rs`, `sis/mod.rs` append-only exports, `lib.rs` append-only re-exports.

**Interfaces:**

- Consumes: `SisField::new(kind, payload)` and `SisField::bytes()`
- Produces: `SisWords::new` / `payload` / `field` (KIND 2); `SisWords16::new` / `field` (KIND 16); `SisWords19::new` / `field` (KIND 19); `SisU32::new` / `payload` / `field` (KIND 40)

- [ ] **Step 1: Failing tests**

```rust
use super::field::SisField;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hello_words_payload_is_raw_u32_concat() {
        assert_eq!(SisWords::new(vec![0x21]).payload(), [0x21, 0, 0, 0]);
        assert_eq!(SisWords::KIND, 2);
    }

    #[test]
    fn hello_words16_field_matches_experiment_25() {
        let f = SisWords16::new(SisWords::new(vec![0x21])).field();
        assert_eq!(
            f.bytes(),
            [
                0x10, 0, 0, 0, 0x0c, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 0x21, 0, 0, 0
            ]
        );
        assert_eq!(SisWords16::KIND, 16);
    }

    #[test]
    fn hello_words19_field_matches_experiment_26() {
        let f = SisWords19::new(SisWords::new(vec![0x14])).field();
        assert_eq!(
            f.bytes(),
            [
                0x13, 0, 0, 0, 0x0c, 0, 0, 0, 2, 0, 0, 0, 4, 0, 0, 0, 0x14, 0, 0, 0
            ]
        );
        assert_eq!(SisWords19::KIND, 19);
    }

    #[test]
    fn hello_u32_field_matches_experiment_27() {
        let f = SisU32::new(0).field();
        assert_eq!(f.bytes(), [0x28, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(SisU32::KIND, 40);
    }
}
```

- [ ] **Step 2:** `cargo test -p symdev-build sis::words --offline` FAIL (types missing)
- [ ] **Step 3:** Implement as specified. `SisWords::payload` concatenates `to_le_bytes()`. Wrappers use `SisField::new(KIND, words.field().bytes())`. `SisU32` is one LE u32, not nested in type 2. No Wine. No Options/Languages names.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis::words --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-16, type-19, and type-40 words from hello goldens.`

---

## Self-review

- Spec coverage: type-2 raw u32 concat, type 16 wrap of `0x21`, type 19 wrap of `0x14`, type 40 value `0`, append-only exports, no product.rs, no Wine, no language tables.
- No placeholders.
- Type names match the spec: `SisWords`, `SisWords16`, `SisWords19`, `SisU32`.
