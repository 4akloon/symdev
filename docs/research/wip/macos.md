# WIP: the symdev toolchain on macOS (brief for the session on the owner's MacBook)

Owner, 2026-10-03: next stage after the current tracks is running the toolchain on macOS; a
separate Claude session on the owner's MacBook does it, in parallel with the Linux session.
**This file is that session's start and its running record** — read it, `git log`, and
`CLAUDE.md` first; update it after every finding; push the branch often so the Linux session
sees it.

## State on 2026-10-03 (from the Linux host)

- Released v0.1.0 (spec `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md` §13):
  `install.sh` + `symdev build` install GCCE, the SDK and the Rust SDK from R2 — **Linux
  x86_64 only**. Packages: `gcce;12.1.0` (host `x86_64-linux`), `sdk;s60-3rd-fp2;1.1`
  (`any`, private bucket), `rust-sdk;0.1.0` (`any`), `symdev;0.1.0` (`x86_64-linux`, static
  musl). Indexes are Ed25519-signed (§15).
- symdev 0.2.0 is being released from the Linux session (signed indexes, link-relative Rust
  SDK paths, `build/rust-sdk`); see `docs/research/wip/v0.2.md`.
- Everything between the compiler and the `.sisx` is native, portable Rust: E32 post-link
  (`symdev-elf2e32`), resources (`symdev-rcomp` + own preprocessor), MIF/MBM, SIS, signing.
  What is host-specific: the GCCE cross compiler (`arm-none-symbianelf-g++` 12.1.0 + GNU ld
  2.29.1), the `symdev` binary, `Host` in `crates/symdev-sdk/src/host.rs` (only
  `x86_64-linux` and `any`), `install.sh` (refuses non-Linux), EKA2L1.
- Experiment 109: Rust applications can be linked by `rust-lld` with prebuilt shims and no
  GCCE at all; productising it is planned on Linux (owner decisions in `v0.2.md`). On macOS
  that route would make Rust apps work without a Darwin GCCE.
- `docs/research/macos-bringup.md` (2026-09-18) is the older checklist; its elf2e32/Wine
  parts are stale (elf2e32 is native now).

## Suggested order (brainstorm → spec → plan per `CLAUDE.md` before code)

1. `cargo test --workspace --offline` and clippy on macOS with Rust 1.98.1; fix what breaks.
   Watch: APFS is case-insensitive by default (symdev builds a case-folded include tree for
   the SDK on Linux), `File::lock`, symlinks (`build/rust-sdk`), path assumptions.
2. Host support: `Host` variants for the Mac (`aarch64-macos`, and `x86_64-macos` if Intel
   matters), `install.sh` on Darwin (`shasum -a 256`, no `flock`, `uname -sm` = `Darwin
   arm64`), a `symdev;<ver>` package per macOS host built on GitHub's macOS runners in
   `symdev-packages` (a binary fetched with `curl` carries no quarantine attribute;
   codesigning is not needed for that path — verify).
3. GCCE for Darwin: the same official tarballs as `recipes/gcce/12.1.0` (experiments
   107–108), built on macOS; binutils 2.29.1 predates Apple Silicon (expect `config.sub`
   and build fixes — record each, observe, never guess). Acceptance like §13 item 1: the
   `.elf` of `hello`/`gui` byte-identical to the Linux package's.
4. EKA2L1 on macOS: build our fork's integration branch (`symdev` in `4akloon/EKA2L1`, being
   set up by the Linux session; upstream builds macOS with Qt6) or use upstream's macOS build
   for a first run; the owner's firmware must be installed into it on the Mac (never commit
   or upload firmware). Screenshots: `screencapture -l<windowid>`; the Linux skill's X11
   tricks do not apply.
5. Acceptance on the Mac: `install.sh`, then `symdev build` + `symdev package` of `hello`,
   `gui` and a `symdev new --lang rust` project, each launched in EKA2L1, window checked.

## Coordination with the Linux session

- Work on a branch `macos` (worktree under `~/worktrees/symdev/macos` or similar), rebase on
  `main` often; merge into `main` only after the gates in `CLAUDE.md` pass. The Linux session
  owns releases, tags and the R2 publish pipeline; propose recipe/workflow changes for
  `4akloon/symdev-packages` on a branch there (`macos`) and say so in this file.
- Do not edit `docs/superpowers/specs/2026-10-02-toolchain-manager-design.md` concurrently;
  write `docs/superpowers/specs/<date>-macos-toolchain-design.md`, and add new experiments
  at the end of `docs/research/experiment-backlog.md` with the next free number (pull first).
- Keys: the owner copies `~/.config/symdev/keys.env` and `~/.config/symdev/sources.toml` to the
  Mac over a private channel (never git, never chat). The SDK comes from the private bucket
  with the reader key; nothing proprietary is copied by hand.

## Log

(the Mac session appends facts, decisions, dead ends and the next step here)
