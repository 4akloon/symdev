# WIP: toolchain manager, Track A (sdk-core) — branch `tm-core`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks A1–A6.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`.
Track B (sigv4, amz_date, http_fetch) runs in parallel in `tm-net`; do not touch those files.

## Status

| Task | State | Commit |
|---|---|---|
| A1 PackageId | done | d9a7cb7 |
| A2 Index / IndexPackage / Host / resolve_url | done | see `git log` |
| A3 SourceSpec / Sources | done | see `git log` |
| A4 TarGz / ReproducibleTarGz | done | see `git log` |
| A5 SdkHome / Receipt / FileFetch | done | see `git log` |
| A6 Gcce / PlatformSdk / Pins | done | see `git log` |

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
- A3: added `SourceSpec::new(name, url, auth) -> Result<SourceSpec>` (not in the plan's
  contract, additive): validates the name (`[a-z0-9-]+`) and URL, adds the trailing `/`,
  refuses `auth = s3` on `file://`. `Sources::parse` uses it and wraps its message in
  `BadSources`; D2's `builtin_source()` should use it too instead of a struct literal.
- A3: plain `http://` is accepted only when the host is exactly `localhost`, `127.0.0.1`
  or `[::1]` (optional numeric port), so `http://localhost.evil.com/` is refused.
- A3: `sources.toml` and its `[[source]]` tables deny unknown keys (`auht = "s3"` would
  otherwise silently disable signing). A user source named like the built-in one is a
  duplicate while the built-in is enabled.
- A4: a lexical check of symlink targets is **not enough**: with `a/b/c/s -> ../../..`
  (the package root, fine) a later `a/b/c/t -> s/..` reads as `a/b/c` but the kernel
  resolves it to the root's parent. `TarGz::extract` therefore (1) writes files and dirs
  first, creating every directory one component at a time and refusing to pass through
  anything that is not a real directory, (2) creates links after that, never through a
  symlinked directory, (3) then follows every symlink component by component like the
  kernel (`TarGz::link_problem`, ≤ 40 hops) and refuses any walk above the root. On error
  the partly filled `into` is left for the caller (SdkHome's staging dir) to remove.
- A4: accepted entry types: regular/continuous, directory, symlink, hard link. Char,
  block, fifo → "is a device or a fifo"; anything else (sparse, PAX global header) →
  "has an unsupported type". Hard link targets must be regular files reached through real
  dirs. Extracted files are 0o755 if any x bit was set, else 0o644 (before umask).
- A4: `ReproducibleTarGz::pack` also accepts `"."` (the whole root, for a GCCE prefix),
  stores the ancestors of a listed dir as dir entries, refuses `..`/absolute includes,
  uses the same `link_problem` on the source tree, GNU headers (long names via the tar
  crate's `././@LongLink`, itself mtime 0), gzip OS byte 255.
- A5: downloads go to `cache/<sha>.tar.gz.part` and are renamed to `<sha>.tar.gz` only
  after size + SHA-256 match; a cached file is reused only if both match, otherwise it is
  deleted and fetched again (Review Focus 1). A failed download removes the `.part`.
- A5: `install` refuses an `ArchiveEntry.sha256` that is not 64 lowercase hex before it
  becomes a cache path (an entry may be built outside `Index::parse`).
- A5: under the lock, `install` removes all of `root/.staging` first (anything there is
  left over: no other installer can be running), then stages in `.staging/<pid>-<n>`.
  A failed extraction removes its staging dir. `installed` errors if a receipt names a
  different id than its directory.
- A5: `uninstall` deletes the receipt first, then the dir, then empty parents
  (`gcce/`), all under the lock. `list` walks at most 8 levels, skipping dot-dirs, and
  stops descending at a receipt.
- A5: `Receipt::write` writes `.symdev-package.toml.partial` then renames; a damaged
  receipt is an error naming the file and "remove <dir> and run `symdev sdk install`".
- A5: lock test checked by mutation: with `file.lock()` commented out the two-thread
  test fails (2 downloads) 3/3; with it, 20/20 runs pass.
- `FileFetch` takes the path after `file://` literally (no percent-decoding); a missing
  file is `SdkError::Fetch { url, detail }`.
- A6: observed layout of `~/gcc-builds/gcc-12.1.0`: `bin/arm-none-symbianelf-{g++,ld,ar}`,
  `lib/gcc/arm-none-symbianelf/12.1.0`, `arm-none-symbianelf/lib` (`ldscripts`,
  `libsupc++.a`) — matches the spec §3 table.
- A6: `Gcce::at` checks all four paths it hands out (gxx, ld, gcc_lib, gcc_target_lib),
  not only gxx and ld, and refuses an id that is not exactly `gcce;<version>`.
  `PlatformSdk::at` refuses an id whose kind is not `sdk`.
- A6: `Pins` builds its ids with the crate-private `PackageId::pinned` (no parse, so no
  `unwrap` in library code); a test checks every pin parses.

## Dead ends

## Next step

Track A done (2026-10-02). Verified: `cargo test --workspace --offline` 580 passed, 0
failed (symdev-sdk: 97); `cargo clippy --workspace --all-targets --offline` no warnings;
`rustfmt --edition 2024 --check` clean on every created file; `cargo fmt --all --check`
fails only on `symbian-rs/crates/symbian-std/src/fs/file.rs` (Track F). Next: merge into
`toolchain-manager` beside Track B (keep both tracks' `mod`/`pub use` lines in `lib.rs`).

## For Track D

- Build `builtin_source()` with `SourceSpec::new("public", BUILTIN_SOURCE, Auth::None)`.
- `SdkHome::install` re-checks `installed` under the lock and returns the existing
  receipt, but D2 should still check `installed` first so no index is fetched.
- `SdkHome` prints nothing; the progress line is D2's. `install` needs `&dyn Fetch`;
  sharing one fetcher across threads needs it `Sync`.
- `TarGz` refuses PAX global headers and sparse entries: archives must be packed by
  `ReproducibleTarGz` (or GNU-format tar without them); `git archive` output is refused.
- `Host` implements `Display` for the "{id} has no archive for {host}" message.
- No `SdkError` variants were added.
