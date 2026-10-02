# Toolchain Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `symdev build` on a machine with only rustup, git and a reader key installs GCCE and
the S60 SDK from R2 by itself and builds; the GCCE and SDK packages exist and are published.

**Architecture:** a new crate `symdev-sdk` (package ids, index, sources, safe archives,
`SdkHome`, layouts, pins, HTTP + SigV4) used by `symdev-build`'s `Toolchain` and by a new
`symdev sdk` CLI; a separate local repository `~/projects/symdev-packages` with recipes and a
`publish` binary on the same crate; a CI job that builds the examples through auto-install.

**Tech Stack:** Rust 1.98.1 / edition 2024, `ureq` 3 (rustls), `tar`, `flate2`, `sha2`,
`hmac`, `toml`, `serde`, `thiserror`; GitHub Actions; Cloudflare R2 (S3 API).

**Spec:** [2026-10-02-toolchain-manager-design.md](../specs/2026-10-02-toolchain-manager-design.md)
— every task's implementer reads it first.

## Global Constraints

- Every `.rs` file ≤ 300 lines including tests; one type per file; long tests in `<module>/tests.rs`.
- Library code returns `Result`; no `unwrap`/`expect`/`panic!` outside tests; every error names what failed and the fix.
- Value types never read env, argv, stdout or the network; only adapters (`HttpFetch`, `FileFetch`, `SdkManager`, the CLI) do I/O to sources or read env.
- `cargo test --workspace --offline` and `cargo clippy --workspace --all-targets --offline` clean, zero warnings; `cargo fmt --all --check` clean.
- `cargo test` never touches the network or R2 (tests use `file://` sources, local `TcpListener`s, temp `SYMDEV_HOME`/`XDG_*`).
- Commit messages: one full imperative sentence ending with a period, then a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Never commit SDK, firmware, `.sis`, `.sisx`, `.cer`, `.key` files; SDK bytes live only in `~/sdk` and the private bucket.
- Package ids: `gcce;12.1.0`, `sdk;s60-3rd-fp2;1.1`; index `schema = 1`; archives `.tar.gz`, stored as `<id path>/<sha256>.tar.gz`.
- Env names: `SYMDEV_HOME`, `SYMDEV_SOURCE_<NAME>_ACCESS_KEY_ID`, `SYMDEV_SOURCE_<NAME>_SECRET_ACCESS_KEY` (`<NAME>` = source name upper-cased, `-` → `_`).
- Do not invent tool argv or configure flags: only what is observed (`g++ -v`, recorded runs).
- Keep `docs/research/wip/toolchain-manager.md` current after every finding; commit after every completed task.

## Review Focus

1. **Interrupted download** (Ctrl-C leaves a truncated file in `~/.cache/symdev/downloads`): the next install must re-download, never extract it. Test owned by Task A5.
2. **Partial environment** (only `SYMDEV_GXX` set): `gcce` is still installed for `ld`/libs, `gxx` comes from env. Test owned by Task D1.
3. **Index lists the id only for another host**: error says "no archive for x86_64-linux", not "not found". Test owned by Task A2 (`archive_for`) and D2 (message).
4. **First source unreachable or keyless, id also in a later source**: install from the later source; the keys hint appears only if the id is found nowhere. Test owned by Task D2.
5. **Receipt present but files deleted by hand**: `Gcce::at` / `PlatformSdk::at` fail naming the missing file and suggesting `symdev sdk uninstall <id> && symdev sdk install <id>`. Test owned by Task A6.

---

## Tracks and waves

| Wave | Track | Tasks | Worktree / branch | Needs |
|---|---|---|---|---|
| 0 | scaffold | 0 | `toolchain-manager` | — |
| 1 | A sdk-core | A1–A6 | `~/worktrees/symdev/tm-core` / `tm-core` | Task 0 |
| 1 | B sdk-net | B1–B3 | `~/worktrees/symdev/tm-net` / `tm-net` | Task 0 |
| 1 | C gcce-recipe | C1–C4 | `~/projects/symdev-packages` + `tm-gcce` for the record | — |
| 1 | F ci-fmt | F1 | `~/worktrees/symdev/ci-fmt` / `ci-fmt` → `main` | — |
| 1 | owner | O1 | Cloudflare dashboard | — |
| 2 | D integration | D1–D3 | `~/worktrees/symdev/tm-cli` / `tm-cli` | A, B merged |
| 2 | E publisher | E1–E3 | `~/projects/symdev-packages` | A, B merged; C4 |
| 2 | G ci-examples | G1 | `toolchain-manager` | D |
| 3 | release | R1–R5 | `toolchain-manager` → `main` | O1, D, E, G |

Every track branch starts from `toolchain-manager` after Task 0 and merges back into it.
Tracks A and B touch disjoint files except `crates/symdev-sdk/src/lib.rs` (one `mod`/`pub use`
line each); the merge resolves that by keeping both lines.

---

### Task 0: Scaffold `symdev-sdk` with the shared error, fetch trait and keys

**Files:**
- Create: `crates/symdev-sdk/Cargo.toml`, `crates/symdev-sdk/src/lib.rs`, `src/error.rs`, `src/fetch.rs`, `src/keys.rs`
- Modify: `Cargo.toml` (workspace members)

**Interfaces — Produces (fixed for all tracks):**

```rust
// error.rs
#[derive(Debug, thiserror::Error)]
pub enum SdkError {
    #[error("invalid package id `{id}`: {reason}")]
    InvalidId { id: String, reason: &'static str },
    #[error("source `{source_name}`: index schema {found} is newer than this symdev understands (1); update symdev")]
    UnknownSchema { source_name: String, found: u32 },
    #[error("source `{source_name}`: bad index: {detail}")]
    BadIndex { source_name: String, detail: String },
    #[error("bad sources file {path}: {detail}")]
    BadSources { path: String, detail: String },
    #[error("{url}: {detail}")]
    Fetch { url: String, detail: String },
    #[error("{url}: access denied (HTTP 403) by source `{source_name}`; check the key's bucket permissions")]
    Forbidden { url: String, source_name: String },
    #[error("{id}: downloaded {url} has sha256 {actual} (size {actual_size}), expected {expected} (size {expected_size}); the file was deleted")]
    HashMismatch { id: String, url: String, expected: String, actual: String, expected_size: u64, actual_size: u64 },
    #[error("archive {url}: entry `{entry}` {reason}")]
    UnsafeEntry { url: String, entry: String, reason: &'static str },
    #[error("{path}: {source}")]
    Io { path: String, #[source] source: std::io::Error },
    #[error("{0}")]
    Other(String),
}
pub type Result<T> = std::result::Result<T, SdkError>;
impl From<SdkError> for symdev_core::Error { fn from(e: SdkError) -> Self { symdev_core::Error::Other(e.to_string()) } }

// fetch.rs
/// Where a source's bytes come from: `file://` directories (tests, local mirrors) or HTTP(S).
pub trait Fetch {
    /// The whole body of `url` as UTF-8 text (an `index.toml`).
    fn text(&self, url: &str) -> Result<String>;
    /// Streams `url` into `dest` (created or truncated); returns the bytes written.
    fn download(&self, url: &str, dest: &std::path::Path) -> Result<u64>;
}

// keys.rs
/// An S3 key pair for one source; read from the environment by the CLI, never by this crate.
#[derive(Clone, PartialEq, Eq)]
pub struct S3Keys { pub access_key_id: String, pub secret_access_key: String }
impl std::fmt::Debug for S3Keys { /* prints access_key_id only, secret as "***" */ }
impl S3Keys {
    /// `SYMDEV_SOURCE_<NAME>_ACCESS_KEY_ID` / `_SECRET_ACCESS_KEY` for a source name.
    pub fn variable_names(source_name: &str) -> (String, String);
}
```

- [ ] **Step 1:** add the crate with dependencies `symdev-core`, `symdev-manifest`, `thiserror = "2"`, `serde = { version = "1", features = ["derive"] }`, `toml = "1"`, `sha2 = "0.10"`, `hmac = "0.12"`, `flate2 = "1"`, `tar = "0.4"`, `ureq = { version = "3", default-features = false, features = ["rustls"] }`; dev: `tempfile = "3"`. Add to workspace members.
- [ ] **Step 2:** write the three files above; test `variable_names("private") == ("SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID", "SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY")`, `variable_names("my-mirror")` → `SYMDEV_SOURCE_MY_MIRROR_…`, and that `format!("{:?}", keys)` does not contain the secret.
- [ ] **Step 3:** `cargo fetch` (online, once), then `cargo test -p symdev-sdk --offline`, `cargo clippy -p symdev-sdk --all-targets --offline`.
- [ ] **Step 4:** commit "Scaffold the symdev-sdk crate with its error type, fetch trait and source keys."

---

## Track A — sdk-core (one agent, tasks in order)

### Task A1: `PackageId`

**Files:** Create `crates/symdev-sdk/src/package_id.rs` (+ `package_id/tests.rs` if long).

**Produces:**
```rust
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageId(String);
impl PackageId {
    pub fn parse(s: &str) -> Result<Self>;          // segments split on ';'
    pub fn kind(&self) -> &str;                     // first segment
    pub fn segments(&self) -> impl Iterator<Item = &str>;
    pub fn relative_path(&self) -> std::path::PathBuf;  // "gcce;12.1.0" -> "gcce/12.1.0"
    pub fn as_str(&self) -> &str;
}
impl std::fmt::Display for PackageId; impl std::str::FromStr for PackageId;
impl serde::Serialize / Deserialize for PackageId (as the string, validated)
```

- [ ] Tests first: parses `gcce;12.1.0`, `sdk;s60-3rd-fp2;1.1`; `kind()`; `relative_path()`; rejects `""`, `gcce`, (single segment: reason "needs a kind and a version"), `gcce;;1`, `gcce;..`, `gcce;.`, `gcce;a/b`, `gcce;a\b`, `gcce;a\0b`, `gcce; 1` (whitespace). Display round-trips. Deserialize of an invalid id fails.
- [ ] Implement, run, clippy, commit "Add PackageId, the validated Android-style package path."

### Task A2: `Index`, `IndexPackage`, `ArchiveEntry`, `Host`, `resolve_url`

**Files:** Create `src/index.rs`, `src/index_package.rs`, `src/host.rs`, `src/url.rs` (+ tests).

**Produces:**
```rust
pub enum Host { X86_64Linux, Any }                  // serde: "x86_64-linux", "any"
impl Host { pub fn current() -> Option<Host>; pub fn as_str(&self) -> &'static str; }
pub struct ArchiveEntry { pub host: Host, pub url: String, pub sha256: String, pub size: u64 }
pub struct IndexPackage { pub id: PackageId, pub license: String, pub source_code: Option<String>,
                          pub depends: Vec<PackageId>, pub archives: Vec<ArchiveEntry> }
impl IndexPackage { pub fn archive_for(&self, host: Host) -> Option<&ArchiveEntry>; } // exact host, else Any
pub struct Index { pub schema: u32, pub packages: Vec<IndexPackage> }
impl Index {
    pub fn empty() -> Self;                         // schema 1, no packages
    pub fn parse(text: &str, source_name: &str) -> Result<Index>;
    pub fn to_toml(&self) -> Result<String>;        // packages sorted by id
    pub fn find(&self, id: &PackageId) -> Option<&IndexPackage>;
    pub fn insert(&mut self, p: IndexPackage) -> Result<()>; // Other("… already published; a rebuild is a new version") if present
}
/// `base` ends with '/'; `relative` must not be absolute, contain "..", "://", or start with '/'.
pub fn resolve_url(base: &str, relative: &str) -> Result<String>;
```
TOML keys exactly as in spec §2: `schema`, `[[package]]` with `id`, `license`, `source-code`, `depends`, `[[package.archive]]` with `host`, `url`, `sha256`, `size`.

- [ ] Tests first: parse the spec §2 example; `schema = 2` → `UnknownSchema{found:2}`; missing `schema` → `BadIndex`; `sha256` not 64 lowercase hex → `BadIndex`; `to_toml` → `parse` round-trip; `insert` duplicate refused; `archive_for(X86_64Linux)` picks exact, falls back to `any`, returns `None` for a package that only has another host (Review Focus 3); `resolve_url("https://x/b/", "gcce/1/a.tar.gz")`; rejects `../a`, `/a`, `https://evil/a`, `a/../b`.
- [ ] Implement, run, clippy, commit "Add the repository index with per-host archives and relative URLs."

### Task A3: `SourceSpec` and `Sources` (sources.toml)

**Files:** Create `src/source.rs`, `src/sources.rs` (+ tests).

**Produces:**
```rust
pub enum Auth { None, S3 }
pub struct SourceSpec { pub name: String, pub base: String /* always ends with '/' */, pub auth: Auth }
impl SourceSpec {
    pub fn index_url(&self) -> String;              // base + "index.toml"
    pub fn is_file(&self) -> bool;                  // base starts with "file://"
}
pub struct Sources { pub list: Vec<SourceSpec> }
impl Sources {
    /// `text` = contents of sources.toml (None if the file does not exist).
    /// Built-in first unless `builtin = false`; names must be unique and match [a-z0-9-]+.
    pub fn parse(text: Option<&str>, path_for_errors: &str, builtin: Option<&SourceSpec>) -> Result<Sources>;
}
```
`sources.toml` keys: top-level `builtin` (bool, default true), `[[source]]` with `name`, `url`, `auth` (`"none"` default or `"s3"`). A `url` without trailing `/` gets one.

- [ ] Tests first: no file → only built-in; `builtin = false` → only listed; order kept; `auth = "s3"`; duplicate name → `BadSources`; name `Bad_Name` → `BadSources`; `url = "ftp://…"` → `BadSources` (only `https://`, `http://127.0.0.1…`/`http://localhost…`, `file://`); trailing slash added.
- [ ] Implement, run, clippy, commit "Add package sources and the sources.toml format."

### Task A4: safe extraction and reproducible packing

**Files:** Create `src/tar_gz.rs`, `src/reproducible.rs` (+ `tests.rs` files).

**Produces:**
```rust
pub struct TarGz<'a> { path: &'a std::path::Path, url: &'a str }  // url only for error messages
impl<'a> TarGz<'a> {
    pub fn new(path: &'a Path, url: &'a str) -> Self;
    /// Extracts into `into` (must exist, empty). Refuses absolute paths, `..`, device/fifo entries,
    /// symlinks and hard links whose target resolves outside `into`. Keeps the executable bit.
    pub fn extract(&self, into: &Path) -> Result<()>;
}
pub struct ReproducibleTarGz;
impl ReproducibleTarGz {
    /// Packs the listed directories of `root` (relative paths, e.g. "epoc32/include") into `out`:
    /// entries sorted bytewise, mtime 0, uid/gid 0, empty user/group names, mode 0o755 for dirs and
    /// files with any x bit, else 0o644; gzip header mtime 0, no file name, level 6.
    /// Symlinks are stored as symlinks only if they stay inside; otherwise an error names them.
    /// Returns (sha256 hex, size).
    pub fn pack(root: &Path, include: &[&str], out: &Path) -> Result<(String, u64)>;
}
```

- [ ] Tests first (archives built in the test with the `tar` crate's `Builder` writing raw headers): `../escape`, `/abs`, `a/../../b`, a symlink `link -> ../../etc` all → `UnsafeEntry` and nothing written outside `into`; an internal symlink `bin/c++ -> g++` extracts; executable bit kept; `pack` twice on the same tree with different mtimes → identical sha; `pack` then `extract` round-trips file bytes and modes; `pack` of a missing include dir → error naming it.
- [ ] Implement, run, clippy, commit "Add safe tar.gz extraction and reproducible packing."

### Task A5: `SdkHome`, `Receipt`, `FileFetch`, install procedure

**Files:** Create `src/home.rs`, `src/receipt.rs`, `src/file_fetch.rs`, `src/home/tests.rs`.

**Produces:**
```rust
pub struct FileFetch;                                // implements Fetch for file:// URLs
pub struct Receipt { pub id: PackageId, pub sha256: String, pub source: String, pub url: String }
impl Receipt { pub fn read(dir: &Path) -> Result<Option<Receipt>>; pub fn write(&self, dir: &Path) -> Result<()>; }
pub struct SdkHome { root: PathBuf, cache: PathBuf }
impl SdkHome {
    pub fn new(root: PathBuf, cache: PathBuf) -> Self;
    pub fn root(&self) -> &Path;
    pub fn package_dir(&self, id: &PackageId) -> PathBuf;
    pub fn installed(&self, id: &PackageId) -> Result<Option<Receipt>>;   // None if no receipt
    pub fn list(&self) -> Result<Vec<Receipt>>;                           // sorted by id
    /// Spec §3 steps 1–6. Takes `root/.lock` (std File::lock), re-checks `installed` after locking,
    /// reuses a cached file only if its size and sha256 match, deletes a mismatching download and
    /// returns HashMismatch, extracts into `root/.staging/<pid>-<n>`, removes a stale receipt-less
    /// package dir, renames, writes the receipt last.
    pub fn install(&self, id: &PackageId, source: &SourceSpec, fetch: &dyn Fetch,
                   entry: &ArchiveEntry) -> Result<Receipt>;
    pub fn uninstall(&self, id: &PackageId) -> Result<bool>;             // under the lock
}
```
Receipt file: `<package dir>/.symdev-package.toml` with keys `id`, `sha256`, `source`, `url`.

- [ ] Tests first, all with a `file://` source in a tempdir built by `ReproducibleTarGz::pack`: install → files present, receipt written; second install → no download (count calls with a wrapper `Fetch`); a dir without receipt → reinstalled; **truncated file in cache → re-downloaded, not extracted** (Review Focus 1); wrong sha in entry → `HashMismatch`, cache file deleted, no package dir; two threads `install` the same id → both `Ok`, one download, one dir; `uninstall` removes dir and returns `true`, then `false`; `list` sorted; staging dir lives under `root`.
- [ ] Implement, run, clippy, commit "Add SdkHome, which installs packages atomically under a lock."

### Task A6: layouts `Gcce`, `PlatformSdk` and `Pins`

**Files:** Create `src/gcce.rs`, `src/platform_sdk.rs`, `src/pins.rs`.

**Produces:**
```rust
pub struct Gcce { root: PathBuf, gcc_version: String }
impl Gcce {
    /// `id` = gcce;<ver>; gcc_version = <ver> up to the first '-'. Checks that gxx and ld exist;
    /// otherwise Other("<path> is missing from installed <id>; run `symdev sdk uninstall <id> && symdev sdk install <id>`").
    pub fn at(root: PathBuf, id: &PackageId) -> Result<Gcce>;
    pub fn gxx(&self) -> PathBuf;            // bin/arm-none-symbianelf-g++
    pub fn ld(&self) -> PathBuf;             // bin/arm-none-symbianelf-ld
    pub fn gcc_lib(&self) -> PathBuf;        // lib/gcc/arm-none-symbianelf/<gcc_version>
    pub fn gcc_target_lib(&self) -> PathBuf; // arm-none-symbianelf/lib
}
pub struct PlatformSdk { root: PathBuf }
impl PlatformSdk {
    pub fn at(root: PathBuf, id: &PackageId) -> Result<PlatformSdk>; // checks root/epoc32/include exists
    pub fn epocroot(&self) -> &Path;
}
pub struct Pins;
impl Pins {
    pub fn gcce() -> PackageId;                                   // gcce;12.1.0
    pub fn platform_sdk(device: symdev_manifest::Device) -> PackageId; // NokiaE52 -> sdk;s60-3rd-fp2;1.1
}
```
`Pins` holds the only literal ids in the code; CI hashes `crates/symdev-sdk/src/pins.rs` for its cache key.

- [ ] Tests first: paths; `gcce;12.1.0-2` → `lib/gcc/arm-none-symbianelf/12.1.0`; **missing `bin/arm-none-symbianelf-g++` → error naming the path and the reinstall command** (Review Focus 5); `PlatformSdk::at` without `epoc32/include` → error; `Pins::platform_sdk(NokiaE52)`.
- [ ] Implement, run, clippy, commit "Add the GCCE and platform SDK layouts and the pinned package ids."

## Track B — sdk-net (one agent, tasks in order)

### Task B1: `SigV4`

**Files:** Create `src/sigv4.rs`, `src/amz_date.rs`, `src/sigv4/tests.rs`.

**Produces:**
```rust
pub struct AmzDate(/* "YYYYMMDDTHHMMSSZ" */ String);
impl AmzDate { pub fn from_unix(secs: u64) -> AmzDate; pub fn now() -> AmzDate; pub fn as_str(&self) -> &str; pub fn day(&self) -> &str; }
pub struct SigV4 { keys: S3Keys, region: String, service: String }
impl SigV4 {
    pub fn s3(keys: S3Keys, region: &str) -> SigV4;   // service "s3"; R2 uses region "auto"
    /// Returns the headers to add: ("x-amz-date", …), ("x-amz-content-sha256", payload_sha256),
    /// ("authorization", "AWS4-HMAC-SHA256 Credential=…, SignedHeaders=…, Signature=…").
    /// Signs host, x-amz-content-sha256, x-amz-date and every `extra` header (lower-cased, sorted).
    pub fn sign(&self, method: &str, url: &str, extra: &[(&str, &str)],
                payload_sha256: &str, now: &AmzDate) -> Result<Vec<(String, String)>>;
}
```
UTC date math by hand (days-from-civil), no `chrono`/`time` crate.

- [ ] Fetch AWS's published examples from the official docs pages (S3 "Signature Calculations for the Authorization Header" GET-object example with `AKIAIOSFODNN7EXAMPLE`, and the generic SigV4 test suite `get-vanilla`); record the exact URLs in the test file's doc comment. Tests: those vectors reproduce byte-for-byte; `AmzDate::from_unix(0)` = `19700101T000000Z`, `from_unix(1440938160)` = `20150830T123600Z`; a URL with a query string is canonicalised (sorted, encoded); a path with `;` or space is percent-encoded once.
- [ ] Implement, run, clippy, commit "Add an AWS Signature V4 signer for S3 GET and PUT requests."

### Task B2: `HttpFetch` (GET)

**Files:** Create `src/http_fetch.rs`, `src/http_fetch/tests.rs`.

**Produces:**
```rust
pub struct HttpFetch { /* ureq::Agent, Option<SigV4>, source name for messages */ }
impl HttpFetch { pub fn new(source_name: &str, signer: Option<SigV4>) -> HttpFetch; }
impl Fetch for HttpFetch   // 403 -> Forbidden; other non-2xx -> Fetch{url, "HTTP <code>"}; timeouts 30 s connect, 10 min total
```
Proxy from `HTTPS_PROXY`/`HTTP_PROXY`/`NO_PROXY` via ureq's env proxy support.

- [ ] Tests first with a `std::net::TcpListener` on 127.0.0.1 in a thread serving canned responses: `text` returns the body; `download` writes the exact bytes and returns the count; 404 → `Fetch` with the URL; 403 → `Forbidden`; with a signer the request carries `authorization` starting `AWS4-HMAC-SHA256 Credential=AKID…/…/auto/s3/aws4_request`, and without one it carries none.
- [ ] Implement, run, clippy, commit "Add HttpFetch, the HTTP source adapter with optional S3 signing."

### Task B3: `HttpFetch::put_file` (for `publish`)

**Produces:** `pub fn put_file(&self, url: &str, file: &Path, sha256: &str, content_type: &str, cache_control: &str) -> Result<()>` — streams the file, signs with the real payload hash.

- [ ] Test first with the same `TcpListener` harness: the server receives method `PUT`, the exact body, `content-type`, `cache-control`, `x-amz-content-sha256 == sha256`, and an `authorization` header whose `SignedHeaders` include `cache-control;content-type`.
- [ ] Implement, run, clippy, commit "Let HttpFetch upload a file with a signed S3 PUT."

## Track C — gcce-recipe (one agent; long builds; no Rust code)

### Task C1: recover the unrecorded build facts

- [ ] Read `~/src/GCC4Symbian` (its `build-toolchain.sh`, any patch directory, `git log -1`, `git status`) and determine whether the GCC behind `~/gcc-builds/gcc-12.1.0` is plain GCC 12.1.0 or patched. Evidence to use: the script's steps, `~/src/` build directories, `strings`/`-v` output of the built binaries. Record what is proven and what is not.
- [ ] Recover binutils 2.29.1's configure line from `~/src/binutils-2.29.1-build/config.status` (or `config.log`).
- [ ] Write findings into `docs/research/wip/toolchain-manager.md` ("GCCE build facts") on branch `tm-gcce` (worktree `~/worktrees/symdev/tm-gcce`) and commit.

### Task C2: build from official tarballs into one relocatable prefix

- [ ] Download `gcc-12.1.0.tar.xz` and `binutils-2.29.1.tar.xz` from `https://ftp.gnu.org/gnu/` into `~/src/gcce-recipe/` (outside any git tree), verify the GNU `.sig` if `gpg` has the keys, otherwise record the SHA-256 you got.
- [ ] Write `~/projects/symdev-packages/recipes/gcce/12.1.0/build.sh` (bash, `set -euo pipefail`): binutils then GCC into one `--prefix` passed as `$1`, with exactly the observed configure flags (spec §6 and C1), any patches C1 proved necessary, `make -j"$(nproc)"`, `make install-strip` where supported. No flag that was not observed.
- [ ] Run it on this host into `~/src/gcce-recipe/prefix-a`; then copy the prefix to a different path `~/src/gcce-recipe/moved` to prove relocation.
- [ ] Record wall time and installed size; check `bin/` for symlinks/hard links (the packer stores internal symlinks; hard links become files).

### Task C3: byte-identical acceptance (spec §8 item 1)

- [ ] Build `examples/hello` and `examples/gui` twice from a clean `build/` each: once with the current env (`~/gcc-builds`), once with `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` pointing into `~/src/gcce-recipe/moved` (same `SYMDEV_EPOCROOT`). `cmp` every `build/*.exe` (and `.o` if differing, to localise). Also a Rust example from `symbian-rs/examples/hello` if its build uses GCCE.
- [ ] If not identical: find the cause (compare `-v` outputs, `cmp -l` the objects, `readelf`), fix the recipe, rebuild. Do not accept "close".

### Task C4: recipe file and experiment record

- [ ] Write `~/projects/symdev-packages/recipes/gcce/12.1.0/recipe.toml`:
```toml
id = "gcce;12.1.0"
license = "GPL-3.0-or-later"
host = "x86_64-linux"
build = "build.sh"
[[source]]
url = "https://ftp.gnu.org/gnu/gcc/gcc-12.1.0/gcc-12.1.0.tar.xz"
sha256 = "<the hash C2 recorded>"
[[source]]
url = "https://ftp.gnu.org/gnu/binutils/binutils-2.29.1.tar.xz"
sha256 = "<the hash C2 recorded>"
```
(`git init` `~/projects/symdev-packages` if it does not exist yet; local only, never pushed by an agent.)
- [ ] Write experiment **107** in `docs/research/experiment-backlog.md` on `tm-gcce`: question, the C1 facts, argv/flags, timings, sizes, the `cmp` result, the build-dependency apt list the Debian 11 workflow will need (observed from what this host needed, marked as such). Commit.

## Track F — ci-fmt (one agent)

### Task F1: make CI on `main` green again

- [ ] In `~/worktrees/symdev/ci-fmt` (branch `ci-fmt` from `main`): `cargo fmt --all`; confirm only `symbian-rs/crates/symbian-std/src/fs/file.rs` (or other formatting-only) changes; find why `cargo fmt --all` at the root formats a file of the excluded `symbian-rs` workspace (path dependency from `symdev-build`'s dev-deps) and note it in the commit message body.
- [ ] `cargo fmt --all --check`, clippy, test — clean. Commit "Format symbian-std's file module, which the root workspace's rustfmt check reaches through a path dependency."; fast-forward `main`; do not push (the lead pushes).

## Owner — O1 (Cloudflare; the lead sends the checklist from spec §10)

Result needed back: the `r2.dev` public URL of `symdev-public`, the account id, and four key values set by the owner in their own shell profile: `SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID`, `SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY` (reader), `PUBLISH_ACCESS_KEY_ID`, `PUBLISH_SECRET_ACCESS_KEY` (publisher). Keys never go into chat or git.

---

## Track D — integration (after A and B are merged into `toolchain-manager`)

### Task D1: `ToolchainOverrides` and `Toolchain::resolve`

**Files:** Modify `crates/symdev-build/src/toolchain.rs` (split into `toolchain.rs` + `toolchain/overrides.rs` if it passes 300 lines), `crates/symdev-build/Cargo.toml` (+ `symdev-sdk`).

**Produces:**
```rust
pub struct ToolchainOverrides { pub epocroot: Option<PathBuf>, pub gxx: Option<PathBuf>, pub ld: Option<PathBuf>,
    pub elf2e32: Option<PathBuf>, pub gcc_lib: Option<PathBuf>, pub gcc_target_lib: Option<PathBuf> }
impl ToolchainOverrides {
    pub fn from_env() -> ToolchainOverrides;   // the adapter, beside the existing from_env pattern
    pub fn needs_gcce(&self) -> bool;          // any of gxx/ld/gcc_lib/gcc_target_lib unset
    pub fn needs_sdk(&self) -> bool;           // epocroot unset
}
impl Toolchain {
    /// Field by field: override if set (error naming the variable if the path does not exist),
    /// else the layout. A field with neither -> Other naming the variable and the package to install.
    pub fn resolve(o: &ToolchainOverrides, gcce: Option<&Gcce>, sdk: Option<&PlatformSdk>) -> Result<Toolchain>;
}
impl Epocroot { pub fn resolve(o: &ToolchainOverrides, sdk: Option<&PlatformSdk>) -> Result<Epocroot>; }
```
Delete `Toolchain::from_env` / `Epocroot::from_env` if nothing else uses them after D3.

- [ ] Tests first: all overrides set → layouts unused; nothing set → all from layouts; **only `gxx` set → gxx from override, ld/libs from `Gcce`** (Review Focus 2); override path missing → error naming `SYMDEV_GXX` and the path; nothing set and no layout → error naming `SYMDEV_GXX` and `symdev sdk install gcce;12.1.0`.
- [ ] Implement, run, clippy, commit "Resolve the toolchain field by field from the environment, then from installed packages."

### Task D2: `SdkManager` (auto-install adapter)

**Files:** Create `crates/symdev-sdk/src/manager.rs`, `src/manager/tests.rs`, `src/builtin.rs`.

**Produces:**
```rust
// builtin.rs — set in R1 once the bucket exists; None disables the built-in source.
pub const BUILTIN_SOURCE: Option<&str> = None;
pub fn builtin_source() -> Option<SourceSpec>;     // name "public", auth None

pub struct SdkManager<'w> { home: SdkHome, sources: Sources, keys: BTreeMap<String, S3Keys>,
                            offline: bool, host: Host, progress: &'w mut dyn std::io::Write }
impl<'w> SdkManager<'w> {
    pub fn new(home: SdkHome, sources: Sources, keys: BTreeMap<String, S3Keys>, offline: bool,
               progress: &'w mut dyn std::io::Write) -> Result<Self>; // host = Host::current() or error
    /// Installs every id not installed (and its depends). No index is fetched if all are installed.
    pub fn ensure(&mut self, ids: &[PackageId]) -> Result<Vec<Receipt>>;
    pub fn available(&mut self) -> Result<Vec<(String, IndexPackage)>>;  // (source name, package)
    pub fn home(&self) -> &SdkHome;
}
```
Fetcher per source: `file://` → `FileFetch`; `auth = S3` with keys → `HttpFetch::new(name, Some(SigV4::s3(keys, "auto")))`; `auth = S3` without keys → source skipped and remembered; else `HttpFetch::new(name, None)`. Progress line: `installing {id} ({MB} MB) from {source}…`.

Errors (spec §5): offline + missing → `"{id} is not installed and --offline forbids downloading it; run `symdev sdk install {id}`"`; not found anywhere and a keyless s3 source was skipped → message names that source, both variable names, and "or set SYMDEV_EPOCROOT to your own SDK" when the id's kind is `sdk`; not found → names id and the searched source names; found but no archive for host → `"{id} has no archive for {host} in source {name}"`.

- [ ] Tests first (file:// sources, tempdirs, progress into a `Vec<u8>`): ensure installs and prints one line per package; second `ensure` fetches no index (a `file://` source whose `index.toml` is deleted after the first run still succeeds); offline error text; **keyless private source first + id present in a second file source → installed from the second, no error** (Review Focus 4); keyless private source and id nowhere → keys hint with both variable names and the SYMDEV_EPOCROOT hint; host-mismatch message (Review Focus 3); `depends` installed first.
- [ ] Implement, run, clippy, commit "Add SdkManager, which installs what a build is missing from the configured sources."

### Task D3: CLI — `symdev sdk`, `--offline`, auto-install in build, hermetic tests

**Files:** Modify `crates/symdev-cli/src/cli.rs`, `src/main.rs`, `src/build_cmd.rs`, `Cargo.toml` (+ `symdev-sdk`); create `src/sdk_cmd.rs`, `src/provision.rs`; modify `tests/common/mod.rs`, `tests/build.rs` and any test that asserted "missing toolchain"; create `tests/sdk.rs`.

**Produces:**
- `Cli { #[arg(long, global = true)] offline: bool, command }`; `Commands::Sdk { #[command(subcommand)] action: SdkAction }` with `List`, `Install { ids: Vec<String> }`, `Uninstall { ids: Vec<String> }` (min 1).
- `provision.rs`: `pub fn manager(offline: bool, progress: &mut dyn Write) -> Result<SdkManager>` — reads `SYMDEV_HOME` (default `$XDG_DATA_HOME/symdev` → `~/.local/share/symdev`), `$XDG_CACHE_HOME/symdev/downloads`, `$XDG_CONFIG_HOME/symdev/sources.toml`, and the key variables for every `s3` source; `pub fn toolchain(m: &Manifest, offline: bool) -> Result<Toolchain>` — `ToolchainOverrides::from_env()`, ensure `Pins::gcce()` if `needs_gcce`, `Pins::platform_sdk(device)` if `needs_sdk`, then `Toolchain::resolve`; `pub fn epocroot(device, offline) -> Result<Epocroot>` likewise for `epocroot_for`.
- `symdev sdk install` with no ids inside a project installs what `toolchain()` would; outside a project it is an error naming that.
- `symdev sdk list`: lines `installed  gcce;12.1.0  (public)` and `available  sdk;s60-3rd-fp2;1.1  (private)`.

- [ ] `tests/common/mod.rs`: `bin()` sets `SYMDEV_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME` to a process-wide `OnceLock<TempDir>` whose config has `symdev/sources.toml` = `builtin = false\n`.
- [ ] Tests first: `build` with no toolchain env and `--offline` → message with `symdev sdk install gcce;12.1.0`; `sdk install` against a `file://` source (written into the test's config) installs and `sdk list` shows it; `sdk uninstall` removes it; `build` with only the `file://` source installs both packages (fake packages: a `gcce` whose `bin/` holds stub scripts is enough to reach the compile step's error, assert the install lines on stderr); existing build tests keep passing with their explicit env.
- [ ] Update `README.md` "Requirements" and `examples/README.md`: packages install themselves; the variables are overrides; `symdev sdk` commands; private source setup.
- [ ] Workspace `cargo test`, clippy, fmt; commit "Add the symdev sdk commands and install missing toolchain packages before a build."

## Track E — publisher (after A and B merged; uses C4's recipe)

### Task E1: `publish` binary

**Files (in `~/projects/symdev-packages`):** `Cargo.toml` (workspace), `publish/Cargo.toml`, `publish/src/main.rs`, `publish/src/recipe.rs`, `publish/src/bucket.rs`, `publish/src/tests.rs`; `README.md`; `.gitignore` (`/target`, `*.tar.gz`).

`symdev-sdk = { path = "/home/genius/worktrees/symdev/toolchain-manager/crates/symdev-sdk" }` for now; R4 switches it to `git = "https://github.com/4akloon/symdev", tag = "…"`.

**Produces (CLI):**
```
publish private <id> --from <dir> --recipe <recipe.toml> [--dry-run]
publish public  <id> --from <prefix> --source-code <tar.gz> --recipe <recipe.toml> [--dry-run]
```
Env: `PUBLISH_PRIVATE_URL`, `PUBLISH_PUBLIC_URL` (S3 endpoint bases, e.g. `https://<acct>.r2.cloudflarestorage.com/symdev-private/`), `PUBLISH_ACCESS_KEY_ID`, `PUBLISH_SECRET_ACCESS_KEY`.
Flow: parse recipe (`id`, `license`, `host`, `include` for SDK, `sha256` optional for private) → `ReproducibleTarGz::pack` → private: compare with recipe `sha256` (absent → print it and stop: "record this in the recipe") → fetch `index.toml` (404 → `Index::empty()`) → `insert` (refuses existing) → `put_file` archive (`application/gzip`, `public, max-age=31536000, immutable`) → source-code archive for public → `put_file` index (`application/toml`, `no-cache`). `--dry-run` does everything but the PUTs and prints the index it would write.

- [ ] Tests first against a local `TcpListener` fake bucket (GET index 404, then PUTs recorded): order archive → index; refusal on existing id leaves the bucket untouched; private sha mismatch stops before any PUT; dry run makes no PUT.
- [ ] Implement, `cargo test`, clippy; `git commit` in that repo.

### Task E2: SDK recipe, verified against the real SDK (no upload)

- [ ] `recipes/sdk/s60-3rd-fp2/1.1/recipe.toml`: `id`, `license = "LicenseRef-Nokia-S60-SDK-EULA"`, `host = "any"`, and an `include` list of paths relative to the SDK root, where a `*` may appear only in the last segment:
```toml
include = [
  "epoc32/include",
  "epoc32/release/armv5/lib/*.dso",
  "epoc32/release/armv5/lib/usrt2_2.lib",
  "epoc32/release/armv5/urel/eexe.lib",
  "epoc32/release/armv5/urel/edll.lib",
  "epoc32/tools/variant/variant.cfg",
]
```
  `publish` copies the matching files into a staging tree (byte copies, same relative paths), then `ReproducibleTarGz::pack(staging, &["epoc32"], out)`. A pattern that matches nothing is an error naming it. The RVCT `.lib` import libraries beside the `.dso` files are deliberately left out (spec §2).
- [ ] Run `publish private 'sdk;s60-3rd-fp2;1.1' --from ~/sdk/S60_3rd_FP2 --recipe … --dry-run` twice → same sha both times; record `sha256` in the recipe; record archive size.
- [ ] Prove the subset is enough: extract the archive into a temp dir, point `SYMDEV_EPOCROOT` at it and build `examples/hello`, `examples/gui`, a Rust example, and a DLL project (find one among the repo's tests/fixtures; a DLL links `edll.lib`); `cmp` every output against a build with the full SDK. If a file is missing, extend `include` with the observed path only, and record it.
- [ ] Commit in the packages repo; note sizes in the wip file.

### Task E3: workflows (written, not pushed)

- [ ] `.github/workflows/build.yml` (PR, paths `recipes/**`): job in `container: debian:11`, installs the apt list from experiment 107, runs `recipes/gcce/12.1.0/build.sh /opt/gcce`, then `publish public … --dry-run` (needs only the public read URL), uploads the packed archive as an Actions artifact.
- [ ] `.github/workflows/publish.yml` (push to `main`, paths `recipes/gcce/**`, `workflow_dispatch`): `environment: publish`, `concurrency: publish`, same build, then `publish public 'gcce;12.1.0' --from /opt/gcce --source-code <tarballs+recipe packed> --recipe …`.
- [ ] `actionlint` if available, else `python3 -c 'import yaml…'` parse check; commit.

## Track G — CI examples job (after D)

### Task G1

**Files:** Modify `.github/workflows/ci.yml`.

- [ ] Job `examples` on `ubuntu-24.04`: step `has-key` checks `secrets.SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID != ''` through an env var and sets an output; every later step `if: steps.has-key.outputs.ok == 'true'`; installs Rust 1.98.1 and the `symbian-rs` nightly; `actions/cache` for `~/.local/share/symdev` keyed `symdev-sdk-${{ hashFiles('crates/symdev-sdk/src/pins.rs') }}`; writes `~/.config/symdev/sources.toml` with the private source (`url` from a repository variable `SYMDEV_PRIVATE_SOURCE_URL`); `cargo build --release -p symdev-cli`; `symdev build` + `symdev package` in `examples/hello`, `examples/gui`, `symbian-rs/examples/hello`; `SYMDEV_SIGN_PASSWORD: ci-throwaway`; uploads the `.sisx` files as an artifact (R5 runs them in EKA2L1).
- [ ] YAML parse check; commit "Build and package the examples in CI through the toolchain manager."

## Wave 3 — release (lead, with the owner)

### Task R1: built-in source
- [ ] Set `BUILTIN_SOURCE` to the owner's `https://pub-….r2.dev/` URL; test that `builtin_source()` is `Some`; commit.

### Task R2: publish the SDK
- [ ] With the owner's publisher key in the environment: `publish private 'sdk;s60-3rd-fp2;1.1' …` for real; then `symdev sdk install 'sdk;s60-3rd-fp2;1.1'` on this host with only the reader key → receipt, files.

### Task R3: publish GCCE
- [ ] Ask the owner before creating the public GitHub repository `4akloon/symdev-packages` and pushing; owner adds the `publish` environment and secrets; the publish workflow runs; check the index on r2.dev lists `gcce;12.1.0` and its source archive.

### Task R4: merge and tag
- [ ] Whole-branch review (superpowers:requesting-code-review); merge `toolchain-manager` into `main`; push after the owner's go; tag; switch the packages repo's `symdev-sdk` dependency to that tag.

### Task R5: acceptance (spec §8), each by running it
- [ ] 1: C3's `cmp` repeated with the **published** `gcce;12.1.0` installed by `symdev sdk install`.
- [ ] 2: fresh `HOME` on this host (`env -i HOME=$(mktemp -d) PATH=/usr/bin:/bin`, install rustup there, `cargo install --git … symdev-cli`, reader key only) builds `hello` and `gui`; plus the CI `examples` job (a clean ubuntu-24.04). Run both `.sisx` (local and CI artifact) in EKA2L1 with the eka2l1-host skill's PID-bound screenshot; look at the window. A container run needs docker/podman, absent here (installing them needs `sudo` — owner's call).
- [ ] 3: `examples` job green on `main`.
- [ ] 4: the owner's current `SYMDEV_*` environment builds `hello`/`gui` byte-identically to before.
- [ ] Move durable facts from the wip note into the spec / experiment 107; delete the wip note; commit.
