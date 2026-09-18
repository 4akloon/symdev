# T3 rcomp body — native RSC encode after the UID header

Date: 2026-09-18
Branch: `t3-rcomp-body`
Worktree: `/home/genius/worktrees/symdev/t3-rcomp-body`
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)
Base: `origin/main` `cd9e24f66f4bdeccd0970694c8798dd49a9a0986`

## Result

Native `Rsc::bytes()` byte-equals both Wine rcomp Unicode goldens (driveinfo 74 bytes, filebrowse 109). Behavior is on types: `RscLtext16`, `RscAppRegistration`, `RscResource`, `Rsc`. Wine/argv stay on `RcompTool`. No RSS parse, no `.rsg`, no `START RESOURCE` / CLI wiring.

## Free functions

None added, none deleted, none moved. This slice did not introduce `pub fn encode_*` / `parse_*` / `crc_*`. Packing is `RscPacked` methods (private). Encode is `RscLtext16::bytes`, `RscAppRegistration::resource` / `bytes`, `Rsc::bytes`. Test-only `parse_hex` in `rcomp/mod.rs` was already there from the UID slice.

## SHAs

| What | SHA |
|---|---|
| origin/main base | `cd9e24f66f4bdeccd0970694c8798dd49a9a0986` |
| Frozen `driveinfo_reg.rsc` | SHA-256 `10bd8e607b9f166629ac1e9285d6abbad24e88972a885a15062f8680d575b7d8` (74 bytes) |
| Frozen `filebrowseapp_reg.rsc` | SHA-256 `437dbc17eef7b26d9650917b408d22922f96e7ed888e2916035f56eb010f0b60` (109 bytes) |

Feature-branch tip SHA is the report commit on this branch.

## Body layout (experiment 42)

After the 16-byte `RscUid` (UID1 `0x101f4a6b`, UID2 `0x101f8021`):

1. **4-byte header:** `00`, largest uncompressed resource size as `u8` (driveinfo `0x3b` = 59, filebrowse `0x7e` = 126), then `00 01` (constant on these two single-resource files).
2. **Packed resource** starting at file offset 20.
3. **Index** at the end: little-endian `u16` offsets from the start of the file, one start plus one end (driveinfo `14 00 46 00`, filebrowse `14 00 69 00`).

`APP_REGISTRATION_INFO` uncompressed (`-u`): `LONG`/`LLINK` 4 bytes, empty `LText16` one `0x00`, non-empty `LText16` is `len`, pad `0x00`, UTF-16LE. Packed form: leading `0x00`, then `(u8 count, literals)` runs; each Latin-1 `LText16` puts `len` in the preceding literals and emits `len` plus 8-bit chars (`0c 0c DriveInfoApp`).

In-crate hex unchanged: `crates/symdev-build/src/rcomp/testdata/*.rsc.hex`. No `.rsc` binaries committed.

## Tests

TDD: `Rsc` / `RscAppRegistration` / `RscLtext16` missing (E0433), then green.

`cargo test --workspace --offline`:

- `symdev-build` 127 passed (7 new: driveinfo full file, filebrowse full file, LText16 empty, LText16 DriveInfoApp UTF-16, driveinfo 59-byte uncompressed + header/index, filebrowse 126-byte uncompressed + header/index, LText16 length 256 error)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. `rustc 1.98.1`. Crate-root `pub use` append-only (`Rsc*` after existing `RcompTool, RscUid`). `mod rcomp` already alphabetical.

## Remaining unknowns

- Header trailing `00 01`: flags `0x0100` vs resource count 1 (only one `RESOURCE` in each golden).
- Uncompressed size ≥ 256 (header stores size in one byte here).
- Non-Latin-1 `LText16` packed form (goldens are ASCII; `RscPacked` errors).
- Multiple `RESOURCE` blocks (index longer than two `u16`s).
- Non-empty `LEN WORD STRUCT[]`, non-zero `LLINK`, `group_name`, embeddability other than 0.
- Non-empty `.rsg` (`NAME`), RSS parse, `START RESOURCE` / clap / `GcceBuild` wiring.

No merge to `main`. Worktree kept.
