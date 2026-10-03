# `emulator` and `firmware;rm-469` packages — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On a clean machine, `install.sh` → `symdev new --lang rust` → `cargo run` installs
an EKA2L1 with `--control` and `--data-dir` (`emulator;<V>`, public bucket) and the E52
firmware (`firmware;rm-469;1`, private bucket) by itself, and shows the app.

**Architecture:** The fork's CI builds the AppImage from our rebuilt integration branch. A
recipe in `symdev-packages` takes that artifact by its SHA-256, extracts it and adds the
notices. The corresponding source goes out through the publisher's existing `--source-code`
archive. The owner stages the firmware from his EKA2L1 data and publishes it privately.
symdev resolves `SYMDEV_EKA2L1` / `SYMDEV_EKA2L1_DATA` first and the pinned packages
second. It starts `<package>/usr/bin/eka2l1_qt` directly, and makes an emulator profile
whose ROM and drive Z are links into the installed firmware package.

**Tech Stack:** Rust 1.98.1 (edition 2024) in `symdev` and in `symdev-packages`
(`publish`, `pkgtools`); bash recipe drivers; GitHub Actions (the fork's `build.yml`,
`symdev-packages`' workflows); Docker `ubuntu:24.04` for the CI rehearsal; EKA2L1 (GPL-3.0,
always a separate process).

**Spec:** `docs/superpowers/specs/2026-10-03-emulator-firmware-packages-design.md`, which
extends `docs/superpowers/specs/2026-10-03-cargo-build-run-design.md` §5–§6 and the toolchain
manager spec (`2026-10-02-toolchain-manager-design.md` §2, §4–§6, §12, §15). Every observed
value below comes from experiment 115 §1 (`docs/research/experiment-backlog.md`, end of
file). Read the spec and §1 before starting.

## Global Constraints

- `CLAUDE.md` is binding. Library paths return `Result`; no `unwrap`/`expect`/`panic!`
  outside tests. Every `.rs` file is at most 300 lines, tests included. One type per file.
  New API is a domain type with methods. Value types never read env, argv or stdout: in
  symdev that is `Provision` (CLI), in `symdev-packages` the `*Tool` types of `pkgtools`.
- Gates before any "done", in `~/worktrees/symdev/cargo-run`: `cargo test --workspace
  --offline`, `cargo clippy --workspace --all-targets --offline` (zero warnings), `cargo fmt
  --all --check`. In `~/worktrees/symdev-packages/cargo-run`: `cargo test --locked`, `cargo
  clippy --all-targets --locked` (zero warnings), `cargo fmt --all --check`, and the shell
  tests under `tests/`. Read a gate's result from a file, never through `| tail`.
- Commit messages: one full imperative sentence ending with a period, then the attribution
  trailer the session gives. Stage files by name, never `git add -A`.
- Branches: symdev `cargo-run` in `~/worktrees/symdev/cargo-run`; symdev-packages
  `cargo-run` in `~/worktrees/symdev-packages/cargo-run` (base `3fe6a76`, not pushed);
  EKA2L1 in an own copy `~/src/EKA2L1-wt/emulator-pkg`, branch `symdev`. Implementing agents
  may push symdev's `cargo-run`, and nothing else.
- **LEAD ONLY, after the owner's explicit go** (implementing agents prepare up to these and
  stop): L1 pushing the rebuilt `symdev` branch to `4akloon/EKA2L1`, which starts its CI;
  L2 publishing `emulator;<V>` to the public bucket; L3 publishing `firmware;rm-469;1` to
  the private bucket; L4 pushing `symdev-packages`.
- Firmware, ROM, SDK, `.sis`, `.sisx`, `.cer` and `.key` files never enter git, CI or the
  public bucket. Staged firmware trees and archives live in `~/src/emu-pkg-scratch/` only.
- EKA2L1 is GPL-3.0: read it to learn its layout, never copy its code into symdev or
  `pkgtools`. Never run it without `--data-dir` (even `--help`: that rotates the owner's
  logs). Run it under the agent lock `flock ~/.local/share/EKA2L1/.symdev-agent.lock`, stop
  only PIDs you started, with `kill -9`. Never write into `~/.local/share/EKA2L1`.
- Recurring helpers (recipe builds, CI steps, checks) are Rust: `pkgtools` subcommands, reusing
  its existing types. `build.sh` / `stage.sh` stay thin bash drivers that fetch, check
  `sha256sum` and call `pkgtools`. `pkgtools` stays offline (no HTTP).
- Never invent tool argv: EKA2L1 gets exactly `--data-dir <dir> --control <socket>` (and
  `--help` for the probe), as experiment 114 observed.
- Scratch: `~/src/emu-pkg-scratch/`. Each script starts with a comment saying what it does.
- Keep `docs/research/wip/emulator-packages.md` current: facts, rulings, the exact next
  step. Commit it after every task, and at least every ~15 minutes.
- `<V>`, the emulator version, is the UTC commit date of the rebuilt integration head as
  `yyyy.mm.dd` (Task 1 step 6 computes it and writes it to the notes). Replace `<V>`
  everywhere below with that value. `<C>` is that head's full SHA-1, and `<c>` its first
  7 hex digits.

## What experiment 115 §1 changed in the spec

1. **symdev starts `usr/bin/eka2l1_qt`, not `AppRun`.** `AppRun` is a symlink to it, and a
   process started as `AppRun` has the comm `AppRun`. `device::is_eka2l1` would drop it from
   the registry. The binary's `RUNPATH $ORIGIN/../lib` and `usr/bin/qt.conf` make the tree
   self-contained, with no environment needed.
2. **The fork has never run its CI.** The workflow is `active`, yet there are zero runs and
   zero artifacts, although branches were pushed. L1 includes checking that a run starts,
   and dispatching one if none does.
3. **The corresponding source is much larger than "EKA2L1 + Qt".** The AppImage bundles
   files from 167 Ubuntu packages, among them FFmpeg, x264 and x265 (GPL-2.0+). 35 of their
   copyright files are not machine-readable. Decision D1 below.
4. **The glibc floor is 2.38.** symdev's other packages run from glibc 2.28. Finding F1
   below; the recipe records the floor and `pkgtools` checks it.
5. **EKA2L1 writes drive Z at every start** (`avkonfep.dll` → `.bak`, then a copy that fails
   on Linux). Task 6 observes a read-only firmware package before Task 7 decides between
   links and copies.
6. **Today the default `~/.local/share/EKA2L1` is read without `SYMDEV_EKA2L1_DATA`**
   (`EmulatorData::from_env`). The spec's order makes the variable the only way to the
   user's data. The default folder is then no longer read, which the acceptance's `bwrap
   --tmpfs` relies on. Task 7 deletes `from_env`.
7. **The publisher refuses unknown recipe keys** (`deny_unknown_fields`, and `commit` needs
   a `tag`). The CI facts therefore live in `artifact.toml` beside `recipe.toml`, read only
   by `build.sh`. The publisher does not change.
8. **The fork's artifact will expire** (no `retention-days`; the repository default, at
   most 90 days). The recipe's hashes are the lasting record; the package in R2 stays.
