# oop-main-fix — move landed free functions onto owning types

Date: 2026-09-18
Branch: `oop-main-fix`
Worktree: `/home/genius/worktrees/symdev/oop-main-fix`
Base: `origin/main` `cd9e24f66f4bdeccd0970694c8798dd49a9a0986`
Did not merge. Did not touch `/home/genius/worktrees/symdev/t3-rcomp-body`.

## Audit

T3 first slice (`RscUid`, `RcompTool`, `rcomp/testdata`) was already type+method. No free-fn moves.

T2 makekeys (`SelfSignedDsa`) already owned generate/PEM; leftovers were `utc` and the Wine dname string still living as `dname()`.

rules-oop leftovers (`parse_mmp`, `parse_bld_inf`, `render_pkg`, `dname`, `validate_password`, `write_pkg_file`, `existing_signing_pair`, `verify_dsa_sha1`) are the rest of this change.

## API moves (old free fn → type method)

| Old | New | Notes |
|-----|-----|--------|
| `parse_mmp(text)` | `Mmp::parse(text)` | Crate-root `pub use` of the free fn removed. |
| `parse_bld_inf(text)` | `BldInf::parse(text)` | Same; `ParseError` re-export kept. |
| `render_pkg(name, uid3, version, vendor)` | `SisPackage::pkg_text(&self)` | `pkg.rs` deleted. |
| `write_pkg_file(dir, name, uid3, version, vendor)` | `SisPackage::write_pkg_file(&self, dir)` | Uses `pkg_text`. |
| `validate_password(password)` | `SisPackage::validate_password(&self)` | Same 4-char `SYMDEV_SIGN_PASSWORD` rule. |
| `existing_signing_pair(cert, key)` | `SisPackage::existing_signing_pair(&self)` | Reads `self.cert` / `self.key`. |
| `dname()` | `SelfSignedDsa::DNAME` | Recorded makekeys Example Usage DN. |
| `verify_dsa_sha1(signed, blob, cert)` | `SisBlob37::verify_dsa_sha1(&self, signed, cert)` | `self.data` is the signature blob. |
| `wrap_sis(uid3, controller, data)` | `SisUnsigned::wrap(...)` | Private. |
| `unsigned_parts(spec)` | `SisUnsignedSpec::parts(&self)` | Private. |
| `capability_word(caps)` | `SisWord41::from_capabilities(caps)` | Private. |
| `datetime_utc(now)` | `SisDateTime::utc(now)` | |
| `utc(t)` (makekeys) | `SelfSignedDsa::utc(t)` | Private. |
| `required` / `path_arg` in `package.rs` | `SisTools::required` / `SisTools::path_arg` | Private; Wine/EPOCROOT stay on the tool. |
| `required` in `toolchain.rs` | `Toolchain::required` | Private. |

## Deleted

- `crates/symdev-build/src/pkg.rs` (`pub fn render_pkg`)
- crate-root `pub use` of `parse_mmp`, `parse_bld_inf`, `render_pkg`, `dname`, `verify_dsa_sha1`
- `sis/mod.rs` `pub use` of `dname`, `verify_dsa_sha1`

`civil_from_unix_days` moved with `SisDateTime::utc` into `datetime.rs` (still a private calendar algorithm).

## Left as allowed free functions

Private parse/crypto/path helpers with no extra owning type:

- `mmp.rs`: `preprocessor`, `parse_uid_token`, `rest`, `rest_tokens`
- `bld.rs`: `usable_platform`, `known_directive`, `preprocessor`
- `sign.rs`: PEM/DER/3DES/base64 helpers (`parse_signature`, `pem_body`, …)
- `datetime.rs`: `civil_from_unix_days`
- `driver.rs`: `arg`, `resolve_source`, `io`
- `uidcrc.rs`: `epoc_crc16`

Not moved (pre-rules constructors / CLI hash, not the T2/T3 slices):

- `symdev_manifest::{parse, load}` → `Manifest`
- `scaffold::{uid3_for_name, uid3_hex, create_project}`

Test-only helpers (`parse_hex`, `fake_pkg`, goldens) stay free.

## Re-exports

Crate-root `pub use` dropped the deleted free fns; remaining names keep prior order (`rcomp` still last). `sis/mod.rs` stays alphabetical; `sign` is no longer `pub use`d.

## Tests

`cargo test --workspace --offline`: **164 passed**, 0 failed.

- `symdev-build` 120 (`pkg_text_matches_experiment_7_grammar` replaced the deleted `pkg.rs` test)
- `symdev` unittests 3
- `cli` 19
- `symdev-core` 5
- `symdev-manifest` 17
