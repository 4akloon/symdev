# T2 native signsis — replace Wine `signsis` with SISX compose

Date: 2026-09-18
Branch: `t2-native-signsis`
Worktree: `/home/genius/worktrees/symdev/t2-native-signsis` (kept)
Primary checkout: `/home/genius/projects/symdev` still `main` (untouched)

## Result

**Signed-bytes rule derived from frozen goldens (SignSIS C not copied).** DSA-SHA1 verifies the experiment-8 type-36 blob over SHA-1 of the uncompressed type-13 payload **without type 39 and without type 40** (hello: 528 bytes = types 14, 16, 15, 17, 19, 28). Digest `f3fca5ab077413d247c256bb81824a053b221caa`.

`SisPackage` writes unsigned `.sis` natively, then SISX via `SisController::with_signatures` plus a live SHA-1 + DSA signature from the makekeys cert/key. **Wine `signsis` is not spawned.** Wine `makekeys` remains when cert/key are absent. Frozen `hello.sisx` is still pinned by the recorded type-39 blob. Live sign of the same controller does not byte-equal that file (RFC 6979 `k` ≠ SignSIS random `k`; new cert dates would also move).

No E52. Never `-fPIC`. No clap verbs. Default `cargo test` does not require Wine.

## SHAs

| What | SHA |
|------|-----|
| origin/main base | `e587a94a17667815d02782d6835951cc2991e194` |
| Native sign + experiment 39 | `70a5ca638a1125c69397859a40a3474822ea8816` |
| Frozen hello.sis | SHA-256 `06f39722f5f911d59c119d126c223eabd7b3ec4c81b3175bbebc3f3eb7855232` |
| Frozen hello.sisx | SHA-256 `fe6bdd338c7a7803a031a6f45e34d838f04843b9d3eac2bb3f475bf4098ba1ea` |

Feature-branch tip SHA is the report commit on this branch.

## Algorithm

`SisController::signed_bytes()` concatenates info/words16/languages/products/words19/files (no signatures, no type-40 trailer). Sign: SHA-1, DSA (`dsa` 0.6, RFC 6979), OID `1.2.840.10040.4.3`, type-37 payload = DER SEQUENCE of `r`,`s` padded to 4 bytes. Cert DER from makekeys PEM (or raw DER) in type 22. Encrypted `BEGIN DSA PRIVATE KEY` uses OpenSSL `DES-EDE3-CBC` + MD5 `EVP_BytesToKey` (makekeys format). PKCS#8 PEM also loads.

## Failed hash candidates

Unsigned type-13 field (548) and payload including type 40 (540); SISX type-13 field/payload; SISX header + unsigned payload; SISX with type 39 or the 48-byte blob zeroed; compressed type 3 field/payload/zlib (SIS and SISX); type 12 ± checksums; file after UID; whole `.sis`/`.sisx`; UID; exe; type 30; SHA-1 of those as the DSA message; SHA-256 left-160; little-endian hash integer; other prefixes/suffixes.

## Wine

| Tool | Status |
|------|--------|
| `makesis` | not spawned (already native unsigned SIS) |
| `makekeys` | Wine, unless `signing.cert` + `signing.key` exist |
| `signsis` | **not spawned** |

`SisTools::from_env` still requires `SYMDEV_EPOCROOT` (makekeys PE path). `signsis_args` remains as the experiment-8 recorded argv helper.

## Tests

TDD: `signed_bytes` failed (missing method), then passed. Frozen DSA verify of the 528-byte prefix passed against `testdata/hello_type39.hex`. Encrypted-PEM sign failed until 3DES decrypt. Package SISX failed while Wine `signsis` still ran, then passed.

`cargo test --workspace --offline`:

- `symdev-build` 114 passed (signed-bytes, frozen DSA verify, traditional/encrypted/PKCS#8 live sign, native SISX without Wine `signsis`, encode_signed_sisx not hello golden). Experiment-5 `hello.key` live sign returns early unless `SYMDEV_SIGN_PASSWORD` is set.
- `symdev` unittests 3 passed
- `cli` 19 passed
- `symdev-core` 5 passed
- `symdev-manifest` 17 passed

No Wine. No `.sis` / `.sisx` / `.cer` / `.key` committed (throwaway DSA fixtures are hex).

## Docs

- Experiment 39 (append-only): `docs/research/experiment-backlog.md`
- This report: `.superpowers/sdd/t2-native-signsis-report.md`
