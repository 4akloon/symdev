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

## File structure

| Path | Responsibility |
|---|---|
| `crates/symdev-cli/src/role.rs` | `Role`: which program the `symdev` binary is, from `argv[0]` |
| `crates/symdev-cli/src/ld.rs` + `ld/` | the `symdev-ld` role |
| `ld/linker_args.rs` | `LinkerArgs`: rustc's argv, the observed flags only |
| `ld/cargo_link_env.rs` | `CargoLinkEnv`: the cargo variables a link sees |
| `ld/link_kind.rs` | `LinkKind`: the main binary or a test |
| `ld/cargo_output.rs` | `CargoOutput`: the `-o` path in cargo's layout, and the paths beside it |
| `ld/link_record.rs` | `LinkRecord`: `<out>.symdev.toml`, what the runner needs to know |
| `ld/link_run.rs` | `LinkRun`: one `symdev-ld` invocation, end to end |
| `ld/testdata/` | experiment 114's recorded argv and environments |
| `crates/symdev-cli/src/setup_linker.rs` | `symdev setup-linker` |
| `crates/symdev-cli/src/sisx.rs` | `ProjectPackage`: `package_project`'s body as a type |
| `crates/symdev-cli/src/rustc_wrapper.rs` | `RustcWrapper`: the `symdev-rustc` role |
| `crates/symdev-cli/src/run.rs` + `run/` | the runner: `ExeTarget`, `AppExit`, `Runner`, `Interrupt` |
| `crates/symdev-cli/src/libtest_print.rs` | `LibtestPrint`: `test x ... ok` and the summary |
| `crates/symdev-cli/src/devices_cmd.rs` | `symdev devices`, `symdev emulator start/stop` |
| `crates/symdev-cli/src/old_shape.rs` | `OldShape`: the 0.3.0 `staticlib` project and its edits |
| `crates/symdev-build/src/driver/rustc_link.rs` | `RustcLink` + `RustBuild::link_rustc_output` |
| `crates/symdev-build/src/std_sysroot.rs` | `StdSysroot`: the sysroot `symdev-rustc` points at |
| `crates/symdev-emulator/src/control.rs` + `control/` | `ControlClient`, `Request`, `AppExited` |
| `crates/symdev-emulator/src/device.rs` + `device/` | `DeviceId`, `DeviceRegistry`, `EmulatorProfile`, `DeviceChoice`, `EmulatorInstance` |
| `symbian-rs/crates/symbian-macros/src/manifest_uid3.rs` | `ManifestUid3`: `symdev.toml`'s UID3 at expansion time |
| `symbian-rs/crates/symbian-test/` | the `harness = false` test harness |

---

### Task 1: The recorded rustc calls as fixtures, and the types that read them

**Files:**
- Create: `crates/symdev-cli/src/ld.rs`, `crates/symdev-cli/src/ld/{linker_args,cargo_link_env,link_kind,cargo_output}.rs`, `crates/symdev-cli/src/ld/tests.rs`
- Create: `crates/symdev-cli/src/ld/testdata/` (copied from `~/src/cargo-run-scratch/fixtures/`)
- Modify: `crates/symdev-cli/src/main.rs` (add `mod ld;`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `LinkerArgs::parse(impl IntoIterator<Item = OsString>) -> Result<LinkerArgs>` with
    `pub inputs: Vec<PathBuf>`, `pub output: PathBuf`, `pub raw_dylibs: Vec<PathBuf>`.
  - `CargoLinkEnv::from_pairs(impl IntoIterator<Item = (String, String)>) -> CargoLinkEnv` with
    `pub bin_name`, `pub crate_name`, `pub target_tmpdir: Option<String>` and
    `pub manifest_dir: Option<PathBuf>`.
  - `LinkKind::of(&CargoLinkEnv, package: &str) -> Result<LinkKind>`, where
    `enum LinkKind { Main, Test { name: String } }`.
  - `CargoOutput::of(&Path) -> Result<CargoOutput>` with `path()`, `profile_dir()`,
    `work_dir()` (`<out>.symdev`), `sisx()` (`<out>.sisx`), `record()`
    (`<out>.symdev.toml`).

- [ ] **Step 1: Copy the fixtures and check them**

```bash
mkdir -p crates/symdev-cli/src/ld/testdata
cp ~/src/cargo-run-scratch/fixtures/{release,dev}-{bin,test}.{argv,env} \
   ~/src/cargo-run-scratch/fixtures/release-bin-flavor.argv \
   ~/src/cargo-run-scratch/fixtures/release-example.{argv,env} \
   ~/src/cargo-run-scratch/fixtures/{run,test-run}.{argv,env} crates/symdev-cli/src/ld/testdata/
(cd crates/symdev-cli/src/ld/testdata && sha256sum * | cut -c1-16,65-)
```

Expected (first 16 hex digits):
`376a67de2d6f8b4c dev-bin.argv`, `bbf8d8ebe141a666 dev-bin.env`, `c57211ec975a1554 dev-test.argv`,
`c1ea86bbaf4f2c39 dev-test.env`, `958eb68fd78aaaf0 release-bin.argv`,
`bbf8d8ebe141a666 release-bin.env`, `9f3baeac322e2de7 release-bin-flavor.argv`,
`c0d6c6058d696cfb release-example.argv`, `ec52cd24b3986728 release-example.env`,
`c6acf1e1b21c5c91 release-test.argv`, `5d8c53d6c4160d0d release-test.env`,
`be709ab59ee8cd4c run.argv`, `6be51f90290a6a7c run.env`, `342a8886bcc0fe94 test-run.argv`,
`a8c3e74b10eec0e5 test-run.env`. If the scratch copy is gone, rebuild the files from
experiment 114 §1.1. `release-bin.argv` is the 13-line block printed there. The test variant
differs only in the object name and the `-o` path:

```
/work/app/build/cargo/arm-symbian-e32/release/build/app/4f71ac116363b7c1/out/smoke-4f71ac116363b7c1.smoke.8fa819f7a81627d9-cgu.0.rcgu.o
--as-needed
-Bstatic
/work/app/build/cargo/arm-symbian-e32/release/build/compiler_builtins/af926986b8385648/out/libcompiler_builtins-af926986b8385648.rlib
-L
/work/app/build/cargo/arm-symbian-e32/release/build/app/4f71ac116363b7c1/out/rustcXXXXXX/raw-dylibs
-Bdynamic
-z
noexecstack
-o
/work/app/build/cargo/arm-symbian-e32/release/build/app/4f71ac116363b7c1/out/smoke-4f71ac116363b7c1
--gc-sections
--strip-debug
```

`release-test.env`:

```
CARGO_BIN_EXE_app=/work/app/build/cargo/arm-symbian-e32/release/app
CARGO_CRATE_NAME=smoke
CARGO_MANIFEST_DIR=/work/app
CARGO_MANIFEST_PATH=/work/app/Cargo.toml
CARGO_PKG_NAME=app
CARGO_PRIMARY_PACKAGE=1
CARGO_TARGET_TMPDIR=/work/app/build/cargo/arm-symbian-e32/tmp
RUSTUP_TOOLCHAIN=nightly-2026-09-19-x86_64-unknown-linux-gnu
```

- [ ] **Step 2: Write the failing tests** — `crates/symdev-cli/src/ld/tests.rs`

```rust
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::{CargoLinkEnv, CargoOutput, LinkKind, LinkerArgs};

fn argv(name: &str) -> Vec<OsString> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ld/testdata").join(name);
    std::fs::read_to_string(path).unwrap().lines().map(OsString::from).collect()
}

fn env(name: &str) -> CargoLinkEnv {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ld/testdata").join(name);
    let text = std::fs::read_to_string(path).unwrap();
    CargoLinkEnv::from_pairs(text.lines().filter_map(|l| {
        l.split_once('=').map(|(k, v)| (k.to_string(), v.to_string()))
    }))
}

const OUT: &str = "/work/app/build/cargo/arm-symbian-e32/release/build/app/51c0ecfd4d2a2dcf/out";

#[test]
fn release_binary_gives_the_lto_object_then_compiler_builtins() {
    let a = LinkerArgs::parse(argv("release-bin.argv")).unwrap();
    assert_eq!(a.inputs, vec![
        PathBuf::from(format!("{OUT}/app.app.6fa0adbb789d939e-cgu.0.rcgu.o")),
        PathBuf::from("/work/app/build/cargo/arm-symbian-e32/release/build/compiler_builtins/af926986b8385648/out/libcompiler_builtins-af926986b8385648.rlib"),
    ]);
    assert_eq!(a.output, PathBuf::from(format!("{OUT}/app")));
    assert_eq!(a.raw_dylibs, vec![PathBuf::from(format!("{OUT}/rustcXXXXXX/raw-dylibs"))]);
}

#[test]
fn every_recorded_call_parses_and_keeps_input_order() {
    for (file, inputs) in [("dev-bin.argv", 36), ("dev-test.argv", 19),
                           ("release-test.argv", 2), ("release-example.argv", 2)] {
        let a = LinkerArgs::parse(argv(file)).unwrap();
        assert_eq!(a.inputs.len(), inputs, "{file}");
        assert!(a.inputs[0].to_string_lossy().ends_with(".o"), "{file}");
    }
    let dev = LinkerArgs::parse(argv("dev-bin.argv")).unwrap();
    assert!(dev.inputs[0].ends_with("symbols.o"));
    assert!(dev.inputs.last().unwrap().to_string_lossy().contains("libcompiler_builtins-"));
}

#[test]
fn a_linker_name_without_ld_gets_flavor_gnu_first_and_the_same_line_otherwise() {
    let plain = LinkerArgs::parse(argv("release-bin.argv")).unwrap();
    let flavor = LinkerArgs::parse(argv("release-bin-flavor.argv")).unwrap();
    assert_eq!(plain, flavor);
}

#[test]
fn an_unseen_argument_is_refused_by_name() {
    let mut a = argv("release-bin.argv");
    a.insert(3, OsString::from("--eh-frame-hdr"));
    let e = LinkerArgs::parse(a).unwrap_err().to_string();
    assert!(e.contains("`--eh-frame-hdr`") && e.contains("experiment 114"), "{e}");
    let mut z = argv("release-bin.argv");
    let at = z.iter().position(|x| x == "noexecstack").unwrap();
    z[at] = OsString::from("relro");
    assert!(LinkerArgs::parse(z).unwrap_err().to_string().contains("`-z relro`"));
}

#[test]
fn no_output_or_no_input_is_an_error() {
    let a: Vec<OsString> = argv("release-bin.argv").into_iter()
        .filter(|x| !x.to_string_lossy().ends_with("/out/app") && x != "-o").collect();
    assert!(LinkerArgs::parse(a).unwrap_err().to_string().contains("no `-o`"));
    let b = ["-o", "/x/out/app"].map(OsString::from);
    assert!(LinkerArgs::parse(b).unwrap_err().to_string().contains("no object"));
}

#[test]
fn the_binary_and_the_test_are_told_apart_by_cargos_variables() {
    assert_eq!(LinkKind::of(&env("release-bin.env"), "app").unwrap(), LinkKind::Main);
    assert_eq!(LinkKind::of(&env("dev-bin.env"), "app").unwrap(), LinkKind::Main);
    assert_eq!(LinkKind::of(&env("release-test.env"), "app").unwrap(),
               LinkKind::Test { name: "smoke".into() });
    assert_eq!(LinkKind::of(&env("dev-test.env"), "app").unwrap(),
               LinkKind::Test { name: "smoke".into() });
}

#[test]
fn a_binary_not_named_after_the_package_is_refused() {
    let e = LinkKind::of(&env("release-example.env"), "app").unwrap_err().to_string();
    assert!(e.contains("`demo`") && e.contains("`app`"), "{e}");
}

#[test]
fn neither_a_binary_nor_a_test_is_refused() {
    let e = LinkKind::of(&CargoLinkEnv::from_pairs([]), "app").unwrap_err().to_string();
    assert!(e.contains("CARGO_BIN_NAME") && e.contains("CARGO_TARGET_TMPDIR"), "{e}");
}

#[test]
fn the_output_names_the_profile_directory_cargo_links_the_binary_into() {
    let o = CargoOutput::of(Path::new(&format!("{OUT}/app"))).unwrap();
    assert_eq!(o.profile_dir(), Path::new("/work/app/build/cargo/arm-symbian-e32/release"));
    assert_eq!(o.sisx(), PathBuf::from(format!("{OUT}/app.sisx")));
    assert_eq!(o.work_dir(), PathBuf::from(format!("{OUT}/app.symdev")));
    assert_eq!(o.record(), PathBuf::from(format!("{OUT}/app.symdev.toml")));
}

#[test]
fn an_output_outside_cargos_observed_layout_is_refused() {
    for odd in ["/work/app/build/cargo/arm-symbian-e32/release/deps/app-0123456789abcdef",
                "/tmp/app", "app"] {
        let e = CargoOutput::of(Path::new(odd)).unwrap_err().to_string();
        assert!(e.contains("not observed"), "{odd}: {e}");
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p symdev-cli --offline ld::tests`
Expected: compile errors, `LinkerArgs`, `CargoLinkEnv`, `LinkKind`, `CargoOutput` not found.
