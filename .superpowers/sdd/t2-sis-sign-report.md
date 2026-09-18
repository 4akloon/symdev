# T2 native signsis — research + first encode slice

Date: 2026-09-18
Branch: `t2-sis-sign`
Worktree: `/home/genius/worktrees/symdev/t2-sis-sign` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)

## Result

SISX does not add a sibling after type 12. UID and type-30 data are identical to SIS. The extra 1172 file bytes are a larger type-3 compressed controller. Inflated, that is still one type-13 field: the unsigned children (14, 16, 15, 17, 19, 28) then **type 39** (signatures) then type 40.

Type 39 is TLVs. DSA value and cert DER are recorded opaque `SisBlob37` payloads (makekeys dates / DSA would change on regenerate). Native RSA/DSA later needs the signed-bytes rule (not derived; SignSIS C not copied) plus makekeys cert dates.

Not wired into `SisController` or `SisPackage`. No Wine in default tests. No E52. Experiment 34 left for unsigned SIS compose.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `35e481c22e776bfe85df7f6604b2413bca9faff2` |
| Encode + experiment 36 | `2b89826c6b144806020ccdf66752163eaae03b95` |
| Frozen hello.sis | SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232` |
| Frozen hello.sisx | SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea` |

Not merged to main. Feature branch only.

## SISX layout (after UID)

Both files: 16-byte UID + one type 12. Type-12 children: 34, 35, 3, 30. Nothing after type 30.

| | SIS | SISX |
|--|-----|------|
| file | 4000 | 5172 |
| type 12 n | 3976 | 5148 |
| type 34 | `5c 9e` | `01 c4` |
| type 35 | `64 03` | `64 03` |
| type 3 n / occupied | 295 / 304 | 1467 / 1476 |
| type 3 uncompressed | 548 | 1868 |
| type 30 | identical 3648-byte field | identical |

Inflated type 13 payload: 528-byte shared prefix, then SISX type 39 `n=1312` (occupied 1320), then type 40 `u32 0`.

Type 39 nested: type 2 array of one type 36 (type 38 OID string `1.2.840.10040.4.3` + type 37 48-byte DSA blob) then type 22 wrapping type 37 cert DER (1171 bytes, SHA-1 `6698f484c9c64d0ddf44240520f0e6bd629acfd9`, equals `hello.cer` DER). Cert Not Before `2026-09-17 15:21:21 GMT` / Not After `2036-09-14 15:21:21 GMT`.

## Encoded vs opaque

Encoded as `SisEncode` value types in `crates/symdev-build/src/sis/signature.rs`:

- `SisAlgorithm38` KIND 38 — `SisString` OID
- `SisSignature36` KIND 36 — algorithm + blob
- `SisChain22` KIND 22 — one cert blob field
- `SisSignatures39` KIND 39 — `SisArray` of signatures + chain
- `SisBlob37` KIND 37 — **opaque** recorded DSA value (48 bytes, trailing two zeros inside n) and cert DER

Left for later: insert type 39 into type 13; type 12 compose; native DSA over the still-unknown signed bytes; `SisPackage`; Wine.

## Tests

TDD: tests failed with missing `SisSignatures39` / siblings, then passed.

`cargo test --workspace --offline`:

- `symdev-build` 95 passed (3 new: algorithm 38 field, signature 36 field, type 39 field vs `testdata/hello_type39.hex`)
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. Tests do not read `.sis` / `.sisx` / `.cer` / `.key` from disk. No `.cer` / `.key` committed.

## Docs

- Experiment 36 (append-only): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t2-sis-sign-report.md`

## Review

**Verdict: Approved** (after alphabetical `sis/mod.rs` exports; crate-root `lib.rs` list stays append-only).

Named check: `testdata/hello_type39.hex` equals type-39 field extracted from frozen `$HOME/src/symdev-experiment-5/hello.sisx` (5172 bytes, SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea`). Type-37 cert blob equals `hello.cer` DER (SHA-1 `6698f484c9c64d0ddf44240520f0e6bd629acfd9`). SISX extra is type 39 inserted before type 40 in inflated type 13, not a sibling after type 12.

Critical: none. Important: `mod signature` / `pub use signature` had been appended after `words`; rustfmt and the unsigned worktree keep `sis/mod.rs` alphabetical. Crate-root brace list already appended `SisAlgorithm38`…`SisSignatures39` after `SisWords19`.
