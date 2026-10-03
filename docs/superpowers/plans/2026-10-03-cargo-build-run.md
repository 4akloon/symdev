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

## Open decision for the owner — D1: signing in `cargo build`

`symdev package` refuses a `SYMDEV_SIGN_PASSWORD` shorter than 4 characters, even for
`[signing] mode = "self-signed"` (`SisPackage::validate_password`,
`crates/symdev-build/src/package.rs:47`). With the spec's "`cargo build` produces a signed
`.sisx`", a fresh project's first `cargo build` therefore fails until the user sets the
variable. Facts for the decision:

- With no `[signing] cert`/`key`, `SisPackage::package` generates a new self-signed pair on
  every run. Its key is **unencrypted** PKCS#8 (`symdev_makekeys::SelfSignedDsa::key_pem`,
  `BEGIN PRIVATE KEY`), and the password plays no part in signing with it. The check guards
  nothing on this path.
- The original tools allow a key with no password (experiment 114 §1.7, through Wine).
  `makekeys` without `-password` asks "Do you want to use a password (y/n)?". Answered `n`, it
  writes an unencrypted `BEGIN DSA PRIVATE KEY`, and `signsis` signs with it given no pass
  phrase.

| Option | What changes | Cost |
|---|---|---|
| **A (recommended)** | The password is required only for a key the user supplies (`[signing] key`) and only when that key is encrypted (`Proc-Type: 4,ENCRYPTED` or `BEGIN ENCRYPTED PRIVATE KEY`). A generated self-signed pair needs none | One rule changes in `SisPackage`; matches what the original tools allow |
| B | `symdev new` writes a random password to `.symdev/sign-password` (git-ignored); `symdev-ld` reads it when `SYMDEV_SIGN_PASSWORD` is unset | A secret file in every project that protects nothing on the generated-key path |
| C | Without the variable, `symdev-ld` stops after the `.exe` and prints `note: set SYMDEV_SIGN_PASSWORD (at least 4 characters) to get <name>.sisx`; the runner then refuses with the same text | Today's rule kept; a fresh `cargo run` fails until the user sets it |

**Recommendation: A.** The password never touches the key symdev generates. The original
tools allow an unencrypted self-signed key. A is the only option where `symdev new` → `cargo
run` works with no setup, which the spec's acceptance (§10) requires.

**Depends on D1:** Task 3, step 6, and Task 19's acceptance run. Every other task is
independent of it. If D1 is not decided when Task 3 is reached, implement steps 1–5, leave
step 6 unchecked, and go on.

## Review Focus

Five inputs the spec implies and no other test covers, most likely first. Each one has its
test in the task named.

1. **`cargo test` links the binary and every test at once.** Intermediates, shims and the
   import-stub object must not be shared between two `symdev-ld` processes of one project.
   Each link works in its own `<out>.symdev/` directory (Task 4, test
   `two_links_of_one_project_use_two_work_dirs`).
2. **`cargo run` from a subdirectory.** The runner gets a path relative to its working
   directory, not to the project root, and must resolve it that way (Task 13, test
   `a_relative_exe_is_resolved_against_the_working_directory`).
3. **A `.sisx` older than the image beside it**, left by an earlier link when the newest one
   failed after writing the image. The runner refuses it rather than installing the previous
   build (Task 13, test `a_sisx_older_than_its_image_is_refused`).
4. **The user's own EKA2L1 is open.** It is never listed, chosen or killed. A registry entry
   whose PID is now some other process is removed and never signalled (Task 11, test
   `an_entry_whose_pid_is_not_eka2l1_is_dropped_not_killed`).
5. **A `[[bin]]` renamed away from the package name.** `symdev-ld` sees a `CARGO_BIN_NAME`
   that is not `symdev.toml`'s `package.name`, and says so with both names instead of
   packaging it as a stray example (Task 1, test `a_binary_not_named_after_the_package_is_refused`).
