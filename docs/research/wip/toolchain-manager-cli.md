# WIP: toolchain manager, Track D (integration) — branch `tm-cli`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks D1–D3.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md` (§3–§5, §8, §9).
Inputs: `toolchain-manager-core.md` ("For Track D"), `toolchain-manager-net.md`.

## Status

| Task | State | Commit |
|---|---|---|
| D1 ToolchainOverrides / Toolchain::resolve | done | see `git log` |
| D2 SdkManager / builtin | done | see `git log` |
| D3 CLI sdk / --offline / provision / hermetic tests / docs | done | see `git log` |
| smoke test (real toolchain, file:// source) | done (2026-10-02), see "Smoke test" | |

## Facts

- Before D: `Toolchain::from_env` (symdev-build) is called only by `build_cmd.rs`;
  `Epocroot::from_env` only by `main.rs::epocroot_for` (used by `freeze` and `package`).
- `Toolchain::ar()` read `SYMDEV_AR` from the environment inside the value type.
- CLI tests `build_valid_manifest_no_bld_inf` / `build_does_not_require_external_elf2e32`
  pass fake paths (`/sdk`, `/gcc/g++`) that do not exist; spec §5 makes a missing
  `SYMDEV_*` path an error, so they must use real temp paths.
- `freeze` loads no `symdev.toml`; the SDK pin needs the device from it.

- Smoke-test input (2026-10-02): `~/gcc-builds/gcc-12.1.0` carries its **own binutils
  2.35** (`bin/arm-none-symbianelf-{as,ld,ar,…}`, hard-linked to
  `arm-none-symbianelf/bin/{as,ld,…}`, and `arm-none-symbianelf/lib/ldscripts`), while
  `~/gcc-builds/binutils-2.29.1` is a separate 2.29.1. `g++ -print-prog-name=as` →
  `<gcc prefix>/arm-none-symbianelf/bin/as`. So the classic env assembles with **as
  2.35** (gcc's own) and links/archives with **ld/ar 2.29.1** (`SYMDEV_LD`, `ar` beside
  it). Overlaying binutils 2.29.1 onto the gcc prefix replaces `as` and the ldscripts
  too. Input for the GCCE recipe (Track C).

## Decisions

- Install commands in messages quote the id (`symdev sdk install 'gcce;12.1.0'`): `;`
  is a shell command separator, so the unquoted form from the plan/spec runs
  `symdev sdk install gcce` and then `12.1.0`.

- D1: `ToolchainOverrides` also carries `SYMDEV_AR` (spec §3 lists it as `ld`'s
  override); `Toolchain` gained `ar: Option<PathBuf>`, so `Toolchain::ar()` no longer
  reads the environment. Every set override is checked to exist, `SYMDEV_ELF2E32` and
  `SYMDEV_AR` too (spec §5: "a `SYMDEV_*` path does not exist").
- D1: the "no package" message for the EPOCROOT says `symdev sdk install` (no id) in the
  project: `Toolchain` does not know the device, and the no-id form installs the
  project's SDK. The GCCE one names `Pins::gcce()`.
- D1: `Toolchain::from_env` / `Epocroot::from_env` stay until D3 moves their callers.

- D2: the source lookup lives in a crate-private `Catalog` (`catalog.rs`) so
  `manager.rs` stays small: fetcher per source, indexes fetched lazily and kept per
  source name, `find` (first source that lists the id decides; no archive for the host
  there is the error `{id} has no archive for {host} in source `{name}``, not a fall
  through to later sources), `all` for `available`.
- D2: a source whose index cannot be read (unreachable, 403, bad index, newer schema)
  is skipped like a keyless one; its reason is appended to the not-found error only.
- D2: not-found text: `{id} was not found in the sources searched: `a`, `b`` then
  `; source `x` was skipped because its keys are not set: set A and B[, or set
  SYMDEV_EPOCROOT to your own SDK]` (the SDK hint only for `sdk;…` ids, as the plan
  says) and `; source `y` could not be read: <error>`. No sources at all → names
  `$XDG_CONFIG_HOME/symdev/sources.toml`.
- D2: offline error lists every missing requested id in one command:
  `A and B are not installed and --offline forbids downloading them; run `symdev sdk
  install 'A' 'B'``; one id gives the plan's text (quoted id).
- D2: `available` filters to packages with an archive for this host, first source
  wins per id, and writes `warning: …` lines (keyless / unreadable sources) to the
  progress writer; offline it is an error.
- D2: progress MB = bytes / 10^6 with one decimal (`installing gcce;12.1.0 (58.3 MB)
  from public…`); a failed progress write is ignored (informational only).
- D2: host-mismatch is only reachable in a unit test (manager's `host` set to `Any`):
  on x86_64-linux every archive is either exact or `any`.
- D2: tests never resolve a name: the keyed `s3` test source is `https://127.0.0.1:1/`.

- D3: `Provision` (`crates/symdev-cli/src/provision.rs`) is a type, not loose fns (the
  plan's `manager`/`toolchain`/`epocroot`): it holds `--offline` and an env lookup, so
  its path and key rules are unit-tested with a map. Paths: `SYMDEV_HOME` (must be
  absolute) else `$XDG_DATA_HOME/symdev`; cache `$XDG_CACHE_HOME/symdev/downloads`;
  sources `$XDG_CONFIG_HOME/symdev/sources.toml`; relative XDG values ignored (XDG
  spec); fallbacks under `$HOME`. One key variable without the other is an error.
- D3: set `SYMDEV_*` paths are checked (`ToolchainOverrides::check`) before anything is
  downloaded, so a typo does not cost a 58 MB download first.
- D3: `symdev package` installs nothing (spec §4): for a `bld.inf` project it takes the
  SDK only from `SYMDEV_EPOCROOT` or an installed package, else an error naming
  `symdev build` / `symdev sdk install 'sdk;…'` / `SYMDEV_EPOCROOT`. `symdev freeze`
  does auto-install (it reads `bld.inf`), and loads `symdev.toml` for the device only
  when `SYMDEV_EPOCROOT` is unset.
- D3: `main.rs` dispatch collapsed into one `result` + one error print (it would have
  passed 300 lines with the `sdk` arm); `manifest()` keeps the `invalid manifest:`
  prefix the tests check.
- D3: `symdev sdk install` prints `installed  <id>  (<source>)` per id on stdout (the
  list format); `uninstall` prints `removed  <id>`, and only warns for an id that is
  not installed. `sdk list --offline` lists only the installed packages.
- D3: hermetic CLI tests: `common::bin()` points `SYMDEV_HOME`/`XDG_*` at one
  `tempfile` dir per test process inside `CARGO_TARGET_TMPDIR` (a static `TempDir`
  is never dropped, so it stays in `target/tmp`, not `/tmp`), with `builtin = false`.
  `common::repo::World` gives a test its own home and a `file://` source.
- D3: the help test now compares the exact command list (it forbade the substring
  `sdk`, and `toolchain`, which help text now contains).

## Smoke test (2026-10-02, scratchpad only, nothing committed)

- Prefix: copy of `~/gcc-builds/gcc-12.1.0` with a copy of `~/gcc-builds/binutils-2.29.1`
  overlaid (so `as`, `ld`, `ar` and the ldscripts are all 2.29.1). gcc found `as`/`ld`
  in the relocated prefix by itself (`-print-prog-name=as` →
  `<prefix>/bin/../lib/gcc/arm-none-symbianelf/12.1.0/../../../../arm-none-symbianelf/bin/as`);
  nothing extra was needed. Packed with `ReproducibleTarGz` (`.`): 72 482 595 bytes,
  7.9 s.
- SDK: `epoc32/include`, `epoc32/release/armv5/lib/*.dso` (570), `epoc32/release/armv5/
  urel/{eexe,edll,usrt2_2}.lib`, `epoc32/tools/variant/variant.cfg`: 2 697 files, 31 MB
  staged, 4 941 155 bytes packed. **`usrt2_2.lib` is in `urel/`, not `lib/`** as spec §2
  and plan E2's `include` list say (`~/sdk/S60_3rd_FP2/epoc32/release/armv5/lib/
  usrt2_2.lib` does not exist; the link line takes it from `-L…/urel`). Input for E2.
- `symdev build` in a copy of `examples/hello` with no `SYMDEV_*` toolchain variable and
  only a `file://` source: printed `installing gcce;12.1.0 (72.5 MB) from local…` and
  `installing sdk;s60-3rd-fp2;1.1 (4.9 MB) from local…`, wrote both receipts, produced
  `build/hello.exe` (3 588 bytes), exit 0, 3.2 s. A second `symdev build --offline`
  printed no install line; `symdev package` (installs nothing) produced `hello.sisx`.
- Same copy, clean `build/`, classic env (all five variables): no package installed (the
  empty `SYMDEV_HOME` was not even created). `cmp`: `hello.elf` **identical**;
  `hello.exe` differs in 7 bytes, all inside `iHeaderCrc` (0x14) and `iTimeLo` (0x24):
  equal with those two masked. Two classic builds differ in the same bytes: the native
  elf2e32 stamps `SystemTime::now()` (`crates/symdev-elf2e32/src/elf2e32.rs:166`), so no
  two `.exe` builds are byte-identical. Spec §8 item 1 ("`.exe` byte-identical") has to
  mask the time and header CRC, or the comparison must be on `.elf`.
- `hello.o` differs (section contents equal per `objdump -s`; `.text` at 0x44 vs 0x48):
  the package's `as` is 2.29.1, the classic env's is gcc's own 2.35. The linked ELF is
  the same.

## Dead ends

## Next step

Review fixes done on `tm-review-fixes` (see "Review fixes"); waiting for the lead to merge.

## Review fixes (branch `tm-review-fixes`, 2026-10-02)

Independent review of the toolchain manager (line numbers from 34987b3). Each finding was
checked against the code before changing it; baseline `cargo test --workspace --offline`:
652 passed. State per finding (commits: `git log`):

| # | Finding | Verified | State |
|---|---|---|---|
| C1 | CI caches the SDK | yes: `path: ~/.local/share/symdev` holds `sdk/s60-3rd-fp2/1.1`; `main` has no examples job yet, so no SDK cache was ever saved in the default branch's scope | fixed: only `~/.local/share/symdev/gcce` is cached (key `symdev-gcce-<pins.rs>`); spec §7 says why |
| M10 | reader keys are job-level env | yes: `cargo build` (third-party build scripts) ran with them | fixed: keys only on the step "Install the toolchain packages", which runs `symdev sdk install` (no ids) in each example; build and package run without them |
| I3 | workflow `RUSTFLAGS` overrides libcalls' `build.rustflags` | yes, and an empty `RUSTFLAGS` overrides it too (fact below), so the suggested `RUSTFLAGS: ""` would not fix it | fixed differently: `RUSTFLAGS: -D warnings` moved from the workflow to the `check` job; the header comment no longer speaks of "missing toolchain" tests |
| 1 | `epocroot`/`installed_epocroot` check every `SYMDEV_*` path | yes: both call `overrides()` → `check()` of all 7 | fixed: both take the unchecked overrides; `Epocroot::resolve` checks `SYMDEV_EPOCROOT` itself (unit + CLI tests for package and freeze) |
| 2 | `available` vs `find` disagree when the first source lacks this host's archive | yes: the `&&` short-circuits before `seen.insert` | fixed: `seen.insert` first, so the first source that lists an id decides for `available` as for `find` |
| 3 | `keys_make_an_s3_source_searchable` makes a real HTTPS request | yes: `ensure` → `find` → ureq GET `https://127.0.0.1:1/…` (proxy from env) | fixed: replaced by `catalog::tests::keys_give_an_s3_source_a_fetcher` (no request; a mutant giving a keyless `s3` source a fetcher fails it). Not fixed, reported: `http_fetch` tests GET `http://127.0.0.1:<port>` and ureq takes the proxy from the environment, so with `HTTP_PROXY` set (and no `NO_PROXY` for 127.0.0.1) they go to the proxy |
| 4 | `Provision` reads the toolchain variables past its own lookup | yes: `ToolchainOverrides::from_env()` in `overrides()`/`needed()` | fixed: `ToolchainOverrides::from_lookup` is public, `from_env` deleted (no caller left), `Provision::overrides(&self)` |
| 5 | an all-installed build fails on download-only problems | yes: `manager()` (sources, keys, host) runs before `ensure` checks receipts | fixed: `Provision::install_missing` reads receipts first, builds the manager only for missing ids (CLI test: malformed `sources.toml`, half key pair; the host check sits in `SdkManager::new`, so it is skipped the same way) |
| 6 | a Rust build downloads before `RustSdk::from_env` can fail | yes: `toolchain()` at the top, `RustSdk::from_env()` in the match | fixed: `RustSdk` resolved first for a Rust project (CLI test: stale `SYMDEV_RUST_SDK`, nothing installed) |
| 7 | `installing …` printed before `install` re-checks under the lock | yes: `writeln!` precedes `home.install`, which returns early if installed | fixed: `SdkHome::install` takes `starting: impl FnOnce()`, run under the lock after the re-check; the manager prints from it (SdkHome prints nothing). Tests moved: `home/tests/receipts.rs` (300-line rule) |
| 8 | without `HOME` the message names `SYMDEV_HOME` for every path | yes: one text for data, cache and config | fixed: packages → `SYMDEV_HOME or XDG_DATA_HOME`, cache → `XDG_CACHE_HOME`, config → `XDG_CONFIG_HOME` |
| 9 | `bin()` keeps keys/toolchain vars; temp homes pile up in `target/tmp` | yes: 10+ `symdev-cli-home-*` after two runs | fixed: `bin()` removes every inherited `SYMDEV_*`/`PUBLISH_*` (tests set what they need after it, which wins); `bin_without_toolchain` and the per-test `env_remove`s deleted. Homes: `target/tmp/symdev-cli-homes/home-*`, one per process, `.lock` held for the process life; the first `bin()` removes every home whose lock it can take (made under a dot name, renamed once locked, so a sweep never takes one being made). `tests/hermetic.rs` checks both (mutants caught); after a full run one home is left, swept by the next. Supersedes the D3 note on the static `TempDir` |
| 10 | `sdk;…` in no source and no private source: no way out named | yes | fixed: `Sources.file` keeps the real `sources.toml` path; an `sdk` id found nowhere without a keyless-source hint adds "set SYMDEV_EPOCROOT to your own SDK, or add a source that has it in <path>"; no sources at all names the path too (was a hard-coded `$XDG_CONFIG_HOME/…`). Spec §5, §8 (prerequisites + 4-line TOML, acceptance item 2), README, examples README |
| 11 | host-mismatch test wording | nit, kept as is | — |

Checked after the last fix (2026-10-02): `cargo test --workspace --offline` 665 passed, 0
failed (652 before: +13 tests, one network test replaced); `cargo clippy --workspace
--all-targets --offline` no warning; `cargo fmt --all --check` clean; every `.rs` file ≤ 300
lines. By hand with the built binary under `env -i`: no `HOME` names `XDG_CONFIG_HOME` for
`sources.toml`; `sdk install 'sdk;s60-3rd-fp2;1.1'` with no source names `SYMDEV_EPOCROOT`
and the real `sources.toml` path, and creates nothing.

Left for the lead: the `http_fetch` tests' proxy exposure (row 3); `bin()` strips every
`SYMDEV_*`, not only the toolchain and key variables (a test sets what it needs after it).

## Rust SDK package (branch `tm-rust-sdk`, 2026-10-02)

Spec §12, symdev side, plus the `http_fetch` proxy exposure left above (row 3).

| Task | State |
|---|---|
| 1 `Pins::rust_sdk`, `RustSdkPackage` | done |
| 2 resolution order in `Provision` (env → checkout → package) | done |
| 3 scaffold: how a new project names the SDK; proposal | done (proposal only) |
| 4 `http_fetch` tests immune to `HTTP_PROXY`/`ALL_PROXY` | done |
| 5 README / examples README: the prebuilt route | done |

Facts (before the change, commit 553fb0f):

- `RustSdk::from_env` (symdev-build) read `SYMDEV_RUST_SDK`, else the compile-time
  `CARGO_MANIFEST_DIR/../../symbian-rs`; callers: `build_cmd.rs` (before the toolchain,
  review fix 6), `scaffold_rust.rs`, and tests in `rust_sdk.rs`, `driver/tests/rust_build.rs`,
  `scaffold_rust.rs`. `RustSdk::at` checks only `targets/arm-symbian-e32.json`.
- ureq 3.4.2: `Agent::config_builder()` starts from `Config::default()`, whose `proxy` is
  `Proxy::try_from_env()` (`ALL_PROXY`, `HTTPS_PROXY`, `HTTP_PROXY`, lower-case too, and
  `NO_PROXY`); `ConfigBuilder::proxy(None)` turns it off.

Decisions:

- 1: `Pins::rust_sdk()` = `rust-sdk;` + symdev-sdk's `CARGO_PKG_VERSION`, which is the
  workspace version (`version.workspace = true`); its test reads `[workspace.package]
  version` from the workspace `Cargo.toml`. `RustSdkPackage::at(root, id)` refuses an id
  that is not exactly `rust-sdk;<version>` and checks `RustSdkPackage::REQUIRED`
  (`targets/arm-symbian-e32.json`, what `RustSdk::at` requires); a test in `rust_sdk.rs`
  keeps the two equal (both mutants — `at` requiring one more file, the list naming one
  more — fail it).
- 2: `Provision::rust_sdk()` (`crates/symdev-cli/src/provision/rust_sdk.rs`, an `impl
  Provision` beside `provision.rs`, which would pass 300 lines) resolves, in order:
  `SYMDEV_RUST_SDK` (through the injectable lookup; set but not a Rust SDK → `SYMDEV_RUST_SDK
  is set, but <why>; point it at a symbian-rs directory, or unset it to use the rust-sdk;…
  package`, nothing installed, no fall-through); the checkout (`RustSdk::CHECKOUT`, the
  compile-time `CARGO_MANIFEST_DIR/../../symbian-rs`, held by `Provision` as
  `checkout: Option<PathBuf>` so tests inject it) if `RustSdk::at` accepts it — gone or
  not an SDK falls through silently; else `install_missing([Pins::rust_sdk()])` (the GCCE
  path: receipts first, sources/keys/host only for a missing id, `--offline`) then
  `RustSdkPackage::at` → `RustSdk::at`. An install failure gets `; or set SYMDEV_RUST_SDK
  to a symbian-rs directory` appended (the catalog's own hint covers only `sdk;…`).
- 2: `RustSdk::from_env` deleted; `RustSdk::at` errors no longer name the variable (the
  caller adds it); `RustSdk::root()` added. Callers: `build_cmd` (still before the
  toolchain, so `rust-sdk` installs before GCCE), `scaffold::create_project(…, rust_sdk:
  impl FnOnce() -> Result<RustSdk>)` (asked only for a Rust project, after the
  exists/template checks), `Provision::needed(device, language)` (`rust-sdk` first, for a
  Rust project with neither variable nor checkout) used by `symdev sdk install` with no ids.
- 2: CLI test without a checkout: the built `symdev` cannot be told its checkout is gone
  without a production knob, so `tests/common/prebuilt.rs` copies the binary with the bytes
  of `RustSdk::CHECKOUT` replaced by a same-length missing path (`…/symbian-xx`) — what a
  binary built on another machine looks like here. It waits out `ETXTBSY` (a child forked by
  another test thread while the copy was open). Mutants caught: `from_env` without the
  checkout, `build_cmd` bypassing `Provision`, `needed` without `rust-sdk`, the missing
  variable hint.

### Scaffold and a changing SDK version (task 3, 2026-10-02)

What `symdev new <name> --lang rust` writes (`scaffold_rust.rs`; checked by
`tests/rust_sdk.rs` and by hand in the scratchpad): **absolute, canonical paths of the SDK it
resolved** — `symbian-core = { path = "<sdk>/crates/symbian-core" }` and the same for
`symbian-std` in `Cargo.toml`, `build.target = "<sdk>/targets/arm-symbian-e32.json"` in
`.cargo/config.toml` — plus verbatim copies of the SDK's `rust-toolchain.toml` and the hello
`src/main.rs` (compiled into symdev with `include_str!`). From a prebuilt symdev `<sdk>` is
the package directory, `$SYMDEV_HOME/rust-sdk/0.1.0`.

After an upgrade to a symdev that pins `rust-sdk;0.2.0`:

- `symdev build` resolves 0.2.0 and passes `--target <0.2.0>/targets/…json` on cargo's command
  line (it overrides the config's 0.1.0 one), builds `symbian-libcalls` with
  `--manifest-path <0.2.0>/…` and compiles the C++ shims from `<0.2.0>/shims` — but cargo
  compiles the application against `symbian-std`/`symbian-core` from `<0.1.0>`, which stays
  installed (no `update`; an old id stays until uninstalled). Two SDK versions end up in one
  binary without any error, although the shim and the crates are one ABI ("the SDK and not
  the project decides what the shim calls").
- After `symdev sdk uninstall 'rust-sdk;0.1.0'` cargo stops: reproduced by renaming
  `rust-sdk/0.1.0` to `0.2.0` under a scaffolded project — `no matching package named
  symbian-core found / location searched: …/rust-sdk/0.1.0/crates/symbian-core`.
- `rust-toolchain.toml` keeps 0.1.0's nightly; a hand `cargo build` keeps 0.1.0's target.
- The checkout route has the same weakness when the checkout moves or is deleted.

**Proposal (not implemented):** a project names the SDK through one project-local,
unversioned path that symdev keeps pointed at the SDK it resolved, instead of an absolute
one. `symdev new` and every `symdev build` make `build/rust-sdk` a symlink to the SDK of the
running symdev (variable, checkout or package); the scaffold writes
`path = "build/rust-sdk/crates/symbian-std"` (etc.) and `build.target =
"build/rust-sdk/targets/arm-symbian-e32.json"`. Every build then takes the crates, the target,
the shims and libcalls from one SDK, an upgrade needs no edit, and `build/` (already ignored
by git, already holding symlinks into the EPOCROOT) is the right place for a host path. A
fresh clone needs one `symdev build` (or `symdev sdk install`) before a hand `cargo`. Open
points, each an experiment before code: whether cargo resolves a relative `build.target` in
`.cargo/config.toml` against the config's parent (cargo's config-relative rule) on the pinned
nightly; and `rust-toolchain.toml` — `symdev build` should compare the project's channel with
the SDK's and stop with the fix (copy the SDK's file) when they differ, since rustup reads the
file from the project and it cannot point elsewhere. Existing projects would need their three
paths rewritten once (a `symdev` message can name them).

### The `symbian-rs` tree alone is not a Rust SDK (found 2026-10-02)

`symbian-rs/crates/symbian-macros/Cargo.toml` has `symdev-locale = { path =
"../../../crates/symdev-locale" }` — a host crate **outside** `symbian-rs` — and
`crates/symdev-locale/Cargo.toml` inherits `version`/`edition`/`license`/`repository` from the
root `Cargo.toml`'s `[workspace.package]`. Every project on `symbian-std` reaches it
(`symbian-std` → `symbian-macros`). Reproduced with a scaffolded project and `cargo metadata
--offline`:

- SDK = a bare `symbian-rs` tree (what spec §12 and the packages track's note say the
  `rust-sdk` package holds): `no matching package named symdev-locale found / location
  searched: …/rust-sdk/crates/symdev-locale` — i.e. **outside** the versioned package dir.
- `symbian-rs/` + `crates/symdev-locale/`: `error inheriting edition from workspace root
  manifest … failed to find a workspace root`.
- `Cargo.toml` (root) + `crates/symdev-locale/` + `symbian-rs/`: resolves. (Cargo does not
  load the root's other members for a path dependency's inheritance.)

So the package must keep the repository's relative layout — pack the tag with `Cargo.toml`,
`crates/symdev-locale` and `symbian-rs` — and the SDK root is `<package>/symbian-rs`.
Alternatives that change the repository instead (the owner's or lead's call): give
`symdev-locale` its own `version`/`edition` and move it under `symbian-rs/`, or stop
`symbian-macros` depending on a host crate.

Decision (symdev side, so the package route can build at all; the lead aligns the packages
track and spec §2/§12): the `rust-sdk` package keeps the repository's layout — `Cargo.toml`,
`crates/symdev-locale/`, `symbian-rs/` — and its SDK is `<package>/symbian-rs`
(`RustSdkPackage::SDK_DIR`, `RustSdkPackage::symbian_rs()`). `RustSdkPackage::REQUIRED` =
`symbian-rs/targets/arm-symbian-e32.json`, `crates/symdev-locale/Cargo.toml`, `Cargo.toml`;
`RustSdk::REQUIRED` = the same seen from the SDK root (`targets/…`, `../crates/symdev-locale/
Cargo.toml`, `../Cargo.toml`), so `SYMDEV_RUST_SDK` pointing at a bare copy of `symbian-rs` is
refused up front ("it is the symbian-rs directory of a symdev checkout or of the rust-sdk
package") instead of failing inside cargo. The packer's include list follows:
`["Cargo.toml", "crates/symdev-locale", "symbian-rs"]` (`corpus/` may be left out: no build
reads it).

### `http_fetch` and the proxy variables (task 4, review row 3)

- Reproduced: `ALL_PROXY=http://127.0.0.1:9 cargo test -p symdev-sdk --lib http_fetch` → 8 of
  11 tests failed (every one that expects a response; the three error tests passed by
  accident).
- Fix: `HttpFetch::new` = `with_proxy(…, Proxy::try_from_env())` — the value
  `Config::default()` gave it before, so production is unchanged; `#[cfg(test)]
  HttpFetch::direct` = `with_proxy(…, None)`; every test in `http_fetch/tests.rs` uses
  `direct`. Same run with `ALL_PROXY`/`HTTP_PROXY` set: 12 of 12 pass.
- Test `http_fetch::tests::proxy::only_production_goes_through_the_proxy_variables`: the
  test re-runs its own binary for that one test with every proxy variable pointing at a
  one-connection proxy in the parent (it accepts ureq's `CONNECT` and answers the tunnelled
  GET itself); the child checks that `new` gets the proxy's body and `direct` the server's,
  the parent that the child ran one test and that the proxy saw `CONNECT 127.0.0.1:…`. The
  parent's environment is never changed (`set_var` would race the parallel tests). RED
  seen with `direct` = `new` (the child's `direct` hit the spent proxy: connection refused).
  ureq 3.4.2 tunnels plain-`http` requests through an HTTP proxy with `CONNECT` too.

### Docs (task 5) and what they found

- README: Requirements gains a "Rust SDK" row (variable → checkout → `rust-sdk` package) and
  the nightly row says Rust projects only; "Toolchain packages" names the Rust SDK and the
  order; Quick start has the prebuilt route (`curl -fsSL
  https://pub-15670d2771364287b9982e497c29f586.r2.dev/install.sh | sh`, "from the first
  release on") and keeps the from-source one. examples/README: how to get `symdev`, C++
  needs no Rust, Rust needs rustup and gets `rust-sdk`.
- `symbian-rs/examples/*` are members of the `symbian-rs` workspace (`version.workspace`,
  `path = "../../crates/…"`): a copy outside the clone does not build, and inside the clone
  a prebuilt `symdev` would compile the clone's crates against the package's shims and
  libcalls (the mix of task 3). So spec §8 item 5's "the same works for
  `symbian-rs/examples/hello` through the rust-sdk package" can only mean `symdev new hello
  --lang rust` (same `src/main.rs` — `RustSdk::HELLO_MAIN` is that file — same UID3), and
  the examples README says to build the in-tree examples with a symdev from the same clone
  or `SYMDEV_RUST_SDK=<clone>/symbian-rs`. For the lead: reword item 5.

### Real builds (2026-10-02, scratchpad only, toolchain from the five `SYMDEV_*` variables)

- A literal copy of `symbian-rs/examples/hello` outside the clone does not build, before or
  after this branch: it inherits `version`/`edition` from the `symbian-rs` workspace and its
  nightly from `symbian-rs/rust-toolchain.toml` (cargo: "`-Z` flag is only accepted on the
  nightly channel"). The SDK resolution itself was right (`--target <checkout>/targets/…`).
- Dev route: `symdev new hello --lang rust` (its `src/main.rs` and `symdev.toml` are identical
  to the example's) then `symdev build` with `SYMDEV_RUST_SDK` unset and an empty
  `SYMDEV_HOME`: SDK = the checkout, nothing installed (the home was never created),
  `build/hello.exe` 968 bytes, 19 s. The in-tree example built by the same binary gives the
  same `.exe` except the E32 header CRC and time (0x14–0x17, 0x24–0x27); the `.elf`s differ
  only in `.strtab` (47 bytes; symbol hashes of a crate built from another path).
- Package route: a `file://` source holding `rust-sdk;0.1.0` packed from the commit's
  `Cargo.toml`, `crates/symdev-locale`, `symbian-rs` (no `corpus/`; 377 466 bytes), and a
  copy of the debug `symdev` with its checkout bytes patched (one occurrence) — `symdev new`
  printed `installing rust-sdk;0.1.0 (0.4 MB) from local…` and wrote
  `…/home/rust-sdk/0.1.0/symbian-rs/crates/…` into `Cargo.toml`; `symdev build` produced
  `hello.exe` equal to the dev route's except the same eight header bytes; `symdev package`
  → `hello.sisx` (2 304 bytes).

### Checked at the end (2026-10-02)

`cargo test --workspace --offline`: 690 passed, 0 failed (665 recorded before this branch: +4
`RustSdkPackage`, +1 pin, +3 `RustSdk`, +9 `Provision::rust_sdk`, +7 CLI `tests/rust_sdk.rs`, +1
proxy). `cargo clippy --workspace --all-targets --offline`: no warning. `cargo fmt --all
--check`: clean. Every tracked `.rs` file ≤ 300 lines. `RustSdk::root()` was dropped again
(only tests read it; they use `target_spec()`). The prebuilt copy is an 88 MB file per run of
the `rust_sdk` test binary, in that process's test home, swept like the homes.

## Next step (Rust SDK package)

For the lead: (1) align the packages track's `rust-sdk` archive with the repository layout
above (`Cargo.toml`, `crates/symdev-locale`, `symbian-rs`), or decide to move
`symdev-locale`; spec §2/§12 say "the symbian-rs tree"; (2) reword spec §8 item 5 (in-tree
Rust examples cannot be built outside the clone; use `symdev new hello --lang rust`); (3) the
task-3 proposal (`build/rust-sdk` link) if wanted. Nothing pushed or merged.


## Whole-branch review fixes (branch `tm-final-fixes`, 2026-10-02)

Whole-branch review of the toolchain manager (reviewer's line numbers from 669dae1; branch
from `toolchain-manager` at 5c90f9c). Each finding is checked against the code before it is
changed; every behaviour fix starts with a failing test. Baseline `cargo test --workspace
--offline`: 690 passed, 0 failed.

| # | Finding | Verified | State |
|---|---|---|---|
| M4 | two types per file; free `pub fn`s | yes: `index_package.rs` (`ArchiveEntry`, `IndexPackage`), `source.rs` (`Auth`, `SourceSpec`); `resolve_url`, `builtin_source` | fixed: `archive_entry.rs`, `auth.rs`; `SourceSpec::resolve(&self, relative)`, `SourceSpec::builtin()` (the URL is a private const of `source.rs`), crate-private `SourceSpec::relative_problem` for `Index`; `url.rs`, `builtin.rs` and the `BUILTIN_SOURCE` export deleted. Crate-root re-exports of `Auth`, `ArchiveEntry`, `IndexPackage` unchanged |
| I1 | `sdk uninstall` deletes whatever a valid id points at | yes: `uninstall` did `remove_dir_all(package_dir(id))` with no receipt check; `install` did `remove_dir_if_exists` on a receipt-less dir, wiping packages below it | fixed: `home/placement.rs` — a dir is a package iff it holds a receipt file; `uninstall` refuses an id inside a package or a receipt-less dir that holds one (error names the package and its uninstall command), and still finishes an unfinished package; `install` checks placement under the lock before announcing or downloading. Tests: 5 unit (partial id, inside, install inside, install above, unfinished) + 1 CLI; RED seen for the 4 refusals and the CLI test |
| M3 | an archive can bring its own receipt | yes: a root `.symdev-package.toml` was visible from the rename until `Receipt::write` replaced it, and a root `.symdev-package.toml.partial/` made the write fail after the rename, leaving the archive's receipt in place (test RED: Io error, package dir left) | fixed: the receipt is written into the staging dir, then renamed into place; both names at the archive root are `UnsafeEntry` ("has the name of symdev's package receipt"), staging removed. `Receipt::PARTIAL` names the temporary file. Spec §3 steps 4–6. The order itself is not observable by a test (both refusals pass either way); reasoned, not tested |
| I2 | the download cache has no lock | yes: every process wrote `<sha>.tar.gz.part` and deleted a mismatching cached file; 8 threads with 8 homes and one cache failed 3/3 (`….tar.gz.part: No such file`), 6 `symdev sdk install` processes failed (`….tar.gz: No such file`) | fixed: `home/download_cache.rs` (`DownloadCache`, private to `home`): check/download/rename under `downloads/.lock`; `.part` named `<sha>.tar.gz.<pid>-<n>.part` and renamed over the cached name only once verified; a damaged cached copy is replaced, never deleted; stale parts of the same archive are removed under the lock (Ctrl-C leftovers); the verified handle is returned and extracted (`TarGz::extract_file`). Tests: 8 threads/8 homes/1 cache → one download, 20/20 green; mutant without the lock fails 3/3; stale part removed (mutant caught); CLI test with 6 processes (RED on the old code) |
| M9 | dot-leading id segments; multi-member gzip truncated | yes: `.staging;x` parsed (it would install into the staging dir that every install clears); a two-member `.tar.gz` whose first member ends at an entry boundary extracted only the first file, silently (test RED: `second` missing) | fixed: a segment starting with `.` is refused ("has a segment starting with `.`"; `.`/`..` keep their own reason); `TarGz` reads through `MultiGzDecoder`. Spec §2 |
| M1 | downloads not capped; no response timeout | yes: `HttpFetch::download` read to EOF, `FileFetch` copied all; only connect (30 s) and global (1 h) timeouts | fixed: `Fetch::download(url, dest, limit)` reads at most `limit + 1` bytes and errs past `limit` (`SdkError::longer_than`: URL, limit, "the source's index or its file is wrong"); `DownloadCache` passes the entry's `size`; `timeout_recv_response` 60 s, global 1 h kept. Tests (all RED first): HTTP 300 bytes with limit 100; `FileFetch` 10 bytes, limit 4; install with `size` one short → the download stops (was `HashMismatch` after the whole file); a silent server with a 200 ms wait (`HttpFetch::impatient`, test-only) errs instead of hanging (RED: still waiting after 10 s). Left: a server that answers and then stalls the body is bounded only by the hour |

Facts found on the way:

- I3: cargo takes `RUSTFLAGS` over `build.rustflags` even when it is **set but empty**:
  in a scratch crate, `cargo build -v --config 'build.rustflags=["--cfg","foo"]'` passes
  `--cfg foo` with `RUSTFLAGS` unset, and not with `RUSTFLAGS=""` nor `RUSTFLAGS="-D
  warnings"`. So the review's `RUSTFLAGS: ""` on the examples job would not help; the
  workflow-level `RUSTFLAGS` moves to the `check` job instead, and the examples job has none.

symdev-packages call sites (not edited; that repository builds against this crate by path):

- M1: `Fetch::download` gained a `limit` argument; the publisher neither calls `download`
  nor implements `Fetch`, so nothing there changes.
- M4: `publish/src/bucket.rs:4-5` imports `resolve_url` (drop it from the `use`);
  `publish/src/bucket.rs:46` `resolve_url(&self.spec.base, key)` becomes
  `self.spec.resolve(key)`. No call of `builtin_source`/`BUILTIN_SOURCE` there.
