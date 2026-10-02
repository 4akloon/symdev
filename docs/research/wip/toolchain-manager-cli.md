# WIP: toolchain manager, Track D (integration) — branch `tm-cli`

Plan: `docs/superpowers/plans/2026-10-02-toolchain-manager.md`, tasks D1–D3.
Spec: `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md` (§3–§5, §8, §9).
Inputs: `toolchain-manager-core.md` ("For Track D"), `toolchain-manager-net.md`.

## Status

| Task | State | Commit |
|---|---|---|
| D1 ToolchainOverrides / Toolchain::resolve | done | see `git log` |
| D2 SdkManager / builtin | todo | |
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

## Dead ends

## Next step

D2: SdkManager + builtin, tests first.
