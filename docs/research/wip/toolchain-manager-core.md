# WIP: toolchain manager, Track A (sdk-core) — branch `tm-core`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks A1–A6.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`.
Track B (sigv4, amz_date, http_fetch) runs in parallel in `tm-net`; do not touch those files.

## Status

| Task | State | Commit |
|---|---|---|
| A1 PackageId | done | 5744e89 |
| A2 Index / IndexPackage / Host / resolve_url | todo | |
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

## Dead ends

## Next step

A2: tests for the index types, then implement.
