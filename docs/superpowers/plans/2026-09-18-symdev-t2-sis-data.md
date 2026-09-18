# T2 SIS Data Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `SisData` encodes the frozen hello type-30 field as nested type 2 / 31 / 2 / 32 / `SisCompressed` (algorithm 0, raw `hello.exe`).

**Architecture:** New types in `crates/symdev-build/src/sis/data.rs`. Reuse `SisField`, `SisArray`, and `SisCompressed` public fields with algorithm `0`. Pin the 3648-byte golden as hex testdata. No Wine. Export from `sis/mod.rs` and crate root.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. No new crates.

## Global Constraints

- Spec: [2026-09-18-symdev-t2-sis-data-design.md](../specs/2026-09-18-symdev-t2-sis-data-design.md)
- Types + methods. These types must not take Wine paths. Do not read a clock.
- Do not invent argv. Do not spawn Wine in default `cargo test`.
- Do not inflate. Do not add crates. Do not wire into `package` or clap. Do not commit `.sis` / `.sisx` / `.pkg` / `.exe`.
- Never `-fPIC`. No E52 claim. Work on branch `t2-sis-data` in an isolated git worktree.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config. Do not amend.
- Reuse `SisArray` and `SisCompressed`. Do not invent C names. Do not encode types 34/35. Experiment 32 is reserved.
- Do not copy MakeSIS C. Do not call `SisCompressed::new` for hello file bytes (`new` sets algorithm 1).

## File structure

- Create: `crates/symdev-build/src/sis/data.rs`
- Create: `crates/symdev-build/src/sis/testdata/hello_type30.hex` (hex of the 3648-byte type-30 field; not a `.sis`)
- Modify: `crates/symdev-build/src/sis/mod.rs` — add `mod data;` and `pub use data::{SisData, SisData31, SisData32}`
- Modify: `crates/symdev-build/src/lib.rs` — add those three names to the `sis` re-export list
- Modify: `docs/research/experiment-backlog.md` — append experiment 33 only (do not take 32)

---

### Task 1: `SisData32` / `SisData31` / `SisData`

**Files:** `sis/data.rs`, `sis/testdata/hello_type30.hex`, `sis/mod.rs` exports, `lib.rs` re-exports, experiment 33.

**Interfaces:** as in the spec.

- Consumes: `SisField`, `SisArray`, `SisCompressed`
- Produces: `SisData32` KIND 32; `SisData31` KIND 31; `SisData` KIND 30

- [ ] **Step 1: Failing tests** — dump the type-30 field from frozen `$HOME/src/symdev-experiment-5/hello.sis` into hex testdata (do not commit the `.sis`). Unit tests in `data.rs`:

```rust
fn parse_hex(s: &str) -> Vec<u8> {
    let hex: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}

fn hello_type30_golden() -> Vec<u8> {
    parse_hex(include_str!("testdata/hello_type30.hex"))
}

fn hello_exe_bytes() -> Vec<u8> {
    hello_type30_golden()[60..].to_vec()
}

fn hello_compressed() -> SisCompressed {
    SisCompressed {
        algorithm: 0,
        uncompressed_size: 3588,
        reserved: 0,
        data: hello_exe_bytes(),
    }
}

fn hello_data() -> SisData {
    SisData::new(SisArray::new(vec![SisData31::new(SisArray::new(vec![
        SisData32::new(hello_compressed()).field(),
    ]))
    .field()]))
}

#[test]
fn hello_data32_header_matches_experiment_33() {
    let f = SisData32::new(hello_compressed()).field();
    assert_eq!(&f.bytes()[..8], &[0x20, 0, 0, 0, 0x18, 0x0e, 0, 0]);
    assert_eq!(f.payload.len(), 3608);
    assert_eq!(SisData32::KIND, 32);
}

#[test]
fn hello_data_payload_hash_matches_experiment_33() {
    let data = hello_exe_bytes();
    assert_eq!(data.len(), 3588);
    assert_eq!(
        &data[..16],
        &[
            0x7a, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0xf9, 0x4c, 0x9e, 0xe7, 0xb0, 0x08,
            0x32, 0xc1
        ]
    );
    assert_eq!(
        &data[data.len() - 16..],
        &[
            0xdc, 0x95, 0x8a, 0x46, 0xa2, 0x45, 0xc4, 0x8c, 0x39, 0x38, 0x35, 0xbb, 0x91, 0x10,
            0x7f, 0xff
        ]
    );
    assert_eq!(
        HELLO_DATA_SHA1,
        [
            0x3a, 0x23, 0xe7, 0xe7, 0xe6, 0x0e, 0xd9, 0x73, 0x54, 0x53, 0x4b, 0x2a, 0x77, 0xe5,
            0x65, 0xcd, 0x64, 0xea, 0x39, 0x70
        ]
    );
}

#[test]
fn hello_data_field_matches_experiment_33() {
    let golden = hello_type30_golden();
    assert_eq!(&golden[..8], &[0x1e, 0, 0, 0, 0x38, 0x0e, 0, 0]);
    assert_eq!(golden.len(), 3648);
    assert_eq!(hello_data().payload().len(), 3640);
    assert_eq!(hello_data().field().bytes(), golden);
    assert_eq!(SisData::KIND, 30);
    assert_eq!(SisData31::KIND, 31);
}
```

`HELLO_DATA_SHA1` is a pinned `[u8; 20]` const (host SHA-1 of the 3588-byte payload / `hello.exe`; same digest as experiment 29). Do not add a sha1 crate. Do not `include_bytes!` a `.sis`.

- [ ] **Step 2:** `cargo test -p symdev-build sis::data --offline` FAIL (type missing)
- [ ] **Step 3:** Implement as specified. `SisData32::payload()` is `compressed.field().bytes()`. `SisData31` / `SisData` payloads are `items.field().bytes()`. Reuse `SisArray` and `SisCompressed`. Algorithm `0` via public fields. No zlib. No Wine.
- [ ] **Step 4:** PASS `cargo test -p symdev-build sis::data --offline` then `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Encode SIS type-30 data from hello compressed file bytes.`

---

## Self-review

- KIND 30/31/32. Reuses `SisArray` and `SisCompressed` prefix. Algorithm 0 not DEFLATE. No inflate crate. No checksums 34/35. No package wiring. No `.sis` in git. Experiment 32 unused.
