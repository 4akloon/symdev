# Task 1 report: T2 SIS type 30 data

## Status

DONE

## What I implemented

Clean-room encode of frozen hello type-30 data. Isolated git worktree `/home/genius/projects/symdev-t2-sis-data` on branch `t2-sis-data` from `origin/main` `ce7e19e`. Not merged to main.

Types in `crates/symdev-build/src/sis/data.rs`:

- `SisData32` KIND 32 — payload is `SisCompressed::field().bytes()`
- `SisData31` KIND 31 — payload is `SisArray::field().bytes()`
- `SisData` KIND 30 — payload is `SisArray::field().bytes()`

Hello file bytes use `SisCompressed` public fields with algorithm `0` (not `SisCompressed::new` / `DEFLATE`). No zlib crate. No Wine. No clap/package wiring. No types 34/35. Experiment 32 left for checksums.

## Structure found (experiment 33)

Frozen `$HOME/src/symdev-experiment-5/hello.sis` (4000 bytes). After UID, type 12 `n=3976`. Type 30 at controller offset 328, `n=3640`, field 3648 bytes. Nested:

| off | type | n |
|-----|------|---|
| 0 | 30 `SisData` | 3640 |
| 8 | 2 `SisArray` | 3632 |
| 16 | 31 `SisData31` | 3624 |
| 24 | 2 `SisArray` | 3616 |
| 32 | 32 `SisData32` | 3608 |
| 40 | 3 `SisCompressed` | 3600 |
| 48 | prefix | alg `0`, uncomp `3588`, reserved `0` |
| 60 | data | 3588 bytes, **equal** `hello.exe` |

Not zlib. SHA-1 of that payload: `3a23e7e7e60ed97354534b2a77e565cd64ea3970` (same as experiment 29). Inner type-2 payloads are TLV arrays, not `SisWords`.

## What I tested

TDD. Red: `cargo test -p symdev-build sis::data --offline` failed with `cannot find type SisData` (and 31/32). Green: three unit tests then workspace.

```
cargo test -p symdev-build sis::data --offline
# 3 passed (hello_data32_header, hello_data_payload_hash, hello_data_field)

cargo test --workspace --offline
# symdev-build 88 passed (was 85); cli 3; cli.rs 19; core 5; manifest 17; 0 failed
```

Tests embed hex via `include_str!("testdata/hello_type30.hex")`. They do not read `.sis` / `.exe` from disk. No `.sis` committed.

## TDD Evidence

- Red: unresolved `SisData` / `SisData31` / `SisData32` before production types existed.
- Green: reconstructed type-30 `field().bytes()` equals the 3648-byte dumped golden; header `1e 00 00 00 38 0e 00 00`; payload length 3640; data length 3588 + pinned SHA-1 + head/tail.

## Files changed

| File | Action |
|---|---|
| `docs/superpowers/specs/2026-09-18-symdev-t2-sis-data-design.md` | created (specify commit) |
| `docs/superpowers/plans/2026-09-18-symdev-t2-sis-data.md` | created (specify); steps checked |
| `docs/research/experiment-backlog.md` | append experiment 33 only |
| `crates/symdev-build/src/sis/data.rs` | created |
| `crates/symdev-build/src/sis/testdata/hello_type30.hex` | created (hex, not `.sis`) |
| `crates/symdev-build/src/sis/mod.rs` | append `mod data` + `pub use` |
| `crates/symdev-build/src/lib.rs` | append `SisData`, `SisData31`, `SisData32` |
| `.superpowers/sdd/t2-sis-data-task-1-report.md` | this report |

## Commits

- `b01881c` Specify SisData from experiment 33 type-30 goldens.
- Encode commit on this branch: `Encode SIS type-30 data from hello compressed file bytes.`

Author/committer env `Yevhenii Aleksiuk` / `38440891+4akloon@users.noreply.github.com`. No `git config`. No amend.

## Self-review

KIND 30/31/32. Reuses `SisArray` and `SisCompressed` prefix. Algorithm 0. No inflate crate. No checksums. No package wiring. No `.sis` in git. Experiment 32 unused. Worktree kept. Not merged to main.

## Issues or concerns

None that block the slice. `HELLO_DATA_SHA1` is a pinned host hash (no sha1 crate), same 20 bytes as experiment 29.
