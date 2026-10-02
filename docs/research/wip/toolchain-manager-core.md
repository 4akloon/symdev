# WIP: toolchain manager, Track A (sdk-core) — branch `tm-core`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks A1–A6.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`.
Track B (sigv4, amz_date, http_fetch) runs in parallel in `tm-net`; do not touch those files.

## Status

| Task | State | Commit |
|---|---|---|
| A1 PackageId | done | d9a7cb7 |
| A2 Index / IndexPackage / Host / resolve_url | done | see `git log` |
| A3 SourceSpec / Sources | todo | |
| A4 TarGz / ReproducibleTarGz | todo | |
| A5 SdkHome / Receipt / FileFetch | todo | |
| A6 Gcce / PlatformSdk / Pins | todo | |

## Facts

- Task 0 is on the branch (0866cab): `SdkError`, `Result`, `Fetch`, `S3Keys`; builds offline.
- Local registry has `tar 0.4.46`, `flate2 1.1.10`, `toml 1.1.6`, `tempfile 3.27.0`, `filetime 0.2.29`.
- `symdev_manifest::Device` has one variant, `NokiaE52`; it derives `Copy`.

## Decisions

- A1: `PackageId::parse` reasons: `is empty`, `needs a kind and a version` (no `;`),
  `has an empty segment`, `has a `.` or `..` segment`, `has a `/`, `\` or NUL in a segment`,
  `has whitespace or a control character`. Serde goes through `parse`, so an index or
  receipt with a bad id fails to load.

- A2: `Index::parse` checks more than the plan's list, all with `BadIndex`: duplicate id,
  a package with no archive, two archives for one host, and every archive URL and
  `source-code` under the index directory (same rule as `resolve_url`, shared through the
  crate-private `url::relative_url_problem`). `schema` below 1 is `BadIndex`, above 1
  `UnknownSchema`; the schema is read first so a newer index is never reported as malformed.
- A2: `Index::insert` also refuses a package `parse` would refuse (so `publish` never
  uploads an index symdev cannot read back). `to_toml` omits an empty `package` list and a
  `None` `source-code`; `depends = []` is written.
- A2: `resolve_url` refuses: empty, leading `/`, `://`, `\`, `?`, `#`, whitespace/control,
  empty/`.`/`..` segments; `a..b` (dots inside a name) is allowed. It is a free function
  because the plan's contract names it so (`pub fn resolve_url`).
- `Host` also implements `Display` (= `as_str`), for D2's "no archive for {host}" message.

## Dead ends

## Next step

A3: tests for `SourceSpec` / `Sources`, then implement.
