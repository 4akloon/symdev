# T2 SIS unsigned — task 1 report

Date: 2026-09-18
Branch: `t2-sis-unsigned`
Worktree: `/home/genius/projects/symdev-t2-sis-unsigned` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` at `35e481c` (untouched)

## Result

**hello.sis matched exactly.** `SisUnsigned::bytes()` equals the frozen 4000-byte experiment-7 file.

Layout: 16-byte `SisUid` + `SisField` KIND 12 whose payload is concatenated padded fields type 34, 35, 3, 30. Checksums are live `SisChecksum34::of(&compressed.field())` and `SisChecksum35::of(&data.field())`. Type 3 zlib is `SisCompressed::zlib` of the 548-byte type-13 `SisController` field. Type 30 reuses `SisData`.

Compressor: `flate2` 1.1.10 `ZlibEncoder` + `Compression::new(6)`.

| Setting | Match? |
|---|---|
| Python `zlib.compress` default (level 6, wbits 15) | yes (283-byte stream) |
| `flate2` default `rust_backend` (miniz_oxide) level 6 | no (type-34 `3a 02`) |
| `flate2` `features = ["zlib-rs"]` level 6 | no (type-34 `9b 1f`) |
| `flate2` `default-features = false, features = ["zlib"]` (system libz / `libz-sys`) level 6 | **yes** (4000-byte file) |

No opaque zlib pin. No Wine on this type. Not wired into `SisPackage` / clap. No `.sis` committed.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `35e481c22e776bfe85df7f6604b2413bca9faff2` |
| Spec + plan + experiment 34 | `d4ff632598afac3f39cacff0c912de6a90e4f16e` |
| Encode | `6563305ce720d8f425bda2f52f55ee3996214e50` |
| Branch HEAD (before this report commit) | `6563305ce720d8f425bda2f52f55ee3996214e50` |

Not merged to main. Feature branch only.

## Types

`SisUnsigned` in `crates/symdev-build/src/sis/unsigned.rs`. `KIND` is `SisField::CONTROLLER` (12). `new(uid, compressed, data)`, `payload`, `field` (`SisEncode`), `bytes` = uid + field. `SisCompressed::zlib` compresses with flate2/libz level 6. Append-only exports on `sis/mod.rs` and crate `lib.rs`.

## Tests

TDD: tests failed with missing `SisUnsigned` / `SisCompressed::zlib`, then passed.

`cargo test --workspace --offline`:

- `symdev-build` 94 passed (2 new: full-file equality, live checksums `5c 9e` / `64 03`)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

Tests embed hex via `include_str!("testdata/hello_sis.hex")`. They do not read `.sis` from disk.

## Docs

- Spec: `docs/superpowers/specs/2026-09-18-symdev-t2-sis-unsigned-design.md`
- Plan: `docs/superpowers/plans/2026-09-18-symdev-t2-sis-unsigned.md`
- Experiment 34 (append-only): `docs/research/experiment-backlog.md`
