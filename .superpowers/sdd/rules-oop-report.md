# rules-oop — types own behavior, production-quality leftovers

Date: 2026-09-18
Branch: `rules-oop`
Worktree: `/home/genius/worktrees/symdev/rules-oop`
Base: `origin/main` (`d04e7c5`)

## API moves (old free fn → type method)

| Old | New | Notes |
|-----|-----|--------|
| `encode_unsigned_sis(spec)` | `SisUnsigned::encode(&spec)` | Canonical unsigned SIS bytes. Crate-root `pub use` of the free fn removed (no external callers). |
| `encode_signed_sisx(spec, key, cert, password)` | `SisUnsigned::encode_signed(&spec, key, cert, password)` | Same; internal `*_ref` wrappers deleted. |
| `signatures39_from_key_and_cert(controller, …)` | `SisController::signatures_from_key_and_cert(…)` | Crypto stays in `sis/sign.rs` as an impl on the owning type. Crate-root `pub use` of the free fn removed. |

Left as allowed free functions (constructor / algorithm / existing helpers, not this refactor):

- `parse_mmp` / `parse_bld_inf` (constructors)
- `verify_dsa_sha1` (algorithm, no owner)
- `dname`, `validate_password`, `write_pkg_file`, `existing_signing_pair` (used by `SisPackage` / `SisTools`; not thin encode wrappers)

## Production leftovers cleaned

- Deleted unused `SisTools::signsis_args` and its experiment-8 argv test. Wine `signsis` is not spawned; `SisTools.signsis` path remains for `from_env` pinning. `makesis_args` kept (recorded tokens, still tested).
- Replaced hand-rolled SHA-1 in `package.rs` with `sha1::Sha1::digest` (crate already in `Cargo.toml`).
- `SisCompressed::zlib` returns `Result` instead of `expect`.
- `parse_mmp` / `parse_bld_inf` no longer `unwrap` on tokens.

## Leftover unwrap / expect / panic

**None in non-test library code** after this change.

Tests still unwrap (allowed): hex fixtures, tempfile, `SisUnsigned::encode` assertions, CLI/scaffold/manifest/core unit tests.

## Re-exports

Crate-root `lib.rs` `pub use` kept original order; only `encode_unsigned_sis`, `encode_signed_sisx`, and `signatures39_from_key_and_cert` were dropped. `sis/mod.rs` module `pub use` order unchanged.

## Tests

`cargo test --workspace --offline`: 157 passed (0 failed).

- `symdev-build` 113 (was 114; dropped `signsis_args_match_experiment_8`)
- `symdev` unittests 3
- `cli` 19
- `symdev-core` 5
- `symdev-manifest` 17
