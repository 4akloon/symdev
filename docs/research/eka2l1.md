# EKA2L1 appendix

Optional emulator path for M0 de-risk. Promoted from spec §12. EKA2L1 is **GPL-3.0** — invoke it as a **separate process** only. Do not copy EKA2L1 source into this tree ([licensing.md](licensing.md)).

## Environment variables

| Variable | Meaning |
|---|---|
| `SYMDEV_EKA2L1` | Absolute path to an EKA2L1 executable the user installed themselves. |
| `SYMDEV_ROM` | Absolute path to a user-supplied ROM image. The user must have the right to use it (typically dumped from hardware they own). Path stays outside git. |

The runbook chapter 11 and later experiments use **only** these two variables. Do not invent additional env vars here.

## User responsibility

- Install EKA2L1 on the host yourself. Set `SYMDEV_EKA2L1` to the executable path.
- Supply a ROM you are entitled to use. Set `SYMDEV_ROM` to its path. Never commit, bundle, or curl ROM URLs from this repository.

## Skip rule

If `SYMDEV_ROM` or `SYMDEV_EKA2L1` is unset, or either path does not exist, **skip** the emulator experiment. Record the skip under `docs/research/`; do not treat it as a failed `cargo test`. Skipping does **not** fail §17 accept. Skipping does **not** authorize claiming emulator support or E52 support.

## Unknown until experiment

The following are **Unknown** until observed on a host with a user-supplied binary and ROM. Do not invent values in the runbook, Dockerfile, or CLI.

| Topic | Status |
|---|---|
| Expected on-disk ROM layout | **Unknown** until observed |
| CLI flags to install a `.sisx` | **Unknown — requires experiment** |
| CLI flags to launch an app | **Unknown — requires experiment** |

```
UNKNOWN — requires experiment
```

Experiment 10 in the backlog covers EKA2L1 install + launch with user ROM and observed CLI. Pass/fail is written in research notes, never CI.

## Scope boundaries

- **Headless install + launch + screenshot** is **M5**, not §17. This appendix documents the optional interim path only; it does not define automation.
- **Emulator success ≠ E52 supported.** Hardware M0 requires a hand-built `.sisx` to install and launch on a **stock** Nokia E52. A working EKA2L1 session does not satisfy that bar.
- **§17 CLI never spawns EKA2L1.** Humans follow [m0-bare-metal-runbook.md](m0-bare-metal-runbook.md) chapter 11. A future `EmulatorBackend` (post-§17) may spawn EKA2L1 with `SYMDEV_ROM`; that is not implemented in this cycle.
- **No EKA2L1 in Docker.** The Dockerfile bind-mounts SDK only; ROM and the emulator binary stay user-supplied on the host.

## Related

- Where to get the binary / legal ROM: [eka2l1-bringup.md](eka2l1-bringup.md)
- Runbook chapter: [m0-bare-metal-runbook.md](m0-bare-metal-runbook.md) §11
- Licensing: [licensing.md](licensing.md)
- Spec: `docs/superpowers/specs/2026-09-16-symdev-m0-north-star-design.md` §12
