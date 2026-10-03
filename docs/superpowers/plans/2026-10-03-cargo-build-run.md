# `cargo build`, `cargo run`, `cargo test` for symdev Rust projects — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** In a project made by `symdev new --lang rust`, plain `cargo build` produces the
signed `.sisx`, `cargo run` picks a device, installs, launches and streams the app until it
ends, and `cargo test` runs `tests/*.rs` on the device with `libtest`-style output.

**Architecture:** The `symdev` binary gains two roles chosen by the name it is started under:
`symdev-ld`, cargo's linker, and `symdev-rustc`, the `rustc` wrapper of `rust-std` projects.
`symdev-ld` takes rustc's objects and rlibs, links them on 0.3.0's rust-lld path and packages
the image. `symdev run --exe` is cargo's runner. It talks JSON-RPC to an EKA2L1 started with
`--control` and `--data-dir` and chosen from a device registry.

**Tech Stack:** Rust 1.98.1 host crates (edition 2024), the pinned nightly `nightly-2026-09-19`
for `symbian-rs`, rust-lld 23.1.1, EKA2L1 (our fork, GPL-3.0, a separate process), JSON-RPC 2.0
over a Unix socket.

**Spec:** `docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`. The spike behind
every observed value in this plan is experiment 114 §1 in
`docs/research/experiment-backlog.md`. Read both before starting.

## Global Constraints

- `CLAUDE.md` is binding. Library paths return `Result`; no `unwrap`/`expect`/`panic!` outside
  tests. Every `.rs` file is at most 300 lines, tests included. One type per file. New API is a
  domain type with methods. Value types never read env, argv or stdout.
- Never invent tool argv. `symdev-ld` accepts exactly the arguments experiment 114 §1.1
  observed, and refuses anything else by name.
- Behaviour never observed from the real tools is an error (`TODO: … (not observed)`).
- Gates before any "done": `cargo test --workspace --offline` and `cargo clippy --workspace
  --all-targets --offline`, zero warnings; `cargo fmt --all --check`.
- Commit messages are one full imperative sentence ending with a period, and end with the
  attribution trailer the session gives.
- Work on branch `cargo-run` in `~/worktrees/symdev/cargo-run`. Stage files by name, never
  `git add -A`: builds rewrite `symbian-rs/examples/std-{hello,net}/Cargo.lock`.
- `symdev.toml` stays the only place for what the phone needs; it gains nothing. C++ projects
  are unchanged.
- EKA2L1 is GPL-3.0. The control client is written from its README only
  (`~/src/EKA2L1-wt/control-server/src/emu/control/README.md`, branch `dev/control-events`);
  never copy its code. Stop only instances symdev started, with `kill -9 <pid>`.
- Never commit SDK, ROM, `.sis`, `.sisx`, `.cer` or `.key` files.
- Target release: symdev 0.4.0 with `rust-sdk;0.4.0`. Do not push to `main`, tag or publish.
- New crates.io dependencies need one online `cargo fetch` before the offline gates; name each
  one in its task. This plan adds `ctrlc = "3"` (Task 13). `toml = "1"` is already in
  `Cargo.lock`.

## What experiment 114 §1 changed in the spec

The spec is right in its goals. These are the places where an observation replaced an
assumption; every task below is written for the observation:

1. **No `deps/`.** This cargo links each unit into `<target-dir>/<triple>/<profile>/build/
   <pkg>/<hash>/out/` and hard-links a binary up to `<target-dir>/<triple>/<profile>/<bin>`.
   A test binary is told from the main one by the environment: `CARGO_BIN_NAME` for a
   binary, `CARGO_TARGET_TMPDIR` without `CARGO_BIN_NAME` for a test (§1.6).
2. **`cargo run` hands the runner the hard link**, not the `-o` path. A `<out>.sisx` is not
   beside it. `symdev-ld` also writes `<profile-dir>/<bin>.sisx` for the main binary (§1.1).
3. **The project needs three more things.** `src/main.rs` needs `#![no_main]` (E0580
   otherwise). `.cargo/config.toml` needs `[unstable] panic-abort-tests = true`, or `cargo
   test` fails with E0152. The target spec needs `"default-visibility": "hidden"`, or a
   `dev` build does not link; release bytes do not change with it (§1.5).
4. **libcalls cannot be an ordinary dependency** without changing bytes (§1.3). `symdev-ld`
   runs today's `cargo rustc --profile libcalls …` itself, nested in cargo's build. This was
   observed to work with no lock wait.
5. **`rust-std` projects can go config-only** (§1.4): `[build] rustc` names `symdev-rustc`,
   which passes `--sysroot` to a sysroot symdev materialises. They therefore move to cargo
   too (Task 9), and do not stay on `symdev build`. Their images differ from 0.3.0's by
   path-dependent bytes only.
6. **UID3 at compile time comes from a proc macro.** A dependency's build script sees its
   own `CARGO_MANIFEST_DIR`, not the application's. A macro expanded in the application
   sees the application's, as `strings!()` already does. `symbian_std::uid3!()` reads
   `symdev.toml` (Task 5).
7. **Our EKA2L1 integration build has neither `--control` nor `--data-dir`.** The `symdev`
   branch in `~/src/EKA2L1-wt/integration` lacks both. Task 10 builds one from
   `dev/data-dir` + `dev/control-events`.
8. **Signing in `cargo build` needs an owner decision** (D1 below).
