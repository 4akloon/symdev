# T2 SIS Controller Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisController` encodes the frozen hello type-13 inflated body from existing child types including `SisFiles`.

**Architecture:** New type in `crates/symdev-build/src/sis/controller.rs`. Payload concatenates child `field().bytes()` in recorded order. No Wine. No inflate crate. Export from `sis/mod.rs` and crate root.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-controller-design.md](../specs/2026-09-18-symdev-t2-sis-controller-design.md)
- Types + methods. This type must not take Wine paths. Do not read a clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-controller` in an isolated git worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config. Do not amend.
- Reuse `SisFiles`. Do not pin type 28 as opaque bytes. Do not encode types 34/35/30 or type-3 zlib.
- `SisController::KIND` is 13. Do not change `SisField::CONTROLLER` (12).

## File structure

- Create: `crates/symdev-build/src/sis/controller.rs`
- Modify: `crates/symdev-build/src/sis/mod.rs` — add `mod controller;` and `pub use controller::SisController`
- Modify: `crates/symdev-build/src/lib.rs` — add `SisController` to the `sis` re-export list

---

### Task 1: `SisController`

**Files:** `sis/controller.rs`, `sis/mod.rs` exports, `lib.rs` re-exports.

**Interfaces:** as in the spec.

- Consumes: `SisInfo`, `SisWords16`, `SisLanguages`, `SisProducts`, `SisWords19`, `SisFiles`, `SisU32`, `SisField`
- Produces: `SisController::new` / `payload` / `field` (KIND 13)

- [ ] **Step 1: Failing test** — unit test `hello_controller_field_matches_experiment_31` constructs hello children with the existing pinned constructors (including `SisFiles`, not a byte blob) and asserts `field().bytes()` equals the 548-byte inflated type-13 field copied from experiment 31. Also `SisController::KIND == 13` and `payload().len() == 540`.

The 548-byte expected array is the host inflate of frozen `hello.sis` type-3 zlib (experiment 16), starting `0d 00 00 00 1c 02 00 00`. Copy it into the test; do not `include_bytes!` a `.sis`.

Hello `SisController::new` arguments:

```rust
SisController::new(
    SisInfo::new(
        SisPkgUid::new(0xe79e_4cf9),
        SisString::new("Vendor"),
        SisArray::new(vec![SisString::new("hello").field()]),
        SisArray::new(vec![SisString::new("Vendor-EN").field()]),
        SisVersion::new(1, 0, 24),
        SisDateTime::new(SisDate::new(2026, 8, 17), SisTime::new(15, 18, 24)),
    ),
    SisWords16::new(SisWords::new(vec![0x21])),
    SisLanguages::new(SisArray::new(vec![SisLanguage::new(1).field()])),
    SisProducts::new(SisArray::new(vec![SisProduct::new(
        SisPkgUid::new(0x1027_52ae),
        SisProductVersion::new(SisVersion::new(0, 0, 0)),
        SisArray::new(vec![SisString::new("S60ProductID").field()]),
    )
    .field()])),
    SisWords19::new(SisWords::new(vec![0x14])),
    SisFiles::new(
        SisArray::new(vec![SisFile::new(
            SisString::new("!:\\sys\\bin\\hello.exe"),
            SisString::new(""),
            SisWord41::new(0x000b_e000),
            SisHash::new(
                [1, 0x25, 0x14],
                [
                    0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77,
                    0xe5, 0x65, 0xcd, 0x64, 0xea, 0x39, 0x70,
                ],
            ),
            SisString::new(""),
            [3588, 0, 3588, 0, 0],
        )
        .field()]),
        SisWords::new(vec![0x0d]),
        SisWords::new(vec![0x1a]),
    ),
    SisU32::new(0),
)
```

- [ ] **Step 2:** `cargo test -p symdev-build sis::controller --offline` FAIL (type missing)
- [ ] **Step 3:** Implement as specified. `payload()` concatenates the seven children's `field().bytes()`. Reuse `SisFiles`. No zlib. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis::controller --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-13 controller body from hello child fields.`

---

## Self-review

- KIND 13 not 12. Reuses `SisFiles`. No opaque type 28. No inflate crate. No checksums/type 30. No package wiring. No `.sis` in git. No clock.
