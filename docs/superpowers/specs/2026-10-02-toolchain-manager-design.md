# Toolchain manager and hosted packages: design (2026-10-02)

Status: **design approved section by section** by the repository owner on 2026-10-01/02,
and this written spec, §1's rule included, on 2026-10-02; built and accepted as release
v0.1.0 (§13). What the brainstorm and the work notes found beyond the design is in §14.

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
[licensing.md](../../research/licensing.md) ("never downloaded by this repository") forbade
what phase 1 does with the private bucket. **Replacement**, approved by the owner with this
spec on 2026-10-02 and now in both files (`CLAUDE.md` word for word):

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

**Ids are validated**: segments are non-empty, do not start with `.` (so not `.` or `..`,
nor the home's own `.staging` and `.lock`), and contain no `/`, `\`, NUL or whitespace. The
id maps
to an install path by replacing `;` with `/`: `gcce;12.1.0` → `gcce/12.1.0`.

**Archives** are `.tar.gz` (`flate2` is already a workspace dependency; the `tar` crate is
new); every gzip member is read, so an archive written in several members (`pigz`, or
concatenated) is not cut short at the first. The archive root is the package root. Archives are stored content-addressed,
`<kind>/<…>/<sha256>.tar.gz`, and never overwritten.

**Index**: one `index.toml` per source.

```toml
schema = 1

[[package]]
id = "gcce;12.1.0"
license = "GPL-3.0-or-later AND MIT"
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
uploaded with `Cache-Control: no-cache`; archives with `immutable`. Since 0.2.0 its first line
is the Ed25519 signature of the rest (§15).

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
key = "builtin"         # the index must be signed by symdev's own key (§15; 0.2.0+)
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
2. under the cache's own `downloads/.lock` (several `SYMDEV_HOME`s may share one
   `XDG_CACHE_HOME`), reuse the cached file if it has the right size and hash, else
   download into a `.part` file named for this process and rename it over the cached name
   only once verified — a cached file is never written into nor deleted, so another
   install that opened it keeps reading verified bytes (review I2, 2026-10-02);
3. verify size and SHA-256; on mismatch delete the download and fail (§5); the archive is
   extracted from the handle that was verified;
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
   `installing gcce;12.1.0 (67.5 MB) from public…`;
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

**Network.** `ureq` with rustls, proxy from the environment, and timeouts: 30 s to connect,
60 s for the server's answer, and for the body a minute plus its size at 16 KiB/s — the
index's `size` for an archive, the 10 MiB cap for an index (`HttpTimeouts`, symdev 0.2.0).
ureq 3.4.2 has no idle timeout; its `timeout_recv_body` is a total for the body, set per
request, so a server that sends its headers and then stalls is dropped once a 16 KiB/s
link would have delivered everything (70 minutes for the 67 MB `gcce;12.1.0`). The rate is
a floor for a stalled body, not a speed requirement: 0.1.0's hour let GCCE through a link
of about 19 KB/s, and a 32 KiB/s floor (the first 0.2.0 draft) would have shut such a link
out for good, since a failed download's `.part` is deleted and the next build starts it
from zero (review 0.2.0, minor 1). A whole request may take an hour, or connect + answer +
body when that is longer. A download stops one byte past the index's `size`. An `https`
source sends no plain-HTTP request, a redirect included (an index from a source without a
key has nothing but TLS to check it, §15), and a signed request follows no redirect (review
M1, M2, 2026-10-02). Sources with
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
| index unsigned, with a malformed signature line, or not verifying, where the source requires a signature (§15) | the index URL, which of the three, the trusted keys' fingerprints, and that it may have been tampered with; the source counts as unreadable |

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
- Built in an AlmaLinux 8 container (glibc 2.28, supported until 2029), so it runs on any
  Linux with glibc 2.28 or newer. Debian 11 was the first choice; its LTS ended on
  2026-08-31 and its security pool already returned 404 in the first publish run
  (2026-10-02), so the build moved before it ever ran.
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
- Every index it writes is signed, and it extends only an index whose signature verifies
  (§15).
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
  build` and `symdev package` for `examples/hello`, `examples/gui` and
  `symbian-rs/examples/hello`.
  The reader key is set **only** on the install step, which runs nothing but symdev:
  `cargo build` runs third-party build scripts, and the build and package steps find
  everything installed and read no source;
- sets `RUSTFLAGS` (`-D warnings`) only on the Rust gate. (It once had to: a set
  `RUSTFLAGS`, even an empty one, replaced the `build.rustflags` that `symdev build` passed
  to the Rust SDK's libcall build. Since 0.2.0 that flag is an argument of `cargo rustc`,
  which no developer's rustflags replace — experiment 111.)
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
key = "builtin"   # since 0.2.0 (§15); symdev 0.1.0 refuses the unknown key
```

Then `curl -fsSL <public bucket>/install.sh | sh`, and the first `symdev build` of any
project installs the rest (`symdev sdk install` installs the same set explicitly, without
building). A C++ project needs no Rust at all; a Rust project needs rustup (its nightly comes
from `rust-toolchain.toml`) and a host C linker `cc`, which build scripts and proc macros
link with (found in acceptance, 2026-10-02). Building symdev from source with
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
toolchain pin. (Index signing, proposed here for phase 2, came in 0.2.0: §15.)

## 12. Prebuilt symdev and the Rust SDK (added 2026-10-02)

The owner asked for the store to hold our own tools ready-built, so a clean machine needs no
Rust for a C++ project.

- **Packages.** `symdev;<ver>` holds `bin/symdev`; `rust-sdk;<ver>` holds the Rust SDK in
  the repository's own layout — the root `Cargo.toml`, `crates/symdev-locale` and
  `symbian-rs` — because `symbian-macros` depends on `../../../crates/symdev-locale`, which
  takes its version and edition from the root `[workspace.package]` (found 2026-10-02); the
  SDK proper is `<package>/symbian-rs`. Both MIT, both in the public bucket, `<ver>` = the workspace version of the tagged
  release. `Pins::rust_sdk()` pins `rust-sdk;<this symdev's version>`. An installed
  package (and any SDK `RustSdk::at` accepts) must hold the target spec, `rust-toolchain.toml`,
  `symbian-rs/Cargo.toml`, the two files outside `symbian-rs`, and the `Cargo.toml` of
  every member that workspace manifest names (`RustSdkWorkspace`, 0.2.0): the libcalls
  build of every Rust application runs in the SDK's workspace, and cargo loads all of its
  members first, so a package without `symbian-rs/examples` is refused with the reinstall
  command instead of failing inside cargo.
- **Finding the Rust SDK.** `SYMDEV_RUST_SDK` first; then the source checkout symdev was built
  from, if it still exists (a developer working on the SDK keeps using their tree); else the
  installed `rust-sdk` package, auto-installed like GCCE. Today's compile-time path alone
  cannot work for a prebuilt binary. A release build has **no** checkout step:
  `RustSdk::CHECKOUT` is `None` when `SYMDEV_RELEASE` is set (not empty) at compile time, so
  on another machine nobody can plant a `symbian-rs` at the path of the machine that built
  it (review, 2026-10-02). The release recipe must build with `SYMDEV_RELEASE=1`.
- **Projects name the SDK through a link (resolved 2026-10-02, symdev 0.2.0; was the known
  gap of 0.1.0).** 0.1.0's `symdev new --lang rust` wrote the SDK's absolute paths into the
  project (`Cargo.toml` path dependencies, `.cargo/config.toml` `build.target`), which with
  the package route name `rust-sdk/<ver>/`: after an upgrade a build mixed two SDKs, and
  uninstalling the old one broke the project. Now `build/rust-sdk` is a link (`RustSdkLink`)
  to the directory **above** the resolved `symbian-rs` — the package root, or the checkout's
  root — and the project names `build/rust-sdk/symbian-rs/crates/<crate>` and
  `build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json`. `symdev new` makes the link,
  every `symdev build` re-points it at the SDK it resolved, and the scaffold copies that
  SDK's `rust-toolchain.toml`. Experiment 110 is the evidence: cargo joins a path
  dependency's `..` lexically, so a link to `symbian-rs` itself breaks `symbian-macros`'
  `../../../crates/symdev-locale`; a relative `build.target` in a config file is relative to
  the directory holding `.cargo/` (any cwd); `-Zbuild-std` and the spec work through the
  link, cargo canonicalising the spec's path; and cargo keeps outputs built from the old tree
  when the link moves to one with older mtimes, so a re-point removes `build/cargo` first.
  A project inside that directory — the SDK's own `symbian-rs/examples/*` built against
  their checkout — gets no link (and loses one a 0.2.0 development build made): it names
  the SDK by relative paths, and a link to an ancestor is a cycle that `grep -R`, `find -L`
  and the like walk again (review 0.2.0, minor 2). A scaffold placed there would not build
  anyway: cargo refuses it as an unlisted member of the tree's workspace.
  Before cargo runs, `symdev build` refuses (and does not rewrite) a project that still names
  another SDK by absolute path — listing each `file:line` with the line as it is and as it
  should be — and one whose `rust-toolchain.toml` names a channel other than the SDK's (a
  project without that file, like the SDK's own examples, is not checked). The SDK directory
  must be named `symbian-rs`, as it is in a checkout and in the package.
- **Build.** Static, `x86_64-unknown-linux-musl`, so the binary runs on any Linux whatever its
  glibc; this can only be proven in CI (no musl tools on the owner's host). Fallback if musl
  fails: a glibc build in the AlmaLinux 8 container, as for GCCE.
- **Release.** A recipe `recipes/symdev/<ver>/recipe.toml` in `symdev-packages` names a git
  tag of `4akloon/symdev`; its CI checks the tag out, builds, and `publish public`es both
  packages (source code: the tag's archive). Merging the recipe is the release, the same
  model as GCCE, and the publisher key stays in one repository.
- **`install.sh`** (POSIX `sh`, in `symdev-packages` and at the public bucket's root, served
  `no-cache`): reads the public `index.toml`, takes the highest `symdev;*` with an archive for
  this host, downloads it, checks its SHA-256 (`sha256sum`), extracts it into
  `$SYMDEV_HOME/symdev/<ver>/` with a receipt, and links `~/.local/bin/symdev` to it.
  Re-running it updates. It touches nothing else.


## 13. Acceptance, as run on 2026-10-02 (release v0.1.0)

Published: `gcce;12.1.0` (67 463 658 bytes, built by symdev-packages CI on AlmaLinux 8, with
its GPL source archive), `rust-sdk;0.1.0` (377 928), `symdev;0.1.0` (3 112 677, static-pie
musl, with `LICENSE` and `THIRD-PARTY-NOTICES.txt`), `install.sh` at the public root
(`text/plain; charset=utf-8`, `no-cache`); `sdk;s60-3rd-fp2;1.1` (4 941 155) in the private
bucket. The clean environment was an empty `HOME` with `PATH=/usr/bin:/bin` on the owner's
host (no `cargo`, `rustc` or `g++` on it), the reader key and `sources.toml` only; no docker
or podman exists on the host, so the container variant was not run, and the CI runner
(item 3) is the second clean machine.

| # | Check | Result |
|---|---|---|
| 1 | `hello`, `gui` built with the **published** `gcce;12.1.0` vs `~/gcc-builds`, same path | `.elf` byte-identical (24 244 / 58 532 bytes); `.exe` identical outside 0x14–0x17 and 0x24–0x2B |
| 2 | `curl … install.sh \| sh`, then `symdev build` + `symdev package` | installed `symdev;0.1.0`; the build installed `gcce;12.1.0` (67.5 MB, public) and the SDK (4.9 MB, private); `hello.sisx` and `gui.sisx` installed and launched in EKA2L1 (RM-469): console "Hello, world! [press any key]"; Avkon status pane "gui", "Hello from symdev", softkey Exit (PID-bound screenshots) |
| 3 | CI `examples` on `main` | green (run 37051833143, re-run after the packages were published; the first attempt failed only because `gcce` was not yet in the index) |
| 4 | the owner's `SYMDEV_*` environment | builds `hello` and `gui` with no install line, outputs identical to item 1's reference |
| 5 | `symdev new rhello --lang rust` + build + package with the prebuilt symdev | installed `rust-sdk;0.1.0` (0.4 MB, public), `rhello.exe` 968 bytes, `rhello.sisx` 2 304; in EKA2L1 the guest's `User::InfoPrint` arrived as `Trying to display: Hello from Rust SDK (19 chars)` (EKA2L1 logs InfoPrint and draws nothing — `notifier.cpp` TODO — the signal experiments 69/71 use) |

Found during acceptance: a Rust project needs a host C linker `cc` (build scripts, the SDK's
proc macros, `-Zbuild-std`'s `compiler_builtins`), as any Rust project does — now in the
README and §8. Open follow-ups: the scaffold's absolute `rust-sdk/<ver>` paths (§12, resolved in
0.2.0 by the `build/rust-sdk` link); building Rust applications without GCCE (spike, experiment 109); index
signing (§11; done in 0.2.0, §15).


## 14. As built: decisions and findings not obvious from the code

Kept from the brainstorm and the work notes of the build (2026-10-01/02), which were deleted
once v0.1.0 was accepted; what the code's doc comments and experiments 107–108 already say
is not repeated. "First review" is the independent review of 34987b3 (rows 1–11); C1, I1–I3
and M1–M10 are the whole-branch review of 669dae1, as elsewhere in this spec.

**Packages and index**

- **`git archive` output cannot be installed**: it begins with a PAX global header, which
  `TarGz` refuses ("entry `pax_global_header` has an unsupported type", rechecked
  2026-10-02), as it refuses sparse entries. Archives are packed by `ReproducibleTarGz`, so
  the `symdev` recipe unpacks the tag's `git archive` and `publish` repacks it.
- **A lexical check of symlink targets is not enough**: after `a/b/c/s -> ../../..` (the
  package root), `a/b/c/t -> s/..` reads as `a/b/c` but resolves to the root's parent. Hence
  links are created after every file and directory and then walked the way the kernel does.
- **The SDK subset was proven sufficient**: `hello`, `gui`, the Rust hello and a DLL (built
  as in experiments 52–53; a DLL links `edll.lib`) are byte-identical built against the
  subset and against the full SDK. The archive holds 2 697 files, 31 MB unpacked, SHA-256
  `cbec6da8…2a5f`, and packs to the same bytes every time.
- **GCCE archive size**: `ReproducibleTarGz` packs a host-built prefix (about 200 MiB) in
  about 8 s into 72.4–72.5 MB; the CI-built package is 67 463 658 bytes (§13).
- **Rejected by the owner (2026-10-01)**: GitHub Releases as the store (object storage
  instead), a separate `sdkmanager`-like binary (the manager lives in symdev, so a build can
  install in-process), and conda, pixi or OCI images as the package format.
- **For phase 2**: the EKA2L1 Z: drive made from the E52 firmware is 208 MB; the host's
  EKA2L1 build links a private sysroot and Qt under `~/.local` (eka2l1-host skill), so it
  cannot be packaged as it is.

**Installs and cache**

- **The first source that lists an id decides.** If it has no archive for this host, the
  install fails naming that source instead of taking a later source's build of the same id;
  `sdk list` applies the same rule, so it never offers what `install` refuses (first
  review, row 2).
- **A source that cannot be read is skipped like a keyless one** (unreachable, 403, a
  malformed or newer index); its reason shows only in a not-found error. The built-in source
  is searched first by every build, and its `index.toml` answered 404 until the first publish
  (2026-10-02): failing there would have stopped builds whose packages all came from the
  private source. The price: while an earlier source is down, a later one holding the same id
  supplies it, and the receipt names which.
- **Why each download has its own `.part` (review I2)**: with one shared
  `<sha256>.tar.gz.part` and no cache lock, installs into 8 homes sharing one cache failed
  3/3, and so did 6 parallel `symdev sdk install` processes, each deleting or replacing the
  others' file. Under `downloads/.lock` a part file belongs to one download, so a part of the
  same archive found there is an interrupted run's and is deleted.
- **Both locks were proven by mutation**: without `File::lock` on `SdkHome` two threads
  installing one id download twice (3/3), with it 20/20 runs download once; without the
  cache lock the 8-home test fails 3/3.
- **Writing the receipt before the rename (§3 step 6, review M3) is reasoned, not tested**:
  both refusals of an archive's own receipt pass in either order.

**Network and signing**

- **TLS roots**: ureq 3.4.2's `rustls` feature already brings ring and the Mozilla roots
  (`webpki-roots` 1.0.9), so no further feature is needed; checked against
  `https://index.crates.io/config.json`.
- **ureq 3.4.2 behaviour the adapter relies on**: `Config::default()` takes the proxy from
  `ALL_PROXY`/`HTTPS_PROXY`/`HTTP_PROXY` (lower-case too) and `NO_PROXY`, and plain-`http`
  requests are tunnelled through an HTTP proxy with `CONNECT` too; there are no timeouts by
  default, `https_only` is off and 10 redirects are followed; `Host` carries `:port` only for
  a non-default port, which the signer copies; `read_to_string` is lossy and capped at
  10 MB, `as_reader` is not; a `Content-Length` header makes a `File` body sized, not chunked.
- **The proxy finding (first review, row 3)**: with `ALL_PROXY=http://127.0.0.1:9`, 8 of the
  11 `http_fetch` tests failed, their requests to 127.0.0.1 going to the proxy. Hence the
  tests' proxy-less `HttpFetch::direct`, and a test that proves in a child process that
  production still follows the variables.
- **`HTTP 404` stays the exact `detail` of `SdkError::Fetch`** because the publisher
  (symdev-packages `publish/src/bucket.rs`) reads a missing `index.toml` as an empty index:
  that is how a new bucket gets its first one. 403 alone has its own variant (§5).
- **SigV4 vectors, dead ends**: beyond the two sources `sigv4/tests.rs` cites, the S3 API PDF
  now carries only SigV2 examples, the IAM SigV4 pages carry none, and smithy-rs's old
  `aws-sig-v4-test-suite/get-vanilla` path is 404.

**Toolchain resolution and CLI**

- **For a Rust project the Rust SDK is resolved, and `rust-sdk` installed, before GCCE**, so
  a stale `SYMDEV_RUST_SDK` fails before a 67 MB download (first review, row 6); every set
  `SYMDEV_*` path is likewise checked before anything is downloaded.
- **The prebuilt-symdev CLI tests cost disk**: `tests/common/prebuilt.rs` writes an 88 MB copy
  of the debug binary per run of the `rust_sdk` test binary, into that process's test home,
  which the next run sweeps.

**Rust SDK package**

- **`symbian-rs` alone does not resolve** (`cargo metadata` on a scaffolded project): a bare
  tree gives "no matching package named `symdev-locale`", searched outside the package
  directory; adding `crates/symdev-locale` gives "error inheriting edition from workspace
  root manifest"; adding the root `Cargo.toml` resolves (cargo loads none of the root's other
  members for a path dependency). Alternatives not taken (the owner's call): give
  `symdev-locale` its own version and edition under `symbian-rs/`, or drop
  `symbian-macros`' dependency on a host crate.
- **What else the package holds** (symdev-packages `recipes/symdev/0.1.0/recipe.toml`,
  measured by building a Rust hello, `examples/ui` and `examples/std-hello` against a
  read-only copy): `symbian-rs/examples`, because they are members of the SDK workspace in
  which `symdev build` builds `symbian-libcalls` (without them: "failed to load manifest for
  workspace member …/examples/async"); `symbian-rs/Cargo.lock`, or that build would write a
  lock into the package; `LICENSE`. Not `symbian-rs/corpus/`, which no build reads. The
  recipe's `build.sh` fails on a new `symbian-rs/` entry its list does not name.
- **The in-tree `symbian-rs/examples` are not projects on the package**: a copy outside the
  clone fails ("`-Z` flag is only accepted on the nightly channel": its nightly and version
  come from the `symbian-rs` workspace), and inside the clone a prebuilt symdev compiles the
  clone's crates against the package's shims and libcalls. Build them with a symdev from the
  same clone or `SYMDEV_RUST_SDK=<clone>/symbian-rs`.
- **Rust outputs depend on the source path**: the Rust hello built from the checkout and from
  the package gives the same `.exe` outside the eight header bytes, but the in-tree example
  and a scaffolded copy of it differ in the `.elf`'s `.strtab` (47 bytes), because rustc's
  symbol hashes include the crate's path. Compare Rust `.elf` files only from one path, as
  experiment 107 did.
- **The scaffold gap (§12), as reproduced**: after an upgrade, `symdev build` passes the new
  SDK's target, libcalls and shims while cargo compiles the application against the old
  `symbian-std` and `symbian-core`, with no error; once `rust-sdk/0.1.0` is gone (reproduced
  by renaming it) cargo stops with "no matching package named `symbian-core` found"; the
  project keeps the old nightly in its `rust-toolchain.toml`; the checkout route breaks the
  same way when the checkout moves. The `build/rust-sdk` proposal also needs `symdev build`
  to compare the project's channel with the SDK's and stop with the fix, since rustup reads
  that file from the project only.
- **`SYMDEV_RELEASE` checked by hand**: a binary built with `SYMDEV_RELEASE=1` holds the
  checkout path 0 times (a normal build: once), and `symdev --offline new app --lang rust` in
  an empty home asks for `rust-sdk;0.1.0` where the normal build scaffolds from the checkout.

**CI and publishing**

- **Review I3, measured**: in a scratch crate, `cargo build -v --config
  'build.rustflags=["--cfg","foo"]'` passes `--cfg foo` with `RUSTFLAGS` unset and not with
  `RUSTFLAGS=""` or `RUSTFLAGS="-D warnings"` (rechecked 2026-10-02). So the review's
  `RUSTFLAGS: ""` would not have helped; `-D warnings` moved to the `check` job (§7).
- **Review C1 leaked nothing**: the SDK cache was found before the `examples` job had ever
  run (its secrets were set after the last push), so no cache holding the SDK was saved.
- **In CI the Rust example builds against the checkout** (the job builds symdev without
  `SYMDEV_RELEASE`), so the job never installs `rust-sdk`: `symdev --offline sdk install` names exactly
  `'gcce;12.1.0' 'sdk;s60-3rd-fp2;1.1'` in each of the three examples, which is why the keyed
  install step covers everything the keyless build step needs.
- **No self-hosted runner**: one on the owner's machine was ruled out as unsafe for a public
  repository; private files reach CI only through a private bucket and a secret.
- **A published version that a released symdev pins is never deleted** (publishing rules
  approved by the owner, 2026-10-01): old releases install their pins by id.
- **GitHub registers a workflow only once an event has triggered it**: `workflow_dispatch` of
  a pushed, never-triggered workflow answered 404, so symdev-packages' `symdev.yml` and
  `publish.yml` were first started by real recipe edits (runs 37052134487 and 37052194350);
  afterwards dispatch works (GCCE was published by run 37052495523, 16 min).
- **`rust-sdk` is published before `symdev`** (`symdev.yml`), so the index never offers a
  symdev whose Rust SDK is missing.
- **`LIBZ_SYS_STATIC=1`**: a local musl build with the host's gcc as `CC` (the host has no
  musl C compiler) linked static-pie and reached r2.dev over TLS, but libz-sys had linked the
  host's glibc-built libz. The recipe therefore sets the variable, so libz-sys compiles its
  bundled zlib, and fails unless libz-sys' build output says `rustc-link-lib=static=z`.
- **`install.sh`** is tested against a fake bucket under dash, bash and busybox applets
  (symdev-packages `tests/install.sh.test`, run by `tests.yml`); `publish file` uploads it to
  the bucket root.
- **Settings beyond §7's secrets**: symdev's repository variable `SYMDEV_PRIVATE_SOURCE_URL`
  (the `examples` job's `sources.toml`); symdev-packages' variable `PUBLIC_READ_URL` and, in
  environment `publish`, variable `PUBLISH_PUBLIC_URL` and secrets `PUBLISH_ACCESS_KEY_ID` /
  `PUBLISH_SECRET_ACCESS_KEY`. The URLs are not secret.
- **On the owner's host** the keys live in `~/.config/symdev/keys.env` (mode 600), sourced by
  `~/.profile`, so only login shells have them; an agent's tool shell needs `set -a;
  . ~/.config/symdev/keys.env; set +a`.

**Licences**

- **GCCE is `GPL-3.0-or-later AND MIT`** in the published index, the MIT part being our two
  sys-include headers; this settles experiment 108's open SPDX question.
- **The index's `license` is parsed but neither shown nor enforced**: it is there for phase
  2's licence acceptance (§11).
- **The prebuilt symdev's `THIRD-PARTY-NOTICES.txt`** is generated by symdev-packages'
  `tools/third_party_notices.py` from the musl build's dependency graph: 117 third-party
  crates plus the C and runtimes they bring (zlib, ring's BoringSSL and fiat code, Rust's
  `std`, the LLVM runtime, musl). musl 1.2.5's `COPYRIGHT` is taken from its signed release
  tarball, so no entry lacks a licence file. Both `symdev` and `rust-sdk` carry `LICENSE`,
  since MIT asks for the notice in every copy.

**Open follow-ups** (beyond §13's)

- Resolved in 0.2.0 (gaps G1): a stalled body is now bounded per request by 60 s plus its
  size at 16 KiB/s (`HttpTimeouts`, ureq's `timeout_recv_body`); the libcall build runs
  `cargo rustc … -- -Zdefault-visibility=hidden`, so a developer's `RUSTFLAGS` no longer drops
  it (experiment 111); both `REQUIRED` lists check every `symbian-rs` workspace member
  (`RustSdkWorkspace`).

## 15. Signed indexes (added 2026-10-03, symdev 0.2.0)

The owner asked to close the gap §11 left: with only HTTPS and the index's SHA-256s, whoever
can write a bucket (a leaked R2 publisher key) can swap the compiler for every user. Now the
index is signed with a key that is not an R2 credential.

**Format.** The first line of `index.toml` is

```
# symdev-signature: ed25519 <base64 of the 64-byte Ed25519 signature>
```

ending in `\n`, and the signature covers exactly the bytes after that `\n`. To TOML the line is
a comment, so symdev 0.1.0 and the 0.1.0 `install.sh` keep reading a signed index; index and
signature are one object, so no upload leaves them disagreeing. Only the first line can hold a
signature. The signature is plain RFC 8032 Ed25519 (`ed25519-dalek` 2 in symdev, verified with
`verify_strict`; OpenSSL 3's `pkeyutl -rawin` makes and checks the same bytes, which a test
pins). symdev-sdk's `SignedIndex` (`split`, `sign`, `verify`), `TrustedKeys` and
`IndexSigningKey` are the one implementation both symdev and the publisher use.

**Keys.** One project key pair, generated with `openssl genpkey -algorithm ed25519` on
2026-10-03. The private half exists only as its 32-byte seed, base64, in the owner's
`~/.config/symdev/keys.env` (`PUBLISH_SIGNING_KEY`, mode 600) and in the GitHub secret of the
same name in `symdev-packages`' environment `publish`. The public half is built into symdev
(`TrustedKeys::builtin`, a list, so a rotation ships a symdev with both keys before the indexes
are re-signed) and into `install.sh`:

| | |
|---|---|
| public key (raw 32 bytes, base64) | `C1yh60B72Qa4YE4rZOgoPJZmTYKbh/uzHjupoVwqfLU=` |
| fingerprint (SHA-256 of the raw 32 bytes, hex) | `bdf5345cc3ca8c30661dbc53b2cbd16983d081bf26913d0c3ebe7480ca334d44` |
| PEM body, as `install.sh` builds it | `MCowBQYDK2VwAyEAC1yh60B72Qa4YE4rZOgoPJZmTYKbh/uzHjupoVwqfLU=` |

A new key (a rotation, or a mirror's own): the seed is the last 32 bytes of the 48-byte PKCS#8
DER that `openssl genpkey -algorithm ed25519 -outform DER` writes, base64; the public half is
the last 32 bytes of `openssl pkey -pubout -outform DER` of the same key, base64.

**Verification in symdev.** The built-in source must carry a valid signature by a built-in key:
an index that is unsigned, has a malformed signature line or does not verify makes the source
unreadable, with an error naming the URL and which of the three it is. A source in
`sources.toml` may set `key = "builtin"` (the built-in keys) or `key = "<base64 Ed25519 public
key>"`; then the same rule holds for it. Without `key` a source's index is read unverified, as
in 0.1.0 (the tests' `file://` sources stay unsigned unless a test opts in). Only the verified
bytes are parsed. An index refused for its signature is a warning on stderr even when a later
source provides the package (a sign of tampering is never silent). A `key` that is not a valid
or is a weak (small-order) Ed25519 key is refused when `sources.toml` is read. The owner's
private source uses `key = "builtin"` once 0.2.0 is installed (0.1.0's `sources.toml` parser
refuses unknown keys).

**Signing in the publisher.** `publish public|private` signs every index it writes with
`PUBLISH_SIGNING_KEY`, which an upload requires and which must be one of symdev's built-in
keys (a stale or mistyped key would sign an index every client refuses, and the right key
could then not even re-sign it; review, 2026-10-03); the upload names the signer's
fingerprint. A `--dry-run` signs with it when it is set and says when it is not, or when
symdev would not trust it. It extends only an index whose signature verifies with a built-in key or
the signing key's own public half: a bad signature is refused (the bucket was written by
someone else), and an unsigned index is refused by an upload and only warned about in a dry
run. This check was added to the lead's design (2026-10-03) because without it the next CI
publish would sign whatever a leaked R2 key had written, which is the attack the signature
exists to stop. `publish sign-index --bucket public|private [--accept-unsigned <sha256>]
[--dry-run]` re-signs the existing index as it is: it reads it, checks that a signature it has
verifies and that it parses, lists its archives, signs the same body and uploads it with
`no-cache` (nothing when no byte would change). It is how the indexes published before 0.2.0
get their signature, and the one way to accept an unsigned index. After that migration an
unsigned index means someone else wrote the bucket, so an unsigned index is signed only with
`--accept-unsigned` and the SHA-256 that the dry run printed for it: exactly the bytes the
owner checked, never a later swap (review, 2026-10-03).

**install.sh** verifies the index when `openssl version` reports OpenSSL 3 or newer
(`openssl pkeyutl -verify -pubin -inkey <pem> -rawin -in <body> -sigfile <sig>`), with the
public key in the script: a missing or bad signature is then a hard error. Without OpenSSL 3 it
warns that the index could not be verified and goes on (HTTPS and SHA-256 still apply).
`SYMDEV_INSTALL_PUBKEY` replaces the key (base64 public keys, space-separated) for tests and
for mirrors signed with their own key.

**CI.** `publish.yml` and `symdev.yml` give `secrets.PUBLISH_SIGNING_KEY` only to the steps
that upload an index; the install.sh upload and the pull-request dry runs do not see it.

**Limits** (recorded, not solved here). The signing key sits beside the R2 publisher key, in
the `publish` environment and in `keys.env`, so the signature protects against a leak of the R2
key alone (or of a bucket token), not of both; §11's original proposal kept the key offline.
`install.sh` is served from the same public bucket, so whoever can write the bucket can replace
the script a first `curl … | sh` runs; the signature protects every later `symdev` download
and re-runs of a saved script. The signature binds no source name, date or version, so an
older validly signed index can be served again (a rollback to packages that were good when
published).

**Owner steps.** (1) Add the secret: `set -a; . ~/.config/symdev/keys.env; set +a; printf
'%s' "$PUBLISH_SIGNING_KEY" | gh secret set PUBLISH_SIGNING_KEY --repo 4akloon/symdev-packages
--env publish`. (2) Re-sign the live indexes once, from the packages repository with
`keys.env` loaded: `cargo run -p publish -- sign-index --bucket public --dry-run`, check every
archive it lists against §13 and the recorded hashes, then run it without `--dry-run` and
with `--accept-unsigned <the SHA-256 it printed>`; then both again with `--bucket
private`. (3) Only
then upload the new `install.sh` (it refuses an unsigned index) and release symdev 0.2.0 (its
built-in source must be signed). (4) After installing 0.2.0, add `key = "builtin"` to the
private source in `sources.toml`.
