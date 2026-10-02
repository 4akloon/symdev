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

Final verification (test, clippy, fmt), then report to the lead.

## Review fixes (branch `tm-review-fixes`, 2026-10-02)

Independent review of the toolchain manager (line numbers from 34987b3). Each finding was
checked against the code before changing it; baseline `cargo test --workspace --offline`:
652 passed. State per finding (commits: `git log`):

| # | Finding | Verified | State |
|---|---|---|---|
| 1 | `epocroot`/`installed_epocroot` check every `SYMDEV_*` path | yes: both call `overrides()` → `check()` of all 7 | fixed: both take the unchecked overrides; `Epocroot::resolve` checks `SYMDEV_EPOCROOT` itself (unit + CLI tests for package and freeze) |
| 2 | `available` vs `find` disagree when the first source lacks this host's archive | yes: the `&&` short-circuits before `seen.insert` | fixed: `seen.insert` first, so the first source that lists an id decides for `available` as for `find` |
| 3 | `keys_make_an_s3_source_searchable` makes a real HTTPS request | yes: `ensure` → `find` → ureq GET `https://127.0.0.1:1/…` (proxy from env) | fixed: replaced by `catalog::tests::keys_give_an_s3_source_a_fetcher` (no request; a mutant giving a keyless `s3` source a fetcher fails it). Not fixed, reported: `http_fetch` tests GET `http://127.0.0.1:<port>` and ureq takes the proxy from the environment, so with `HTTP_PROXY` set (and no `NO_PROXY` for 127.0.0.1) they go to the proxy |
| 4 | `Provision` reads the toolchain variables past its own lookup | yes: `ToolchainOverrides::from_env()` in `overrides()`/`needed()` | fixed: `ToolchainOverrides::from_lookup` is public, `from_env` deleted (no caller left), `Provision::overrides(&self)` |
| 5 | an all-installed build fails on download-only problems | yes: `manager()` (sources, keys, host) runs before `ensure` checks receipts | fixed: `Provision::install_missing` reads receipts first, builds the manager only for missing ids (CLI test: malformed `sources.toml`, half key pair; the host check sits in `SdkManager::new`, so it is skipped the same way) |
| 6 | a Rust build downloads before `RustSdk::from_env` can fail | yes: `toolchain()` at the top, `RustSdk::from_env()` in the match | fixed: `RustSdk` resolved first for a Rust project (CLI test: stale `SYMDEV_RUST_SDK`, nothing installed) |
| 7 | `installing …` printed before `install` re-checks under the lock | yes: `writeln!` precedes `home.install`, which returns early if installed | fixed: `SdkHome::install` takes `starting: impl FnOnce()`, run under the lock after the re-check; the manager prints from it (SdkHome prints nothing). Tests moved: `home/tests/receipts.rs` (300-line rule) |
| 8 | without `HOME` the message names `SYMDEV_HOME` for every path | yes: one text for data, cache and config | fixed: packages → `SYMDEV_HOME or XDG_DATA_HOME`, cache → `XDG_CACHE_HOME`, config → `XDG_CONFIG_HOME` |
| 9 | `bin()` keeps keys/toolchain vars; temp homes pile up in `target/tmp` | yes: 10+ `symdev-cli-home-*` after two runs | todo |
| 10 | `sdk;…` in no source and no private source: no way out named | yes | fixed: `Sources.file` keeps the real `sources.toml` path; an `sdk` id found nowhere without a keyless-source hint adds "set SYMDEV_EPOCROOT to your own SDK, or add a source that has it in <path>"; no sources at all names the path too (was a hard-coded `$XDG_CONFIG_HOME/…`). Spec §5, §8 (prerequisites + 4-line TOML, acceptance item 2), README, examples README |
| 11 | host-mismatch test wording | nit, kept as is | — |
