# Toolchain manager and hosted packages: design (2026-10-02)

Status: **design approved section by section** by the repository owner on 2026-10-01/02;
this written spec awaits the owner's review. Brainstorm record:
[toolchain-manager.md](../../research/wip/toolchain-manager.md).

## 0. Goal

Today a first `hello` needs a hand-built GCCE, a hand-built EKA2L1, an S60 SDK found
somewhere, and seven `SYMDEV_*` variables. The goal is a toolchain that deploys with one
command from a clean environment, modelled on the Android SDK (`sdkmanager`, repository
manifests, side-by-side versions, Gradle installing what a build is missing).

**Phase 1 (this spec): the owner and CI.** The owner's machines and the `symdev` GitHub
Actions get GCCE and the S60 SDK from an object store, and `symdev build` installs them by
itself; a prebuilt `symdev` and the Rust SDK are packages too, installed by `install.sh`
(added by the owner on 2026-10-02, §12). **Phase 2 (separate specs):** public users,
licence acceptance, emulator and firmware packages, `symdev device create`. The EKA2L1
fork the owner intends to maintain (upstream moves slowly) is its own sub-project; the
manager only has to take the emulator from that fork's builds when phase 2 adds it.

## 1. Licensing and the repository rule this changes

| Component | Licence | Where it lives |
|---|---|---|
| symdev | MIT | this repository |
| GCCE (GCC 12.1.0 + binutils 2.29.1) | GPL-3 | public bucket, **with its corresponding source** |
| S60 3rd FP2 SDK | Nokia EULA, redistribution not established | **private** bucket, owner's own copy |
| E52 firmware (phase 2) | Nokia, no redistribution grant known | private bucket, owner's own copy |

A public mirror of the SDK or firmware is not part of any phase: the public phase must work
without it (users point at their own SDK file or `SYMDEV_EPOCROOT`).

`CLAUDE.md` ("Never commit or download SDK, ROM …") and
[licensing.md](../../research/licensing.md) ("never downloaded by this repository") forbid
what phase 1 does with the private bucket. **Proposed replacement** (owner to approve with
this spec):

> Never commit SDK, ROM/firmware, `.sis`, `.sisx`, `.cer` or `.key` files, and never link to
> a third-party copy of them. symdev may download the SDK and firmware only from a source
> the operator configured with their own credentials (the owner's private bucket); the
> built-in public source carries only GPL/MIT packages and their sources.

## 2. Packages, archives, index, sources

**Package ids** follow Android's `kind;…` paths, with the full version always in the id.
A published id never changes; a rebuild is a new version. (Android's NDK moved from the
in-place `ndk-bundle` to side-by-side `ndk;<version>` for exactly this reason.)

| Id | Contents | Host |
|---|---|---|
| `gcce;12.1.0` | GCC 12.1.0 and binutils 2.29.1 in one prefix | `x86_64-linux` |
| `sdk;s60-3rd-fp2;1.1` | headers, `.dso` import stubs, three static libraries, `variant.cfg` | `any` |
| `symdev;0.1.0` | `bin/symdev`, statically linked (§12) | `x86_64-linux` |
| `rust-sdk;0.1.0` | the Rust SDK in the repository's layout: `Cargo.toml`, `crates/symdev-locale`, `symbian-rs` (§12) | `any` |
| `emulator;…`, `firmware;rm-469;…` | phase 2 | — |

The SDK package is only what a GCCE build reads, measured on 2026-10-02: `epoc32/include`
(2 123 files, 24 MB), the 570 `.dso` import stubs in `epoc32/release/armv5/lib` (6.3 MB), the
static libraries the link line names (`usrt2_2.lib`, `eexe.lib` and `edll.lib`, all in
`epoc32/release/armv5/urel`) and `epoc32/tools/variant/variant.cfg`, which names the variant header for `bld.inf`
preprocessing. Left out: the 570 RVCT `.lib` import libraries beside the `.dso` files
(77.5 MB; GCCE links the `.dso`), and everything else in the 466 MB SDK. Compressed, the
include tree is 3.8 MB and the `.dso` files 1.5 MB; the whole earlier directory-level subset
was 9.9 MB.

**Ids are validated**: segments are non-empty, are not `.` or `..`, and contain no `/`, `\`
or NUL. The id maps
to an install path by replacing `;` with `/`: `gcce;12.1.0` → `gcce/12.1.0`.

**Archives** are `.tar.gz` (`flate2` is already a workspace dependency; the `tar` crate is
new). The archive root is the package root. Archives are stored content-addressed,
`<kind>/<…>/<sha256>.tar.gz`, and never overwritten.

**Index**: one `index.toml` per source.

```toml
schema = 1

[[package]]
id = "gcce;12.1.0"
license = "GPL-3.0-or-later"
source-code = "src/gcce/12.1.0/3f9a….tar.gz"   # GPL corresponding source
depends = []                                     # exact ids, no ranges

[[package.archive]]
host = "x86_64-linux"                            # or "any"
url = "gcce/12.1.0/8c41….tar.gz"                 # relative to index.toml
sha256 = "8c41…"
size = 61234567
```

URLs are relative to the index, so moving a bucket to another domain changes no byte of the
index. An unknown `schema` is an error that tells the user to update symdev. The index is
uploaded with `Cache-Control: no-cache`; archives with `immutable`.

**Sources.** symdev has one built-in source: the public bucket's `index.toml` on its
`*.r2.dev` URL (no custom domain yet; the r2.dev URL stays enabled after one is added, so
old releases keep working). Further sources are listed in
`$XDG_CONFIG_HOME/symdev/sources.toml`:

```toml
builtin = true          # false disables the built-in source (tests set this)

[[source]]
name = "private"
url = "https://<account-id>.r2.cloudflarestorage.com/symdev-private/"
auth = "s3"
```

Keys come **only from the environment**:
`SYMDEV_SOURCE_<NAME>_ACCESS_KEY_ID` / `SYMDEV_SOURCE_<NAME>_SECRET_ACCESS_KEY`
(`<NAME>` upper-cased). `url` may also be `file:///…` (a directory holding `index.toml` and
archives), which is what tests use. When an id is in several sources the first source in
order wins (built-in first). SHA-256 and size are always checked before extracting.

## 3. Where packages live, and how `Toolchain` finds them

| What | Path |
|---|---|
| Installed packages | `$SYMDEV_HOME`, default `$XDG_DATA_HOME/symdev` (`~/.local/share/symdev`) |
| Sources | `$XDG_CONFIG_HOME/symdev/sources.toml` |
| Download cache | `$XDG_CACHE_HOME/symdev/downloads/<sha256>.tar.gz` |

**Install procedure** — no half-installed state is ever visible:

1. take `$SYMDEV_HOME/.lock` (`std::fs::File::lock`), so parallel builds do not race;
2. download into the cache (skip if the cached file already has the right size and hash);
3. verify size and SHA-256; on mismatch delete the file and fail (§5);
4. extract into `$SYMDEV_HOME/.staging/<random>`, refusing absolute paths, `..`, links
   that point outside the package, and a `.symdev-package.toml` or
   `.symdev-package.toml.partial` at the archive's root (an archive cannot bring its own
   receipt);
5. write the receipt `.symdev-package.toml` (id, sha256, source name, archive URL) into the
   staging directory;
6. rename the staging directory to `$SYMDEV_HOME/<id path>` **last**, so the package
   appears with its receipt in one step (review M3, 2026-10-02; the receipt used to be
   written after the rename).

A package directory without a receipt is unfinished and is replaced on the next install.
A package never sits inside another or above one: `install` refuses such an id (naming
both), and `uninstall` removes a directory only if it holds a receipt, or if it is an
unfinished package that neither lies inside a package nor holds one — so a partial id
(`sdk;s60-3rd-fp2`) or an id inside a package (`gcce;12.1.0;bin`) removes nothing.

**Layouts are types.** `Gcce` knows its files, `PlatformSdk` its root:

| `Toolchain` field | Variable that overrides it | From the package |
|---|---|---|
| `gxx` | `SYMDEV_GXX` | `gcce`: `bin/arm-none-symbianelf-g++` |
| `ld` (and `ar` beside it) | `SYMDEV_LD` (`SYMDEV_AR`) | `gcce`: `bin/arm-none-symbianelf-ld` |
| `gcc_lib` | `SYMDEV_GCC_LIB` | `gcce`: `lib/gcc/arm-none-symbianelf/12.1.0` |
| `gcc_target_lib` | `SYMDEV_GCC_TARGET_LIB` | `gcce`: `arm-none-symbianelf/lib` |
| `epocroot` | `SYMDEV_EPOCROOT` | `sdk`: the package root (holds `epoc32/`) |
| `elf2e32` | `SYMDEV_ELF2E32` | none: native unless set (unchanged) |

**Every variable that is set wins**, field by field, so every existing environment keeps
working unchanged; the packages fill the rest. Reading the environment stays in the CLI
adapter, as today; `Toolchain` and the layout types receive values.

**Which versions.** The platform SDK follows `[target] device` in `symdev.toml`
(`nokia-e52` → `sdk;s60-3rd-fp2;1.1`); that table lives in `symdev-sdk`, so the manifest's
`Device` knows nothing of packages. The GCCE version is pinned by the symdev release
(`gcce;12.1.0`), as AGP pins its default NDK. Both live in one file, `pins.rs`. Phase 1 has
**no per-project override**: symdev and the examples move together, and `SYMDEV_*` covers
experiments with another compiler; a `[toolchain]` pin in `symdev.toml` is a public-phase
feature.

## 4. Auto-install

Every command that needs the toolchain — `symdev build` (C++ and Rust: the Rust SDK's C++
shims use GCCE too) and anything that reads `bld.inf` (it needs `epocroot`) — does this:

1. work out the required ids from the device and the pins;
2. drop an id whose every field is already set by the environment;
3. if all remaining ids are installed, **touch no network** — a build works offline after
   the first install; indexes are fetched only when something is missing;
4. install each missing id (§3), printing one line to stderr:
   `installing gcce;12.1.0 (58 MB) from public…`;
5. carry on with the build.

A global `--offline` flag (as in cargo) forbids the network: a missing package is then an
error carrying the exact install command. `symdev package` needs no toolchain and installs
nothing.

**Commands:**

| Command | Does |
|---|---|
| `symdev sdk list` | installed packages (from receipts) and available ones (from indexes) |
| `symdev sdk install [<id>…]` | installs the ids; with none, what the current project needs |
| `symdev sdk uninstall <id>…` | removes installed packages |

There is no `update`: ids are immutable. When a symdev release pins `gcce;14.2.0`, the next
build installs it beside `gcce;12.1.0`, which stays until uninstalled.

**Network.** `ureq` with rustls, timeouts, proxy from the environment. Sources with
`auth = "s3"` sign requests with AWS Signature V4 (region `auto` for R2). The signer is our
own, GET and PUT only (PUT is for `publish`, §6), built on the workspace's `hmac` and
`sha2`, and tested against AWS's published SigV4 examples. No S3 client crate.

## 5. Errors

Every error names what failed and the fix (`CLAUDE.md`):

| Situation | Message carries |
|---|---|
| package only in a private source, keys not set | the source, both variable names, **and** "or set `SYMDEV_EPOCROOT` to your own SDK" |
| SHA-256 or size mismatch | id, expected and actual hash, URL; file deleted; no automatic retry |
| id in no source | id and the sources searched; for an `sdk` id also "set `SYMDEV_EPOCROOT` to your own SDK, or add a source that has it in `<sources.toml>`" |
| `--offline` and not installed | id and `symdev sdk install <id>` |
| a `SYMDEV_*` path does not exist | the variable and the path |
| unknown index `schema` | source, schema number, "update symdev" |
| archive entry escapes the package | archive URL and the entry |
| HTTP 403 from an `s3` source | the source and "check the key's bucket permissions" |

## 6. The packages repository

Public repository `4akloon/symdev-packages`. It holds recipes and the publisher, never a
proprietary file.

```
recipes/
  gcce/12.1.0/recipe.toml          # source URLs + sha256, patches, configure flags
  gcce/12.1.0/build.sh
  sdk/s60-3rd-fp2/1.1/recipe.toml  # directories taken, expected archive sha256
publish/                           # Rust binary; symdev-sdk as a git dependency on a tag
.github/workflows/build.yml        # PR: build changed recipes, upload nothing
.github/workflows/publish.yml      # main: build, upload, update the index
```

**GCCE recipe.**

- Official `gcc-12.1.0` and `binutils-2.29.1` tarballs with pinned SHA-256; both configured
  with one prefix. GCC's flags are the ones the current build reports (`g++ -v`):
  `--target=arm-none-symbianelf --without-headers --enable-languages=c,c++,lto --enable-lto
  --enable-interwork --enable-long-long --enable-tls --enable-multilib --enable-wchar_t
  --enable-c99 --with-newlib --with-dwarf2 --with-static-standard-libraries
  --disable-hosted-libstdcxx --disable-libstdcxx-pch --disable-shared
  --disable-option-checking --disable-threads --disable-nls --disable-win32-registry
  --disable-libssp --disable-libquadmath`.
- Verified in experiment 107: `~/gcc-builds` also used GCC4Symbian's libgcov fix and two
  sys-include headers (each needed) and in-tree gmp/mpfr/mpc/isl, so the recipe pins the
  prerequisites too; binutils' flags are GCC4Symbian's binutils step. Examples built with
  the relocated result are byte-identical. Experiment 108 replaced the three GCC4Symbian
  files (unclear licence) with our own, written clean-room — two sys-include headers and a
  `-D__INTPTR_TYPE__=int` in `CFLAGS_FOR_TARGET` — with the same target libraries,
  `c++config.h` and examples; the recipe pins only official GNU tarballs.
- Built in a Debian 11 container (glibc 2.31), so it runs on Ubuntu 20.04+ / Debian 11+.
  The current host build needs glibc 2.38 and would not.
- The GPL source archive (tarballs and the recipe directory: `recipe.toml`, `build.sh`, our
  headers) is published beside the package.
- Acceptance: §8, item 1.

**SDK recipe.** The owner runs the publisher locally with the publisher key:

```bash
cargo run -p publish -- private 'sdk;s60-3rd-fp2;1.1' --from ~/sdk/S60_3rd_FP2
```

The archive is reproducible (entries sorted; mtime, uid/gid and modes normalised), so the
same SDK always gives the same SHA-256 and the recipe pins it; `publish` refuses a result
that differs from the recipe.

**Publishing rules.**

- `publish` refuses an id that is already in the index: no overwrite.
- It reads the index, adds the entry, uploads archive (and source archive) first, index
  last. `concurrency: publish` in Actions serialises public publishes.
- For built packages the bucket index is the record of hashes (a GCC build is not
  reproducible, so git cannot know the hash beforehand); git records the recipes. No bot
  commits back into the repository.
- Merge to `main` is the release of a built package.

## 7. CI of symdev

A new job `examples` in `.github/workflows/ci.yml`, beside the existing Rust gate:

- runs on push to `main` and on PRs from this repository; a first step skips the job when
  the reader key is absent (forks, dependabot), instead of failing;
- caches only `$SYMDEV_HOME/gcce` (GPL, public anyway) with the hash of `pins.rs` as the key,
  so an unchanged pin downloads no compiler. The SDK (about 5 MB) downloads on every run and
  is never cached: pull-request runs — a fork's too, running the fork's own workflow — can
  restore the default branch's caches, so a cached SDK would be a copy anyone could take
  (review C1, 2026-10-02);
- builds symdev, then installs the packages with `symdev sdk install` in each example
  (what the project needs, as a clean machine's first build would), then runs `symdev
  build` and `symdev package` for `examples/hello`, `examples/gui` and the Rust examples.
  The reader key is set **only** on the install step, which runs nothing but symdev:
  `cargo build` runs third-party build scripts, and the build and package steps find
  everything installed and read no source;
- sets `RUSTFLAGS` (`-D warnings`) only on the Rust gate: a set `RUSTFLAGS`, even an empty
  one, replaces the `build.rustflags` that `symdev build` passes to the Rust SDK's libcall
  build;
- sets `SYMDEV_SIGN_PASSWORD` to a dummy value in the workflow (the key is throwaway and
  generated per run).

| Repository | Secret | Permission |
|---|---|---|
| `symdev-packages`, environment `publish` (main only) | publisher key | read/write, both buckets |
| `symdev` | reader key | read-only, `symdev-private` |

## 8. Testing and acceptance

**`cargo test` never touches the network or R2.** Tests point `SYMDEV_HOME` and `XDG_*` at
temporary directories, set `builtin = false`, and use `file://` sources. The current
"missing toolchain" tests become: `--offline` with an empty `SYMDEV_HOME` fails with the
install command.

Unit tests in `symdev-sdk`: id parsing and path mapping (rejecting `..`, `/`, empty
segments); index parsing, schema check, relative URL resolution; extraction refusing
absolute paths, `..` and escaping links (archives built in the test); an unfinished
`.staging` / receipt-less install is redone, a receipted one skipped; two threads installing
one id end with one good install; hash mismatch deletes the file; SigV4 against AWS's
published examples; `Toolchain` field-by-field override; an HTTP install served by a
`TcpListener` inside the test. CLI tests: `symdev sdk list/install/uninstall` and
`symdev build --offline` against a `file://` source.

**"One command" in phase 1.** Prerequisites: `curl`, the reader key in the environment
(`SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID`, `SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY`) **and** the
private source listed in `~/.config/symdev/sources.toml` — the built-in source never carries
the SDK, so without this entry the key is never used:

```toml
[[source]]
name = "private"
url = "https://<account-id>.r2.cloudflarestorage.com/symdev-private/"
auth = "s3"
```

Then `curl -fsSL <public bucket>/install.sh | sh`, and the first `symdev build` of any
project installs the rest (`symdev sdk install` installs the same set explicitly, without
building). A C++ project needs no Rust at all; a Rust project needs rustup (its nightly comes
from `rust-toolchain.toml`). Building symdev from source with
`cargo install --git https://github.com/4akloon/symdev symdev-cli` keeps working. An `sdk;…`
id that no source has is an error that says so: "set SYMDEV_EPOCROOT to your own SDK, or add
a source that has it in <path of sources.toml>".

**Phase 1 is done when all four hold, each checked by running it, not by reading code:**

1. The examples built with `gcce;12.1.0` extracted into a different directory match those
   built with `~/gcc-builds`: the linked `.elf` byte for byte, and the `.exe` byte for byte
   except the E32 header's time stamp (0x24–0x2B) and header CRC (0x14–0x17), which the
   post-linker writes from the clock, so two builds with the same compiler differ there
   too (found 2026-10-02, track D). Objects may differ where the assembler differs:
   `~/gcc-builds/gcc-12.1.0` assembles with its own binutils 2.35, while the package uses
   binutils 2.29.1 throughout; the report says which `as` each side used.
2. In a clean `ubuntu:24.04` container with only rustup, git, the reader key and the
   `sources.toml` above, the commands above produce `hello.sisx` and `gui.sisx`, and both install and launch in
   EKA2L1 on the host (window checked, not the log).
3. The `examples` job is green on `main`.
4. The owner's current `SYMDEV_*` environment builds everything exactly as before.
5. On a clean Linux with no Rust installed, `install.sh` followed by `symdev build` and
   `symdev package` in a copy of `examples/hello` produces `hello.sisx`; with rustup added,
   the same works for a project made by `symdev new hello --lang rust`, through the
   `rust-sdk` package. (The in-tree `symbian-rs/examples` inherit their workspace and nightly
   from `symbian-rs` and build only inside the clone.)

## 9. Code placement

New crate `crates/symdev-sdk` (every file ≤ 300 lines, one type per file):
`PackageId`, `Index`, `Source` (value) and its fetching adapter (HTTP, `file://`, S3
signing — the only place that does I/O to sources), `SigV4`, `SdkHome`, `Receipt`, the
extractor, the layouts `Gcce` and `PlatformSdk`, and `pins.rs`. `symdev-build`'s
`Toolchain` gains a constructor from override values plus installed layouts;
`Toolchain::from_env` and `Epocroot::from_env` callers
(`crates/symdev-cli/src/build_cmd.rs:21`, `crates/symdev-cli/src/main.rs:125`) move to it.
`symdev-cli` gets the `sdk` subcommand and `--offline`. New dependencies: `ureq` (rustls),
`tar`.

## 10. What the owner does (accounts are theirs)

Each step is placed in the implementation plan right before the step that needs it:

1. Cloudflare account with R2 enabled (it may ask for a payment method even on the free
   tier; 10 GB free is ample).
2. Buckets `symdev-public` (public access on its `r2.dev` URL) and `symdev-private` (no
   public access).
3. R2 API tokens: publisher (object read & write on both) and reader (object read on
   `symdev-private`).
4. Repository `4akloon/symdev-packages`; environment `publish` limited to `main` with the
   publisher key; the reader key as a secret in `symdev`; branch protection on both.

## 11. Out of scope

Licence acceptance, `symdev device create`, emulator and firmware packages, the EKA2L1 fork,
`symdev self update` (re-running `install.sh` updates), release channels, macOS/Windows hosts,
a per-project
toolchain pin, index signing (proposed for phase 2: an ed25519 key kept offline, public key
in symdev, so a leaked publisher key cannot swap the compiler).

## 12. Prebuilt symdev and the Rust SDK (added 2026-10-02)

The owner asked for the store to hold our own tools ready-built, so a clean machine needs no
Rust for a C++ project.

- **Packages.** `symdev;<ver>` holds `bin/symdev`; `rust-sdk;<ver>` holds the Rust SDK in
  the repository's own layout — the root `Cargo.toml`, `crates/symdev-locale` and
  `symbian-rs` — because `symbian-macros` depends on `../../../crates/symdev-locale`, which
  takes its version and edition from the root `[workspace.package]` (found 2026-10-02); the
  SDK proper is `<package>/symbian-rs`. Both MIT, both in the public bucket, `<ver>` = the workspace version of the tagged
  release. `Pins::rust_sdk()` pins `rust-sdk;<this symdev's version>`.
- **Finding the Rust SDK.** `SYMDEV_RUST_SDK` first; then the source checkout symdev was built
  from, if it still exists (a developer working on the SDK keeps using their tree); else the
  installed `rust-sdk` package, auto-installed like GCCE. Today's compile-time path alone
  cannot work for a prebuilt binary.
- **Known gap (follow-up, not phase 1).** `symdev new --lang rust` writes absolute SDK paths
  into the project (`Cargo.toml` path dependencies, `.cargo/config.toml` `build.target`). With
  the package route these name `rust-sdk/<ver>/`, so after an upgrade a build silently mixes
  two SDK versions, and uninstalling the old one breaks the project. Proposed fix: the build
  keeps a `build/rust-sdk` link to the SDK it resolved and the scaffold writes relative paths
  through it; needs an experiment on how the pinned nightly resolves a relative
  `build.target` first.
- **Build.** Static, `x86_64-unknown-linux-musl`, so the binary runs on any Linux whatever its
  glibc; this can only be proven in CI (no musl tools on the owner's host). Fallback if musl
  fails: a glibc build in a Debian 11 container, as for GCCE.
- **Release.** A recipe `recipes/symdev/<ver>/recipe.toml` in `symdev-packages` names a git
  tag of `4akloon/symdev`; its CI checks the tag out, builds, and `publish public`es both
  packages (source code: the tag's archive). Merging the recipe is the release, the same
  model as GCCE, and the publisher key stays in one repository.
- **`install.sh`** (POSIX `sh`, in `symdev-packages` and at the public bucket's root, served
  `no-cache`): reads the public `index.toml`, takes the highest `symdev;*` with an archive for
  this host, downloads it, checks its SHA-256 (`sha256sum`), extracts it into
  `$SYMDEV_HOME/symdev/<ver>/` with a receipt, and links `~/.local/bin/symdev` to it.
  Re-running it updates. It touches nothing else.

