# WIP: toolchain manager + hosted artifacts (brainstorm, 2026-10-01)

Idea (owner): an SDK/toolchain manager plus hosting of firmware, emulator and SDK, so the
whole toolchain deploys with one command from a clean environment.

Stage: brainstorming, architectural path. No design approved yet.

## Facts gathered

- symdev reads the toolchain only from env: `SYMDEV_EPOCROOT`, `SYMDEV_GXX`, `SYMDEV_LD`,
  `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` (`crates/symdev-build/src/toolchain.rs`),
  `SYMDEV_EKA2L1`, `SYMDEV_SIGN_PASSWORD`; `SYMDEV_AR` derived beside `ld`.
- From the SDK only files are used: `epoc32/include` (+ `gcce/gcce.h`, `variant/*.hrh`),
  `epoc32/release/armv5/{lib,urel}` (`.dso`, `.lib`). No SDK program is spawned.
- Sizes on this host: GCC 12.1.0 build 203M, binutils 2.29.1 23M, whole SDK 466M,
  EKA2L1 Z: drive (from firmware) 208M.
- EKA2L1 here is a host build (`~/src/EKA2L1-build`) linked against a private sysroot and
  Qt 6.8.3 under `~/.local` — not portable as is. symdev needs our unmerged fixes
  (`--install` with `--run` is only in upstream PR #726).
- Current policy (`CLAUDE.md`, `docs/research/licensing.md`, `docker/`): never commit,
  COPY, curl or bundle SDK / ROM / firmware / keys; they are user-supplied paths.

## Licensing by component (to confirm before any hosting decision)

| Component | License | Redistributable? |
|---|---|---|
| symdev | MIT | yes |
| GCCE (fedor4ever GCC + binutils) | GPL-3 | yes, with corresponding source |
| EKA2L1 + our fixes | GPL-3 | yes, with corresponding source |
| S60 3rd FP2 SDK | Nokia EULA | not established — needs the EULA text read |
| E52 firmware / ROM | Nokia proprietary | no grant known |

- Repo `4akloon/symdev` is PUBLIC; CI is GitHub Actions on `ubuntu-24.04`, Rust gate only
  (fmt, clippy, test), no SDK/ROM/EKA2L1. Private files for CI therefore mean a private
  store + an Actions secret; fork and dependabot PRs get no secrets (no full run there).
  A self-hosted runner on the owner's machine is unsafe for a public repo.
- CI on `main` is red since d9e6389: `cargo fmt --check` on
  `symbian-rs/crates/symbian-std/src/fs/file.rs` (spun off as a separate task).

## Decisions

1. Audience: owner + CI first, public later (owner, 2026-10-01). Proprietary blobs may sit
   in private storage for phase 1; the public phase must not depend on hosting them.

2. CI scope: phase 1 = build + package the examples (GCCE + SDK only); emulator tests in CI
   are phase 2 (owner, 2026-10-01).
3. Emulator direction (owner, 2026-10-01): upstream EKA2L1 moves slowly, so we will likely
   maintain our own fork (`4akloon/EKA2L1`, GPL-3, separate repo, separate process) and
   develop what symdev needs there. The manager must take the emulator from our fork's
   builds, not upstream releases. Fork strategy itself is a separate sub-project.

## More facts

- Host: Ubuntu 26.04.1, glibc 2.43. `~/gcc-builds` GCC 12.1.0 is dynamically linked
  (libc, libm only) and needs GLIBC_2.38 → runs on the CI's 24.04 (2.39), not on older
  distros. A hosted GCCE should be built on an older base for portability.

4. Storage: object storage (R2 / S3 / B2 class), not GitHub Releases (owner, 2026-10-01).
5. Model: work like the Android SDK (sdkmanager / repository manifests / avdmanager);
   take its best approaches (owner, 2026-10-01).

## Android SDK → symdev mapping (draft, not agreed)

| Android | symdev candidate |
|---|---|
| `ANDROID_HOME`, one root, layout = package path | `SYMDEV_HOME` (`~/.local/share/symdev`), `gcce/12.1.0/`, `sdk/s60-3rd-fp2/` … |
| package paths `build-tools;34.0.0` | `gcce;12.1.0`, `sdk;s60-3rd-fp2`, `emulator;<ver>`, `firmware;rm-469;<ver>` |
| repository XML (+ add-on sites) | repository manifest; several sources (public + private with auth) |
| per-host archives + checksum + size | per-host archive (linux-x86_64 now), SHA-256, size |
| `package.xml` in each installed dir | install receipt per package (version, sha, source) |
| `licenses/` accepted-hash files, CI copies them | license acceptance per package; for Nokia blobs it cannot grant rights we do not hold |
| dependencies between packages | e.g. emulator needs firmware, build needs gcce + sdk |
| channels stable/beta/canary | probably YAGNI in phase 1 |
| side-by-side versions, project pins them | several versions installed; project or repo pins one |
| AGP auto-installs missing packages | `symdev build` installs what is missing (licences accepted) |
| `avdmanager` + system images | `symdev device create` = EKA2L1 device from firmware (phase 2) |
| cmdline-tools bootstrap zip | symdev binary itself is the bootstrap |

6. Phase 1 = one root, package ids, side-by-side versions, repository manifest with several
   sources (public + private with auth), per-host archives + SHA-256, install receipts,
   simple dependencies, `symdev sdk list/install/uninstall`, env overrides, **and
   auto-install from `symdev build`**. Phase 2 = licence acceptance, devices
   (`symdev device create`), emulator packages. Channels: YAGNI (owner, 2026-10-01).

## Facts for auto-install

- `symdev.toml` already has `[target] device = "nokia-e52"` — the analogue of Android's
  `compileSdk`: the device names the platform SDK (and later the firmware) a build needs.
- Memory rule "manifest is config only" is about UI vs config; build config already lives
  there (`[language]`, `[signing]`). A GCCE version pin would be build config — decide in
  design (Android: AGP pins a default build-tools / NDK version per AGP release).

7. Storage: Cloudflare R2, owner's account; client speaks plain S3 API (owner, 2026-10-01).
8. Package recipes / publishing live in a separate repository (owner, 2026-10-01).

## SDK subset size

The build reads `epoc32/include` (24M) + `epoc32/release/armv5/lib` (87M, `.dso` + `.lib`)
+ `epoc32/release/armv5/urel` (0.4M) ≈ 112M of the 466M SDK, before compression.

## Publishing proposal (sent to owner 2026-10-01, not agreed)

- Two buckets: `symdev-public` (GPL builds + their sources, index) behind a custom domain;
  `symdev-private` (SDK, firmware, own index), no public access.
- Blobs immutable, content-addressed (`<id>/<version>/<sha256>.tar.zst`); only the index
  is mutable. Never delete a version a released symdev pins.
- Built packages (gcce, later emulator): PR changes a recipe → CI builds (no upload) →
  merge to main → CI builds, uploads blob + GPL sources, regenerates and uploads index.
  Merge = release. Publisher token only in a protected Actions environment.
- Proprietary packages (sdk, firmware): owner runs a publish command locally — trim, pack,
  hash, upload to the private bucket, update the private index; the recipe (file list,
  expected hash) is committed to the repo, so git records what is in the private bucket.
- Admin once: Cloudflare account + R2 (likely needs a payment method even on free tier),
  2 buckets, custom domain, 2 API tokens (publisher r/w; reader read-only on private),
  GitHub secrets (publisher in the packages repo, reader in symdev CI), branch protection.
- Index signing (ed25519, key offline, public key in symdev): proposed for later.

9. Public bucket on `*.r2.dev` for now; publishing proposal above approved as is
   (owner, 2026-10-01). Keep r2.dev access enabled after a custom domain is added, so
   URLs baked into old symdev releases keep working.

## Facts for the GCCE recipe

- `~/gcc-builds` build steps were never recorded (docs say Unknown), but the binary
  reports them: `arm-none-symbianelf-g++ -v` → GCC 12.1.0, `--target=arm-none-symbianelf
  --without-headers --enable-languages=c,c++,lto --enable-lto --enable-interwork
  --enable-long-long --enable-tls --enable-multilib --enable-wchar_t --enable-c99
  --with-newlib --with-dwarf2 --with-static-standard-libraries --disable-hosted-libstdcxx
  --disable-libstdcxx-pch --disable-shared --disable-option-checking --disable-threads
  --disable-nls --disable-win32-registry --disable-libssp --disable-libquadmath`;
  ld is GNU Binutils 2.29.1. Sources on host: `~/src/GCC4Symbian`,
  `~/src/binutils-2.29.1-build`.
- Workspace already has `flate2`, `sha2`, `hmac`, `toml`; no HTTP client yet.

10. Client approach 1: new workspace crate `symdev-sdk` (PackageId, Index, Source, SdkHome),
    `symdev sdk` subcommands, `Toolchain` env-first then SdkHome, in-process auto-install;
    the packages repo depends on the same crate (git tag) for `publish` (owner, 2026-10-01).
    Rejected: separate sdkmanager binary; conda/pixi/OCI.

## Design sections (presented one by one)

1. Packages, archives, index, sources — **approved 2026-10-02** ("ніби ок").
   Full version in every id, immutable (Android NDK moved from in-place `ndk-bundle` to
   side-by-side `ndk;<ver>` for this reason); ids `gcce;12.1.0` (gcc+binutils, one
   package), `sdk;s60-3rd-fp2;1.1`; `.tar.gz` via existing flate2; archive root = package
   root, installed at `$SYMDEV_HOME/<id with ; → />` + receipt `.symdev-package.toml`;
   per-source `index.toml` (schema = 1; id, license, source-code, depends, archive{host,
   url relative to index, sha256, size}); `any` host for SDK/firmware; exact-id deps.
   Built-in public source; extra sources in `~/.config/symdev/sources.toml`, `auth = "s3"`
   with keys only from env `SYMDEV_SOURCE_<NAME>_ACCESS_KEY_ID` / `_SECRET_ACCESS_KEY`;
   first source wins; SHA-256 always verified before extract.
2. SdkHome + Toolchain — **approved 2026-10-02**, option A (no project GCCE override in
   phase 1; symdev release pins the default; env covers experiments).
   XDG: packages `$SYMDEV_HOME` (default `~/.local/share/symdev`), sources
   `~/.config/symdev/sources.toml`, download cache `~/.cache/symdev/downloads/<sha>.tar.gz`.
   Install: download → verify size+sha → extract to `.staging` → atomic rename → receipt
   last (no receipt = broken, reinstall); `$SYMDEV_HOME/.lock` via std `File::lock`.
   Toolchain: each `SYMDEV_*` var wins when set, else the field comes from packages
   (`Gcce::gxx()/ld()/gcc_lib()/gcc_target_lib()`, `PlatformSdk::epocroot()`); gcc and
   binutils share one prefix. Device → SDK mapping lives in `symdev-sdk`, not on `Device`.
   Code facts: `Device` enum in `crates/symdev-manifest/src/schema.rs:38`;
   `Toolchain::from_env` at `crates/symdev-cli/src/build_cmd.rs:21`,
   `Epocroot::from_env` at `crates/symdev-cli/src/main.rs:125`.
3. Auto-install — **approved 2026-10-02**. Any command needing the toolchain (build,
   bld.inf read); packaging installs nothing. Skip a package whose fields are all set by
   env. No network when everything is installed (index fetched only when something is
   missing). One stderr line per install. `--offline` → error with the install command.
   Errors: private-only package without keys names the env vars AND `SYMDEV_EPOCROOT`;
   sha mismatch deletes the file, names expected/actual/URL, no retry; not found names
   id + sources searched. No `update` (immutable ids); old versions stay until
   `symdev sdk uninstall`. Network: `ureq` + rustls, timeouts, env proxy; own SigV4 GET
   signer on `hmac`/`sha2`, tested against AWS's published vectors; region `auto`.
   Licence gate on auto-install arrives in phase 2 (`license` field already in index).
4. Packages repo + CI — **approved 2026-10-02**. Public repo `4akloon/symdev-packages`:
   `recipes/<kind>/<name>/<ver>/{recipe.toml,build.sh}`, `publish/` Rust bin (symdev-sdk
   via git tag), workflows `build.yml` (PR, no upload) and `publish.yml` (main → R2 →
   index, `concurrency: publish`). GCCE: official gcc-12.1.0 + binutils-2.29.1 tarballs,
   pinned sha, configure flags from `g++ -v`; GCC4Symbian patches — check first, do not
   assume; built in Debian 11 container (glibc 2.31); acceptance = examples' `.exe`
   byte-identical to `~/gcc-builds` output from a relocated package; source archive
   alongside. SDK: owner runs `publish private` locally; reproducible tar (sorted,
   normalized mtime/uid/mode) so recipe pins the sha. Publisher refuses existing ids;
   bucket index is the truth for hashes of built packages; no bot commits.
   symdev CI job `examples`: push to main + same-repo PRs (explicit skip step for forks/
   dependabot), caches `~/.local/share/symdev` keyed on the pins file, builds + packages
   hello, gui, Rust examples via auto-install; dummy `SYMDEV_SIGN_PASSWORD` in workflow.
   Secrets: publisher key in `symdev-packages` env `publish` (main only); reader key
   (read-only private) in `symdev`.
5. Errors + testing + acceptance — **approved 2026-10-02**.

## Spec

Written on branch `toolchain-manager` (worktree `~/worktrees/symdev/toolchain-manager`):
`docs/superpowers/specs/2026-10-02-toolchain-manager-design.md`. It also proposes new
wording for the "never download SDK/ROM" rule in `CLAUDE.md` and `licensing.md` (§1) —
the owner must approve that explicitly; neither file is edited yet.

Owner approved the spec as written, including the §1 rule wording ("Все ок, можеш
продовжувати", 2026-10-02); `CLAUDE.md` and `licensing.md` updated on this branch.
Owner then asked: "Роби все і паралельно" — do everything, in parallel.

## Next step

Write the implementation plan (superpowers:writing-plans), then dispatch independent
tracks in parallel.
