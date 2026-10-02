# WIP: toolchain manager, Track D (integration) — branch `tm-cli`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks D1–D3.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md` (§3–§5, §8, §9).
Inputs: `toolchain-manager-core.md` ("For Track D"), `toolchain-manager-net.md`.

## Status

| Task | State | Commit |
|---|---|---|
| D1 ToolchainOverrides / Toolchain::resolve | done | see `git log` |
| D2 SdkManager / builtin | done | see `git log` |
| D3 CLI sdk / --offline / provision / hermetic tests / docs | todo | |
| smoke test (real toolchain, file:// source) | todo | |

## Facts

- Before D: `Toolchain::from_env` (symdev-build) is called only by `build_cmd.rs`;
  `Epocroot::from_env` only by `main.rs::epocroot_for` (used by `freeze` and `package`).
- `Toolchain::ar()` read `SYMDEV_AR` from the environment inside the value type.
- CLI tests `build_valid_manifest_no_bld_inf` / `build_does_not_require_external_elf2e32`
  pass fake paths (`/sdk`, `/gcc/g++`) that do not exist; spec §5 makes a missing
  `SYMDEV_*` path an error, so they must use real temp paths.
- `freeze` loads no `symdev.toml`; the SDK pin needs the device from it.

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

## Dead ends

## Next step

D3: CLI (`--offline`, `symdev sdk`, provision.rs), hermetic tests, docs.
