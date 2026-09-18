# T2 SISX compose — type 39 in controller, wrap like SisUnsigned

Date: 2026-09-18
Branch: `t2-sisx`
Worktree: `/home/genius/worktrees/symdev/t2-sisx` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)

## Result

**hello.sisx matched exactly.** `SisUnsigned::bytes()` equals the frozen 5172-byte experiment-8 file when the controller carries recorded type 39.

Optional `SisSignatures39` on `SisController` (`signatures: None` by default; `with_signatures` inserts type 39 immediately before type 40). The outer file is still `SisUnsigned`: UID + type 12 of live checksums 34/35 + zlib type 3 + type 30. No second type-12 wrapper. Unsigned `hello.sis` tests still pass.

Compressor: same `flate2` 1.1.10 `ZlibEncoder` + `Compression::new(6)` + system libz as unsigned SIS.

| File | Bytes | Type 34 | Type 12 n |
|---|---|---|---|
| hello.sis (`signatures: None`) | 4000 | `5c 9e` | 3976 |
| hello.sisx (recorded type 39) | 5172 | `01 c4` | 5148 |

Type 35 stays `64 03`. Type 30 is identical. No Wine. No DSA invented. No `.cer` / `.key` / `.sisx` committed. Not wired into `SisPackage` / clap.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `99d942718ee79514af3338da8a9de90b63f3663d` |
| Encode + experiment 37 | `4a9b65860dff9d3f1aee385fc740ff9bafb877b7` |
| Frozen hello.sis | SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232` |
| Frozen hello.sisx | SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea` |

Not merged to main. Feature branch only.

## Types

`SisController` in `crates/symdev-build/src/sis/controller.rs` gained `pub signatures: Option<SisSignatures39>`. `new(...)` still takes the unsigned children and sets `signatures: None`. `with_signatures` stores the recorded type-39 field and `payload()` concatenates it before the type-40 trailer.

SISX tests build the same hello children as unsigned SIS, call `with_signatures` with the experiment-36 `SisSignatures39` (blobs from `testdata/hello_type39.hex`), then `SisCompressed::zlib` + `SisUnsigned::new`. Full file hex is `testdata/hello_sisx.hex` (`include_str`, not a `.sisx`).

## Tests

TDD: tests failed with missing `with_signatures`, then passed.

`cargo test --workspace --offline`:

- `symdev-build` 100 passed (3 new: signed controller inserts type 39 before type 40; full-file SISX equality; live checksums `01 c4` / `64 03`)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. Tests do not read `.sis` / `.sisx` / `.cer` / `.key` from disk. Existing `hello_unsigned_bytes_match_experiment_34` still equals the 4000-byte SIS.

## Docs

- Experiment 37 (append-only): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t2-sisx-report.md`
