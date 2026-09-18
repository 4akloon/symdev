# T2 SIS checksums — task 1 report

Date: 2026-09-18
Branch: `t2-sis-checksums`
Worktree: `/home/genius/projects/symdev-t2-sis-checksums` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` at `ce7e19e` (untouched)

## Result

**Algorithm derived.** Type 34/35 are EPOC CRC16 init 0 (same byte step as experiment 13 `UidCrc`) over the padded inner `SisField::bytes()`:

- type 34 ← type-3 field (hello SIS 304 bytes including one pad → `5c 9e`)
- type 35 ← type-30 field (3648 bytes, identical on SIS and SISX → `64 03`)

Confirmed on frozen `hello.sis` and `hello.sisx` (type 34 changes to `01 c4` on SISX; type 35 unchanged). CRC of payload-only, zlib-only, inflated type 13, type-12 without checksums, file after UID, and the type-3 field without pad all failed.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `ce7e19e51c153b558f9430edc5385a565fa4284e` |
| Spec + experiment 32 | `a79620b4adb9099fc7320ce5b9ef4b738ab553eb` |
| Encode | `b194404f2a65981c38b555a46f3d0d1ad2e1a69e` |
| Branch HEAD (before this report commit) | `b194404f2a65981c38b555a46f3d0d1ad2e1a69e` |

Not merged to main. Feature branch only.

## Types

`SisChecksum34` (KIND 34) and `SisChecksum35` (KIND 35) in `crates/symdev-build/src/sis/checksum.rs`. `new([u8; 2])`, `of(&SisField)`, `payload`, `field`. Append-only exports on `sis/mod.rs` and crate `lib.rs`.

## Tests

TDD: tests failed with missing types, then passed.

`cargo test --workspace --offline`:

- `symdev-build` 88 passed (3 new: field goldens for 34/35, `of` on constructed hello type-3 `SisCompressed` field matches `5c 9e`)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

Type 35 hello value is pinned (`64 03`). `of` is the same CRC; type-30 encoder is a later slice.

## Docs

- Spec: `docs/superpowers/specs/2026-09-18-symdev-t2-sis-checksum-design.md`
- Plan: `docs/superpowers/plans/2026-09-18-symdev-t2-sis-checksum.md`
- Experiment 32 (append-only): `docs/research/experiment-backlog.md`
