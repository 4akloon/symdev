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

- [ ] **Step 4: Implement** — `crates/symdev-cli/src/ld.rs`

```rust
//! The `symdev-ld` role: cargo's linker for `arm-symbian-e32` (design spec §4).
mod cargo_link_env;
mod cargo_output;
mod link_kind;
mod linker_args;

pub(crate) use cargo_link_env::CargoLinkEnv;
pub(crate) use cargo_output::CargoOutput;
pub(crate) use link_kind::LinkKind;
pub(crate) use linker_args::LinkerArgs;

#[cfg(test)]
mod tests;
```

`crates/symdev-cli/src/ld/linker_args.rs`:

```rust
//! `LinkerArgs`: rustc's argv to cargo's linker, as experiment 114 §1.1 recorded it.
use std::ffi::OsString;
use std::path::PathBuf;

use symdev_core::{Error, Result};

/// The inputs, in rustc's order, and the `-o` path. Every other argument rustc was seen
/// to pass is either already on 0.3.0's line (`--gc-sections`, `--strip-debug`) or means
/// nothing to it (`--as-needed`, `-Bstatic`, `-Bdynamic`, `-z noexecstack`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkerArgs {
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    /// rustc's `-L <tmp>/raw-dylibs`: empty for every program observed. `LinkRun`
    /// refuses a non-empty one, which would mean a `raw-dylib` import nobody has seen.
    pub raw_dylibs: Vec<PathBuf>,
}

impl LinkerArgs {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self> {
        let mut words = Vec::new();
        for a in args {
            words.push(a.into_string().map_err(|a| {
                Error::Other(format!("symdev-ld: an argument is not UTF-8: {}", a.to_string_lossy()))
            })?);
        }
        let mut rest = words.as_slice();
        if let [flavor, gnu, tail @ ..] = rest
            && flavor == "-flavor" && gnu == "gnu"
        {
            rest = tail;
        }
        let (mut inputs, mut output, mut raw_dylibs) = (Vec::new(), None, Vec::new());
        let mut it = rest.iter();
        while let Some(word) = it.next() {
            match word.as_str() {
                "--as-needed" | "-Bstatic" | "-Bdynamic" | "--gc-sections" | "--strip-debug" => {}
                "-z" => match it.next().map(String::as_str) {
                    Some("noexecstack") => {}
                    other => return Err(unseen(&format!("-z {}", other.unwrap_or("")))),
                },
                "-L" => raw_dylibs.push(PathBuf::from(value(&mut it, "-L")?)),
                "-o" if output.is_none() => output = Some(PathBuf::from(value(&mut it, "-o")?)),
                w if !w.starts_with('-') && (w.ends_with(".o") || w.ends_with(".rlib")) => {
                    inputs.push(PathBuf::from(w))
                }
                w => return Err(unseen(w)),
            }
        }
        let output = output.ok_or_else(|| {
            Error::Other("symdev-ld: rustc passed no `-o`; symdev-ld is cargo's linker".into())
        })?;
        if inputs.is_empty() {
            return Err(Error::Other("symdev-ld: rustc passed no object or rlib".into()));
        }
        Ok(Self { inputs, output, raw_dylibs })
    }
}

fn value<'a>(it: &mut impl Iterator<Item = &'a String>, flag: &str) -> Result<&'a String> {
    it.next()
        .ok_or_else(|| Error::Other(format!("symdev-ld: `{flag}` without a value")))
}

fn unseen(word: &str) -> Error {
    Error::Other(format!(
        "symdev-ld: rustc passed `{word}`, which experiment 114 never saw it pass for \
         arm-symbian-e32; TODO: support it once observed (not observed). Report it with the \
         command that produced it."
    ))
}
```

`crates/symdev-cli/src/ld/cargo_link_env.rs`:

```rust
//! `CargoLinkEnv`: the variables cargo gives rustc, and rustc its linker (exp. 114 §1.1).
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CargoLinkEnv {
    pub bin_name: Option<String>,
    pub crate_name: Option<String>,
    pub target_tmpdir: Option<String>,
    pub manifest_dir: Option<PathBuf>,
}

impl CargoLinkEnv {
    /// From `(name, value)` pairs — `std::env::vars()` in `LinkRun`, a fixture in tests.
    pub fn from_pairs(pairs: impl IntoIterator<Item = (String, String)>) -> Self {
        let mut env = Self::default();
        for (k, v) in pairs {
            match k.as_str() {
                "CARGO_BIN_NAME" => env.bin_name = Some(v),
                "CARGO_CRATE_NAME" => env.crate_name = Some(v),
                "CARGO_TARGET_TMPDIR" => env.target_tmpdir = Some(v),
                "CARGO_MANIFEST_DIR" => env.manifest_dir = Some(PathBuf::from(v)),
                _ => {}
            }
        }
        env
    }
}
```

`crates/symdev-cli/src/ld/link_kind.rs`:

```rust
//! `LinkKind`: what `symdev-ld` is linking (experiment 114 §1.6).
use symdev_core::{Error, Result};

use super::CargoLinkEnv;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkKind {
    /// The project's `[[bin]]`: `CARGO_BIN_NAME` is the package name.
    Main,
    /// A `tests/<name>.rs` with `harness = false`: no `CARGO_BIN_NAME`, a
    /// `CARGO_TARGET_TMPDIR`. Linked as a console program even in an Avkon project.
    Test { name: String },
}

impl LinkKind {
    pub fn of(env: &CargoLinkEnv, package: &str) -> Result<Self> {
        match (&env.bin_name, &env.target_tmpdir, &env.crate_name) {
            (Some(bin), _, _) if bin == package => Ok(Self::Main),
            (Some(bin), _, _) => Err(Error::Other(format!(
                "symdev-ld: cargo is linking the binary `{bin}`, but symdev.toml's package is \
                 `{package}`: a symdev project has one [[bin]], named after the package; \
                 examples and further binaries are not supported"
            ))),
            (None, Some(_), Some(name)) => Ok(Self::Test { name: name.clone() }),
            _ => Err(Error::Other(
                "symdev-ld: neither CARGO_BIN_NAME (a binary) nor CARGO_TARGET_TMPDIR (a test) \
                 is set; symdev-ld is cargo's linker, run `cargo build`"
                    .into(),
            )),
        }
    }
}
```

`crates/symdev-cli/src/ld/cargo_output.rs`:

```rust
//! `CargoOutput`: rustc's `-o` in cargo's build-directory layout (experiment 114 §1.1):
//! `<target-dir>/<triple>/<profile>/build/<package>/<16 hex>/out/<file>`.
use std::path::{Path, PathBuf};

use symdev_core::{Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CargoOutput {
    path: PathBuf,
    profile_dir: PathBuf,
}

impl CargoOutput {
    pub fn of(path: &Path) -> Result<Self> {
        let up: Vec<&Path> = path.ancestors().take(6).collect();
        let name = |i: usize| up.get(i).and_then(|p| p.file_name()).and_then(|n| n.to_str());
        let hash_ok = name(2).is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()));
        if name(1) != Some("out") || !hash_ok || name(4) != Some("build") || up.len() < 6 {
            return Err(Error::Other(format!(
                "symdev-ld: rustc's output {} is not in cargo's \
                 <profile>/build/<package>/<hash>/out/ layout; TODO: another cargo layout \
                 (not observed)",
                path.display()
            )));
        }
        Ok(Self { path: path.to_path_buf(), profile_dir: up[5].to_path_buf() })
    }

    pub fn path(&self) -> &Path { &self.path }

    /// Where cargo hard-links the main binary, and so where `cargo run`'s path points.
    pub fn profile_dir(&self) -> &Path { &self.profile_dir }

    pub fn work_dir(&self) -> PathBuf { self.with_suffix(".symdev") }
    pub fn sisx(&self) -> PathBuf { self.with_suffix(".sisx") }
    pub fn record(&self) -> PathBuf { self.with_suffix(".symdev.toml") }

    fn with_suffix(&self, suffix: &str) -> PathBuf {
        let mut s = self.path.clone().into_os_string();
        s.push(suffix);
        PathBuf::from(s)
    }
}
```

Add `mod ld;` to `crates/symdev-cli/src/main.rs`, after `mod cli;`. The module is unused by
`main` until Task 4; add `#[allow(dead_code)]` on the `mod ld;` line with the comment `//
Task 4 wires the symdev-ld role`, and remove it in Task 4.

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test -p symdev-cli --offline ld::tests`
Expected: 10 passed. Then `cargo fmt --all` and `cargo clippy -p symdev-cli --all-targets
--offline` — zero warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/symdev-cli/src/main.rs crates/symdev-cli/src/ld.rs crates/symdev-cli/src/ld
git commit -m "Read rustc's linker argv and cargo's variables as experiment 114 recorded them."
```

### Task 2: `RustBuild` links rustc's inputs in a directory of its own

**Files:**
- Modify: `crates/symdev-build/src/driver/link_inputs.rs` (`archive` → `rust: &[PathBuf]`)
- Modify: `crates/symdev-build/src/driver/rust_link.rs` (`link_args`, `link_line` take `&[PathBuf]`)
- Modify: `crates/symdev-build/src/driver/rust_lld_link.rs` (passes `inputs.rust`)
- Modify: `crates/symdev-build/src/driver/rust_shims.rs` (`shim_archives`, `build_shims` take the work directory)
- Modify: `crates/symdev-build/src/driver/rust_build.rs` (the 0.3.0 path passes `&[archive]` and `build/`)
- Create: `crates/symdev-build/src/driver/rustc_link.rs` (`RustcLink`, `RustBuild::link_rustc_output`)
- Modify: `crates/symdev-build/src/driver/mod.rs`, `crates/symdev-build/src/lib.rs` (export `RustcLink`)
- Test: `crates/symdev-build/src/driver/tests/{rust_build,libcalls,lld_line,link,rust_ui,rust_lld_link}.rs`

**Interfaces:**
- Consumes: nothing from Task 1.
- Produces:
  - `pub struct RustcLink { pub inputs: Vec<PathBuf>, pub work: PathBuf }`.
  - `RustBuild::link_rustc_output(&self, project: &Project, link: &RustcLink) ->
    Result<Vec<Artifact>>`. The first artifact is `link.work/<name>.exe`; the UI and strings
    resources follow, all inside `link.work`.
  - `RustBuild::link_args(&self, rust: &[PathBuf], shim: Option<&Path>, libcalls:
    Option<&Path>, elf: &Path, map: &Path) -> Result<Vec<String>>`.

Experiment 114 §1.2 proved the rule this task encodes. rustc's inputs take the staticlib's
place in order: the first where the archive stood, the rest right after it, before the
shims and libcalls. With that rule all 19 `no_std` images are byte-equal.

- [ ] **Step 1: Write the failing tests** — append to `crates/symdev-build/src/driver/tests/rust_build.rs`

```rust
#[test]
fn rustc_inputs_stand_where_the_archive_stood_in_order() {
    let b = rust();
    let (elf, map) = (Path::new("/p/w/hello.elf"), Path::new("/p/w/hello.exe.map"));
    let obj = PathBuf::from("/p/out/hello.hello.9136cb57f297e5ab-cgu.0.rcgu.o");
    let cb = PathBuf::from("/p/out/libcompiler_builtins-af926986b8385648.rlib");
    let shim = PathBuf::from("/p/w/shims/libsymrs.a");
    let lc = PathBuf::from("/p/build/cargo/arm-symbian-e32/libcalls/libsymbian_libcalls.rlib");
    let got = b.link_args(&[obj.clone(), cb.clone()], Some(&shim), Some(&lc), elf, map).unwrap();
    let mut want = b.link_args(&[obj.clone()], Some(&shim), Some(&lc), elf, map).unwrap();
    let at = want.iter().position(|x| *x == obj.display().to_string()).unwrap();
    want.insert(at + 1, cb.display().to_string());
    assert_eq!(got, want);
    let pos = |p: &PathBuf| got.iter().position(|x| *x == p.display().to_string()).unwrap();
    let drt = got.iter().position(|x| x == "-l:drtaeabi.dso").unwrap();
    assert!(drt < pos(&obj) && pos(&obj) < pos(&cb) && pos(&cb) < pos(&shim) && pos(&shim) < pos(&lc));
}

#[test]
fn a_link_with_no_rust_input_is_an_error() {
    let (elf, map) = (Path::new("/p/w/hello.elf"), Path::new("/p/w/hello.exe.map"));
    let e = rust().link_args(&[], None, None, elf, map).unwrap_err().to_string();
    assert!(e.contains("no Rust object"), "{e}");
}
```

In `the_sdk_owns_the_shim_sources_and_compiles_them_with_the_cpp_argv` and
`shim_objects_follow_the_archive_and_keep_the_dso_ordering`, change the expected
`/p/build/shims/…` paths to `/p/w/shims/…` and pass `Path::new("/p/w")` as the work
directory. That is the reason for the change: two links of one project, under `cargo test`,
must not share `shims/` or `sdk-include-casefold/`.

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p symdev-build --offline driver::tests`
Expected: compile errors (`link_args` takes `&Path`; `shim_archives` takes no directory).

- [ ] **Step 3: Implement**

`link_inputs.rs` becomes:

```rust
//! `LinkInputs`: what a Rust program's link takes besides the SDK's own files.
use std::path::{Path, PathBuf};

/// rustc's objects and rlibs (or 0.3.0's one staticlib), the shim archives in link order,
/// and the libcall archive.
pub struct LinkInputs<'a> {
    pub rust: &'a [PathBuf],
    pub shims: &'a [PathBuf],
    pub libcalls: &'a Path,
}
```

In `rust_link.rs`, `link_args` and `link_line` take `rust: &[PathBuf]` in place of
`archive: &Path`, and `link_line` returns `Result<Vec<String>>`. Its first lines:

```rust
        let Some((first, rest)) = rust.split_first() else {
            return Err(Error::Other(
                "the link has no Rust object: rustc passed none, or cargo built nothing".into(),
            ));
        };
        let archive = first.as_path();
```

Everything after that is unchanged, with `archive` as before. After the existing
`args.splice(after..after, extras);` the rest go in right behind the first input, so the
shims and libcalls follow the last of them:

```rust
        let at = args.iter().position(|a| a == &arg(archive)).map_or(args.len(), |i| i + 1);
        args.splice(at..at, rest.iter().map(|p| arg(p)));
```

(Insert `rest` *before* the existing `extras` splice, so that `after` in that splice is
computed from the last Rust input: replace its `position(|a| a == &arg(archive))` with
`position(|a| rust.last().is_some_and(|l| a == &arg(l)))`.)

`rust_lld_link.rs`: `self.link_line(&linker, inputs.rust, inputs.shims, Some(inputs.libcalls),
&first_elf, map)?`.

`rust_shims.rs`: add `work: &Path` as the last parameter of `shim_archives` and
`build_shims`. `build_shims` uses `work.join("shims")` and `work.join("sdk-include-casefold")`
instead of `project.root.join("build")…`.

`rust_build.rs` (the 0.3.0 path, removed in Task 15): pass `&[archive.clone()]` and
`&build_dir`.

`rustc_link.rs`:

```rust
//! `RustcLink`: a link of what rustc compiled, run by `symdev-ld` (design spec §4).
use std::path::PathBuf;

use symdev_core::{Artifact, Project, RemotePath, Result};

use super::{LinkInputs, RustBuild, arg, produced};
use crate::RustLinker;
use crate::file_error;
use crate::required_capability::RequiredCapability;

/// One link's inputs and its own directory. `cargo test` links the binary and every test
/// at once, so nothing a link writes may be shared with another (experiment 114 §1.1).
pub struct RustcLink {
    /// rustc's objects and rlibs, in rustc's order (experiment 114 §1.2).
    pub inputs: Vec<PathBuf>,
    /// Shims, both ELFs, the import stubs, the map, the image and its resources.
    pub work: PathBuf,
}

impl RustBuild {
    /// 0.3.0's link after cargo, on rustc's inputs: the shims, the libcall archive (built
    /// by the same `cargo rustc` as 0.3.0, now nested in cargo's own build: experiment
    /// 114 §1.3 saw no lock wait), rust-lld's two links or GNU ld, the capability check,
    /// elf2e32, and the resources. The image is `work/<name>.exe`.
    pub fn link_rustc_output(&self, project: &Project, link: &RustcLink) -> Result<Vec<Artifact>> {
        std::fs::create_dir_all(&link.work).map_err(|e| file_error(&link.work, e))?;
        let cwd = RemotePath::new(arg(&project.root));
        let lld = match &self.linker {
            RustLinker::Lld { rust_lld, cache } => {
                Some((self.rust_lld_ready(rust_lld.as_deref(), &cwd)?, cache))
            }
            RustLinker::Gnu => None,
        };
        let prebuilt = self.linker.prebuilt(&self.sdk)?;
        let shims = self.shim_archives(project, &cwd, prebuilt.as_ref(), &link.work)?;
        self.run_cargo_args(&self.libcalls().cargo_args(), &cwd)?;
        let libcalls = produced(self.libcalls().path(project), "the Rust SDK's symbian-libcalls crate defines the __atomic_* family and memcmp")?;
        let elf = link.work.join(format!("{}.elf", self.name));
        let map = link.work.join(format!("{}.exe.map", self.name));
        match lld {
            None => self.gcce.run_tool(
                &self.link_args(&link.inputs, shims.first().map(PathBuf::as_path), Some(&libcalls), &elf, &map)?,
                &cwd,
            )?,
            Some((rust_lld, cache)) => self.link_lld(
                &rust_lld, cache, prebuilt.as_ref(),
                &LinkInputs { rust: &link.inputs, shims: &shims, libcalls: &libcalls },
                &elf, &map, &cwd,
            )?,
        }
        RequiredCapability::check(&elf, &self.gcce.capabilities, &format!("{}.exe", self.name))?;
        let out = link.work.join(format!("{}.exe", self.name));
        self.gcce.run_elf2e32(&self.gcce.elf2e32_args(&self.name, &elf, &out), &cwd)?;
        let mut artifacts = vec![Artifact::exe(out)];
        artifacts.extend(self.build_ui(&link.work)?);
        artifacts.extend(self.build_strings(&project.root, &link.work)?);
        Ok(artifacts)
    }
}
```

Make `run_cargo_args` `pub(super)` if it is private. Export: `pub use driver::RustcLink;`
in `lib.rs`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p symdev-build --offline`
Expected: all pass, including every pre-existing link test after the `&[a.into()]` call
changes (the GNU line tests pin that a one-element slice is 0.3.0's line exactly).

- [ ] **Step 5: Check 0.3.0's images did not move**

```bash
S=~/src/cargo-run-scratch; mkdir -p $S/t2 && git archive HEAD | tar -x -C $S/t2
cargo build --release --offline -p symdev-cli --target-dir $S/target-t2 && cp $S/target-t2/release/symdev $S/bin/symdev-t2
. $S/env.sh; export SYMDEV_RUST_SDK=$S/t2/symbian-rs
for ex in hello ui async; do for b in symdev-030 symdev-t2; do
  (cd $S/t2/symbian-rs/examples/$ex && $S/bin/$b build >/dev/null && cp build/*.exe $S/t2/$ex.$b.exe); done
  python3 $S/e32cmp.py $S/t2/$ex.symdev-030.exe $S/t2/$ex.symdev-t2.exe; done
```

Expected: `EQUAL` three times (both binaries build the same tree at the same path).

- [ ] **Step 6: Commit**

```bash
git add crates/symdev-build/src
git commit -m "Link rustc's objects and rlibs where the staticlib stood, each link in its own directory."
```

### Task 3: Packaging as a type, resources beside the image — and D1

**Files:**
- Create: `crates/symdev-cli/src/sisx.rs` (`ProjectPackage`)
- Modify: `crates/symdev-cli/src/main.rs` (`package_project` becomes a call to `ProjectPackage`)
- Modify: `crates/symdev-cli/src/artifacts.rs` (resources from the image's directory, not `cwd/build`)
- Modify (step 6, D1 = A only): `crates/symdev-build/src/package.rs`, `crates/symdev-build/src/package/tests.rs`, `crates/symdev-cli/tests/package.rs`
- Test: `crates/symdev-cli/src/sisx.rs` (`#[cfg(test)] mod tests` at its end, the file stays ≤ 300 lines)

**Interfaces:**
- Consumes: nothing.
- Produces: `ProjectPackage::new(manifest: Manifest, root: PathBuf, epocroot: PathBuf) ->
  Result<ProjectPackage>`, `ProjectPackage::app(&self) -> &str`, and
  `ProjectPackage::package(&self, exe: &Path, password: &str) -> Result<PathBuf>`. `exe`
  must be named `<app>.exe`; its directory holds the resources and receives
  `<name>.sis`/`<name>.sisx`. Returns the `.sisx` path.

- [ ] **Step 1: Write the failing test** — at the end of `sisx.rs`

```rust
#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::artifacts::package_artifacts;

    #[test]
    fn resources_are_taken_from_beside_the_image() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("out/app.symdev");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::write(work.join("app.exe"), b"E32").unwrap();
        let project = symdev_core::Project { root: dir.path().to_path_buf() };
        let got = package_artifacts(&project, &work.join("app.exe"), None, &[], &[],
                                    Path::new(""), None).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, work.join("app.exe"));
    }
}
```

A second assertion covers a `[ui]` project. Build the `UiResources` the way
`build_cmd.rs` does (app `"app"`, uid3 `0xe1234567`, `symdev_manifest::UiApp` with
`kind = Avkon`, caption `"App"`, softkeys `OptionsExit`). Write the three resource files that
`UiResources::artifacts(&work)` names into `work`, and assert every returned path starts with
`work`.

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p symdev-cli --offline sisx::tests`
Expected: FAIL: the `[ui]` assertion sees `<root>/build/…` paths, because `package_artifacts`
joins `cwd.join("build")`.

- [ ] **Step 3: Implement**

In `artifacts.rs`, replace each `cwd.join("build")` with `build`, computed once at the top:

```rust
    let build = cwd.join(e32).parent().map(Path::to_path_buf).ok_or_else(|| {
        Error::Other(format!("{}: the image has no directory", e32.display()))
    })?;
```

`symdev package` passes `build/<app>.exe`, so its behaviour is unchanged.

`sisx.rs` carries the body of `main.rs`'s `package_project`, from `let uid3 = …` to the
`.package(&package_artifacts(…))` call, as a method:

```rust
//! `ProjectPackage`: a project's `.sisx` from an E32 image and the resources beside it.
use std::path::{Path, PathBuf};

use symdev_build::{AppTarget, SisPackage, UiResources};
use symdev_core::{Error, PackageBackend, Project, Result};
use symdev_manifest::Manifest;

use crate::artifacts::package_artifacts;

pub(crate) struct ProjectPackage {
    manifest: Manifest,
    root: PathBuf,
    epocroot: PathBuf,
    app: String,
    uid3: u32,
}

impl ProjectPackage {
    pub fn new(manifest: Manifest, root: PathBuf, epocroot: PathBuf) -> Result<Self> {
        let uid3 = manifest.symbian.uid3.ok_or_else(|| {
            Error::Other("uid3 required for package (set symbian.uid3)".into())
        })?;
        let project = Project { root: root.clone() };
        let app = AppTarget::of(&project, &manifest.package.name, &epocroot)?.name().to_string();
        Ok(Self { manifest, root, epocroot, app, uid3 })
    }

    pub fn app(&self) -> &str { &self.app }

    pub fn package(&self, exe: &Path, password: &str) -> Result<PathBuf> {
        let m = &self.manifest;
        let project = Project { root: self.root.clone() };
        // … the rest of package_project's body, unchanged, with `cwd` → `self.root`,
        // `e32` → `exe`, `app.name()` → `self.app`, `password` → `password.to_string()`,
        // `m.<field>` cloned where it was moved …
        Ok(package.primary)
    }
}
```

`main.rs`'s `package_project` keeps its `e32.is_file()` check and its printing. In between:

```rust
    let package = ProjectPackage::new(m, cwd.clone(), epocroot)?;
    let e32 = cwd.join("build").join(format!("{}.exe", package.app()));
    // … the existing is_file check on e32 …
    let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default();
    println!("{}", package.package(&e32, &password)?.display());
```

Add `mod sisx;` to `main.rs`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p symdev-cli --offline`
Expected: all pass, including `tests/package.rs` unchanged. `main.rs` shrinks by about 60
lines.

- [ ] **Step 5: Commit**

```bash
git add crates/symdev-cli/src/sisx.rs crates/symdev-cli/src/main.rs crates/symdev-cli/src/artifacts.rs
git commit -m "Package a project from an image and the resources beside it, as a type."
```

- [ ] **Step 6 (needs D1; Option A shown): Ask for the password only for an encrypted key of the user's**

Tests first, in `crates/symdev-build/src/package/tests/validation.rs` (a child of
`package/tests.rs`, whose helpers `fake_pkg()` and `hello_exe_bytes()` it reaches as
`super::`):

```rust
#[test]
fn a_generated_self_signed_pair_needs_no_password() {
    let dir = tempfile::tempdir().unwrap();
    let pkg = SisPackage { password: String::new(), ..super::fake_pkg() };
    let exe = dir.path().join("hello.exe");
    std::fs::write(&exe, super::hello_exe_bytes()).unwrap();
    assert!(pkg.package(&[symdev_core::Artifact::exe(exe)]).is_ok());
}

#[test]
fn an_encrypted_key_of_the_users_still_needs_four_characters() {
    let dir = tempfile::tempdir().unwrap();
    let (cer, key) = (dir.path().join("a.cer"), dir.path().join("a.key"));
    std::fs::write(&cer, "-----BEGIN CERTIFICATE-----\n").unwrap();
    std::fs::write(&key, "-----BEGIN DSA PRIVATE KEY-----\nProc-Type: 4,ENCRYPTED\n").unwrap();
    let pkg = SisPackage { password: "ab".into(), cert: Some(cer), key: Some(key), ..super::fake_pkg() };
    let exe = dir.path().join("hello.exe");
    std::fs::write(&exe, super::hello_exe_bytes()).unwrap();
    let e = pkg.package(&[symdev_core::Artifact::exe(exe)]).unwrap_err().to_string();
    assert!(e.contains("at least 4 characters"), "{e}");
}
```

Implement in
`SisPackage::package`: replace `self.validate_password()?;` with

```rust
        if let Some((_, key)) = self.existing_signing_pair() {
            let pem = std::fs::read(&key).map_err(|e| Error::Other(format!("{}: {e}", key.display())))?;
            let text = String::from_utf8_lossy(&pem);
            if text.contains("Proc-Type: 4,ENCRYPTED") || text.contains("BEGIN ENCRYPTED PRIVATE KEY") {
                self.validate_password()?;
            }
        }
```

The doc comment cites experiment 114 §1.7: the original `makekeys` allows an unencrypted key
and `signsis` signs with it with no pass phrase. Change `tests/package.rs`'s
`package_missing_sign_password` (and any test in `package/tests/validation.rs` that expects
the old refusal for a generated pair) into `package_without_a_password_signs_with_a_generated_pair`
(expects success and a `.sisx`). Run `cargo test --workspace --offline` and commit: "Ask for
a signing password only for an encrypted key the project supplies."

For D1 = B or C, replace this step with that option's behaviour from the D1 table, test
first, before Task 4.

### Task 4: The `symdev-ld` role and `symdev setup-linker`

**Files:**
- Create: `crates/symdev-cli/src/role.rs` (`Role`)
- Create: `crates/symdev-cli/src/rust_project.rs` (`RustProject`: the provisioning `build_cmd.rs` does for a Rust project, reusable)
- Create: `crates/symdev-cli/src/ld/link_record.rs` (`LinkRecord`), `crates/symdev-cli/src/ld/link_run.rs` (`LinkRun`)
- Create: `crates/symdev-cli/src/setup_linker.rs`
- Modify: `crates/symdev-cli/src/main.rs` (role dispatch first; `Commands::SetupLinker`), `crates/symdev-cli/src/cli.rs`, `crates/symdev-cli/src/build_cmd.rs` (uses `RustProject`), `crates/symdev-cli/src/ld.rs`
- Modify: `crates/symdev-cli/Cargo.toml` (`toml = "1"`, already in `Cargo.lock`)
- Test: `crates/symdev-cli/tests/ld.rs`, `crates/symdev-cli/tests/setup_linker.rs`, `crates/symdev-cli/src/ld/tests.rs`

**Interfaces:**
- Consumes: Task 1 (`LinkerArgs`, `CargoLinkEnv`, `LinkKind`, `CargoOutput`), Task 2
  (`RustcLink`, `RustBuild::link_rustc_output`), Task 3 (`ProjectPackage`).
- Produces:
  - `Role::of(argv0: &OsStr) -> Role`, where `enum Role { Cli, Linker, Rustc }`. The
    names are `symdev-ld` and `symdev-rustc` (file stem); anything else is `Cli`.
  - `LinkRecord { pub kind: LinkKind }` with `LinkRecord::write(&self, path: &Path) ->
    Result<()>` and `LinkRecord::read(path: &Path) -> Result<LinkRecord>`. The file is TOML:
    `kind = "main"`, or `kind = "test"` and `name = "<test>"`.
  - The files one link leaves: `<out>` (the E32 image), `<out>.sisx`, `<out>.symdev.toml`
    and `<out>.symdev/` (work). The main binary also leaves `<profile-dir>/<bin>.sisx`,
    `<profile-dir>/<bin>.symdev.toml`, and `build/<name>.exe` + `build/<name>.sisx` in the
    project, as 0.3.0 did.
  - `RustProject::resolve(manifest: &Manifest, root: &Path, provision: &Provision, ui: bool)
    -> Result<RustProject>`, with `pub build: RustBuild` and `pub epocroot: PathBuf`.

- [ ] **Step 1: Write the failing tests**

`crates/symdev-cli/tests/ld.rs` drives the real binary through a link named `symdev-ld`:

```rust
use std::path::{Path, PathBuf};
use std::process::Command;

fn symdev_ld(dir: &Path) -> PathBuf {
    let link = dir.join("symdev-ld");
    std::os::unix::fs::symlink(assert_cmd::cargo::cargo_bin("symdev"), &link).unwrap();
    link
}

fn fixture(name: &str) -> Vec<String> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ld/testdata").join(name);
    std::fs::read_to_string(p).unwrap().lines().map(String::from).collect()
}

fn run(dir: &Path, envs: &[(&str, &Path)], args: &[String]) -> (bool, String) {
    let mut cmd = Command::new(symdev_ld(dir));
    cmd.args(args).env_remove("CARGO_MANIFEST_DIR").env_remove("CARGO_BIN_NAME")
        .env_remove("CARGO_TARGET_TMPDIR");
    for (k, v) in envs { cmd.env(k, v); }
    let out = cmd.output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

#[test]
fn symdev_ld_outside_cargo_says_it_is_cargos_linker() {
    let dir = tempfile::tempdir().unwrap();
    let (ok, err) = run(dir.path(), &[], &fixture("release-bin.argv"));
    assert!(!ok);
    assert!(err.contains("CARGO_MANIFEST_DIR") && err.contains("cargo's linker"), "{err}");
}

#[test]
fn symdev_ld_without_symdev_toml_names_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("app");
    std::fs::create_dir(&project).unwrap();
    let (ok, err) = run(dir.path(), &[("CARGO_MANIFEST_DIR", &project),
                                     ("CARGO_BIN_NAME", Path::new("app"))],
                        &fixture("release-bin.argv"));
    assert!(!ok);
    assert!(err.contains(&project.display().to_string()) && err.contains("symdev.toml"), "{err}");
}

#[test]
fn symdev_ld_refuses_an_unseen_argument_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let mut args = fixture("release-bin.argv");
    args.push("--eh-frame-hdr".into());
    let (ok, err) = run(dir.path(), &[], &args);
    assert!(!ok && err.contains("`--eh-frame-hdr`"), "{err}");
}
```

`crates/symdev-cli/tests/setup_linker.rs`:

```rust
#[test]
fn setup_linker_links_both_roles_to_this_binary() {
    let dir = tempfile::tempdir().unwrap();
    let bin = assert_cmd::cargo::cargo_bin("symdev");
    let ok = std::process::Command::new(&bin).args(["setup-linker", "--dir"]).arg(dir.path())
        .status().unwrap().success();
    assert!(ok);
    for role in ["symdev-ld", "symdev-rustc"] {
        assert_eq!(std::fs::read_link(dir.path().join(role)).unwrap(), bin);
    }
    // A second run accepts its own links and changes nothing.
    assert!(std::process::Command::new(&bin).args(["setup-linker", "--dir"]).arg(dir.path())
        .status().unwrap().success());
}
```

Append to `crates/symdev-cli/src/ld/tests.rs` (Review Focus 1):

```rust
#[test]
fn two_links_of_one_project_use_two_work_dirs() {
    let bin = super::LinkRun::work_for(&LinkerArgs::parse(argv("release-bin.argv")).unwrap()).unwrap();
    let test = super::LinkRun::work_for(&LinkerArgs::parse(argv("release-test.argv")).unwrap()).unwrap();
    assert_ne!(bin, test);
    assert!(bin.ends_with("out/app.symdev") && test.ends_with("out/smoke-4f71ac116363b7c1.symdev"));
}

#[test]
fn a_link_record_says_main_or_names_the_test() {
    let dir = tempfile::tempdir().unwrap();
    for kind in [LinkKind::Main, LinkKind::Test { name: "smoke".into() }] {
        let p = dir.path().join("r.toml");
        super::LinkRecord { kind: kind.clone() }.write(&p).unwrap();
        assert_eq!(super::LinkRecord::read(&p).unwrap().kind, kind);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test -p symdev-cli --offline --test ld --test setup_linker` and `cargo test -p
symdev-cli --offline ld::tests`
Expected: the `symdev-ld` link starts the normal CLI, which prints clap's usage, so the
assertions fail; `setup-linker` is an unknown subcommand; `LinkRun` and `LinkRecord` are
not found.

- [ ] **Step 3: Implement**

`role.rs`:

```rust
//! `Role`: one binary, three programs, told apart by the name it is started under
//! (design spec §4: `install.sh` and `symdev setup-linker` make the links).
use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role { Cli, Linker, Rustc }

impl Role {
    pub fn of(argv0: &OsStr) -> Self {
        match Path::new(argv0).file_stem().and_then(OsStr::to_str) {
            Some("symdev-ld") => Self::Linker,
            Some("symdev-rustc") => Self::Rustc,
            _ => Self::Cli,
        }
    }
}
```

`main.rs`: `main` starts with

```rust
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    match role::Role::of(&argv0) {
        role::Role::Linker => return exit(ld::LinkRun::from_env(args).and_then(|r| r.run())),
        role::Role::Rustc => return exit(Err(Error::Other(
            "symdev-rustc: TODO: the rust-std wrapper arrives with Task 9 (not observed)".into()))),
        role::Role::Cli => {}
    }
```

with `fn exit(r: Result<(), Error>) -> ExitCode` printing `error: {e}` and returning 1 (the
same shape as the existing tail of `main`; reuse it there too). Remove Task 1's
`#[allow(dead_code)]`.

`rust_project.rs` moves `build_cmd.rs`'s Rust half (`provision.rust_sdk()`, `rust_linker()`,
`needs_gcce`, `prebuilt_note`, `toolchain`, the `UiResources` with locales, `GcceBuild`,
`RustBuild { … }`) into `RustProject::resolve(m, root, provision, ui)`. `ui = false` leaves
`RustBuild::ui` as `None`: a test binary is a console program even in an Avkon project
(spec §4.5). `build_cmd.rs` calls it with `ui = true`; behaviour unchanged.

`ld/link_record.rs`:

```rust
//! `LinkRecord`: `<out>.symdev.toml`, what `symdev-ld` tells the runner about an image.
use std::path::Path;

use symdev_core::{Error, Result};

use super::LinkKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkRecord { pub kind: LinkKind }

impl LinkRecord {
    pub fn write(&self, path: &Path) -> Result<()> {
        let text = match &self.kind {
            LinkKind::Main => "kind = \"main\"\n".to_string(),
            LinkKind::Test { name } => format!("kind = \"test\"\nname = {:?}\n", name),
        };
        std::fs::write(path, text).map_err(|e| Error::Other(format!("{}: {e}", path.display())))
    }

    pub fn read(path: &Path) -> Result<Self> {
        let bad = |why: &str| Error::Other(format!("{}: {why}; relink with cargo build", path.display()));
        let text = std::fs::read_to_string(path).map_err(|e| bad(&e.to_string()))?;
        let table: toml::Table = text.parse().map_err(|e: toml::de::Error| bad(&e.to_string()))?;
        match (table.get("kind").and_then(|v| v.as_str()), table.get("name").and_then(|v| v.as_str())) {
            (Some("main"), _) => Ok(Self { kind: LinkKind::Main }),
            (Some("test"), Some(name)) => Ok(Self { kind: LinkKind::Test { name: name.into() } }),
            _ => Err(bad("not a symdev-ld record")),
        }
    }
}
```

`ld/link_run.rs`:

```rust
//! `LinkRun`: one `symdev-ld` call — link, package, and leave the files cargo and the
//! runner look for (design spec §4; experiment 114 §1).
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use symdev_build::RustcLink;
use symdev_core::{Error, Project, Result};

use super::{CargoLinkEnv, CargoOutput, LinkKind, LinkRecord, LinkerArgs};
use crate::provision::Provision;
use crate::rust_project::RustProject;
use crate::sisx::ProjectPackage;

pub(crate) struct LinkRun { args: LinkerArgs, env: CargoLinkEnv }

impl LinkRun {
    pub fn from_env(args: impl Iterator<Item = OsString>) -> Result<Self> {
        let args = LinkerArgs::parse(args)?;
        Ok(Self { args, env: CargoLinkEnv::from_pairs(std::env::vars()) })
    }

    /// This link's own directory, beside rustc's output.
    pub fn work_for(args: &LinkerArgs) -> Result<PathBuf> {
        Ok(CargoOutput::of(&args.output)?.work_dir())
    }

    pub fn run(&self) -> Result<()> {
        let root = self.env.manifest_dir.clone().ok_or_else(|| Error::Other(
            "symdev-ld: CARGO_MANIFEST_DIR is not set: symdev-ld is cargo's linker, named in \
             .cargo/config.toml; run `cargo build`".into()))?;
        let manifest = symdev_manifest::load(&root.join("symdev.toml")).map_err(|e| Error::Other(format!(
            "symdev-ld: no usable symdev.toml in {} ({e}); `symdev new --lang rust` makes one",
            root.display())))?;
        if !manifest.language.is_rust() {
            return Err(Error::Other(format!("symdev-ld: {} is not a Rust project", root.display())));
        }
        let kind = LinkKind::of(&self.env, &manifest.package.name)?;
        let out = CargoOutput::of(&self.args.output)?;
        for dir in &self.args.raw_dylibs {
            if std::fs::read_dir(dir).is_ok_and(|mut d| d.next().is_some()) {
                return Err(Error::Other(format!("symdev-ld: rustc's {} is not empty: TODO: \
                    raw-dylib imports (not observed)", dir.display())));
            }
        }
        let provision = Provision::from_env(false);
        let rust = RustProject::resolve(&manifest, &root, &provision, kind == LinkKind::Main)?;
        let link = RustcLink { inputs: self.args.inputs.clone(), work: out.work_dir() };
        let artifacts = rust.build.link_rustc_output(&Project { root: root.clone() }, &link)?;
        let exe = &artifacts.first().ok_or_else(|| Error::Other("symdev-ld: no image".into()))?.path;
        let package = ProjectPackage::new(manifest.clone(), root.clone(), rust.epocroot.clone())?;
        let password = std::env::var("SYMDEV_SIGN_PASSWORD").unwrap_or_default(); // per D1
        let sisx = package.package(exe, &password)?;
        copy(exe, out.path())?;
        copy(&sisx, &out.sisx())?;
        LinkRecord { kind: kind.clone() }.write(&out.record())?;
        if kind == LinkKind::Main {
            let bin = out.path().file_name().map(PathBuf::from).unwrap_or_default();
            let beside = out.profile_dir().join(&bin);
            copy(&sisx, &beside.with_extension("sisx"))?;
            LinkRecord { kind }.write(&PathBuf::from(format!("{}.symdev.toml", beside.display())))?;
            let build = root.join("build");
            copy(exe, &build.join(format!("{}.exe", package.app())))?;
            copy(&sisx, &build.join(format!("{}.sisx", manifest.package.name)))?;
        }
        Ok(())
    }
}

fn copy(from: &Path, to: &Path) -> Result<()> {
    std::fs::copy(from, to).map(|_| ()).map_err(|e| Error::Other(format!(
        "symdev-ld: copy {} to {}: {e}", from.display(), to.display())))
}
```

(`<bin>` has no extension, so `beside.with_extension("sisx")` is `<profile-dir>/<bin>.sisx`,
exactly `<exe>.sisx` for the path `cargo run` hands the runner.) Export `LinkRecord` and
`LinkRun` from `ld.rs`.

`setup_linker.rs`: `pub(crate) fn setup_linker(dir: Option<PathBuf>) -> Result<ExitCode>`.
For `symdev-ld` and `symdev-rustc` in `dir` (default: the directory of
`std::env::current_exe()`): an existing link to `current_exe()` is kept; anything else there
is an error naming it; otherwise `std::os::unix::fs::symlink(current_exe, …)`. Print each
link. If `dir` is not on `PATH`, print `note: <dir> is not on PATH; cargo looks the linker
up there`. `cli.rs` gains

```rust
    /// Make the `symdev-ld` and `symdev-rustc` links cargo starts (design spec §4).
    SetupLinker {
        /// Where to put them; default: beside this symdev.
        #[arg(long)]
        dir: Option<std::path::PathBuf>,
    },
```

and `main.rs` dispatches it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p symdev-cli --offline`
Expected: all pass.

- [ ] **Step 5: A real link** (needs the GCCE route environment of experiment 114)

```bash
S=~/src/cargo-run-scratch; cargo build --release --offline -p symdev-cli
mkdir -p $S/bin4 && ./target/release/symdev setup-linker --dir $S/bin4
. $S/env.sh; cd $S/tree/symbian-rs/examples/hello && touch src/main.rs
env -u RUSTUP_TOOLCHAIN PATH=$S/bin4:$PATH SYMDEV_SIGN_PASSWORD=scratch cargo build --release
python3 $S/e32cmp.py build/cargo/arm-symbian-e32/release/hello $S/out/q2/hello.exe
ls build/cargo/arm-symbian-e32/release/hello.sisx build/hello.sisx
```

(`tree/` holds the spike's bin-shape `hello`. Its `symbian-rs/.cargo/config.toml` names
`linker = "symdev-ld"`, which `$S/bin4` now resolves to this build.)
Expected: `EQUAL 975/1348 vs 975/1348`, and both `.sisx` files exist.

- [ ] **Step 6: Commit**

```bash
git add crates/symdev-cli
git commit -m "Run symdev as cargo's linker under the name symdev-ld, and make its links with setup-linker."
```

### Task 5: The target spec links executables, and UID3 comes from `symdev.toml` at compile time

**Files:**
- Modify: `symbian-rs/targets/arm-symbian-e32.json`
- Create: `symbian-rs/crates/symbian-macros/src/manifest_uid3.rs` (`ManifestUid3`)
- Modify: `symbian-rs/crates/symbian-macros/src/lib.rs` (`uid3!`), `symbian-rs/crates/symbian-std/src/lib.rs` (re-export, `report!`)
- Modify: `symbian-rs/crates/symbian-std/src/test_report/mod.rs` (delete `Report::new`, `uid3_from_env`, `parse_hex_u32`)
- Modify: the 15 callers of `Report::new` (`symbian-rs/examples/{fmt,ui,async,query,files,net,tls,notes,std-net,locale,std-hello,ui-list,atomics,cleanup,time}/src/main.rs`)
- Modify: `crates/symdev-build/src/driver/rust_build.rs` (drop `cmd.env("SYMDEV_UID3", …)`)
- Test: `symbian-rs/crates/symbian-macros/src/manifest_uid3.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: nothing.
- Produces: `symbian_std::uid3!()` (a `u32` literal from `$CARGO_MANIFEST_DIR/symdev.toml`'s
  `[symbian] uid3`), and `symbian_std::report!("app")`, which expands to
  `symbian_std::test_report::Report::with_uid3("app", symbian_std::uid3!())`. Target spec
  keys `"executables": true` and `"default-visibility": "hidden"`.

- [ ] **Step 1: Write the failing tests** — at the end of `manifest_uid3.rs`

```rust
#[cfg(test)]
mod tests {
    use super::ManifestUid3;

    const SCAFFOLD: &str = "[package]\nname = \"hello\"\nversion = \"0.1.0\"\n\n[target]\n\
        device = \"nokia-e52\"\n\n[language]\nname = \"rust\"\n\n[symbian]\nuid3 = \"0xef9f2cab\"\n\
        capabilities = []\nvendor = \"symdev\"\n\n[signing]\nmode = \"self-signed\"\n";

    #[test]
    fn the_scaffolds_uid3_is_read() {
        assert_eq!(ManifestUid3::parse(SCAFFOLD), Ok(0xef9f_2cab));
    }

    #[test]
    fn a_uid3_outside_the_symbian_table_does_not_count() {
        let text = "[package]\nuid3 = \"0x1\"\n[symbian]\nvendor = \"x\"\n";
        assert!(ManifestUid3::parse(text).unwrap_err().contains("[symbian] uid3"));
    }

    #[test]
    fn a_uid3_that_is_not_quoted_hex_is_refused() {
        for bad in ["uid3 = 0xe1", "uid3 = \"e1\"", "uid3 = \"0xZZ\"", "uid3 = \"0x123456789\""] {
            let text = format!("[symbian]\n{bad}\n");
            assert!(ManifestUid3::parse(&text).is_err(), "{bad}");
        }
    }

    #[test]
    fn spaces_and_comments_around_it_are_fine() {
        assert_eq!(ManifestUid3::parse("[symbian]\n  uid3=\"0xE0000687\"  # app\n"), Ok(0xe000_0687));
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo +nightly-2026-09-19 test --offline --manifest-path symbian-rs/crates/symbian-macros/Cargo.toml`
(from the repository root, so `symbian-rs/.cargo/config.toml` does not apply and the tests
build for the host). Expected: `ManifestUid3` not found.

- [ ] **Step 3: Implement**

`manifest_uid3.rs`, dependency-free like the rest of `symbian-macros` (its `Cargo.toml`
says why):

```rust
//! `ManifestUid3`: `[symbian] uid3` of the application's `symdev.toml`, read while the
//! application compiles. A proc macro runs in the rustc that compiles the application, so
//! its `CARGO_MANIFEST_DIR` is the application's; a dependency's build script would see
//! its own (experiment 114 §1, design spec §3).
use std::path::Path;

pub struct ManifestUid3;

impl ManifestUid3 {
    pub fn read(dir: &Path) -> Result<u32, String> {
        let path = dir.join("symdev.toml");
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("`symbian_std::uid3!()` reads {}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The scaffold's shape: a `[symbian]` table with `uid3 = "0x<1–8 hex digits>"`.
    pub fn parse(text: &str) -> Result<u32, String> {
        let mut in_symbian = false;
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.starts_with('[') {
                in_symbian = line == "[symbian]";
                continue;
            }
            let Some((key, value)) = line.split_once('=') else { continue };
            if !in_symbian || key.trim() != "uid3" {
                continue;
            }
            let hex = value.trim().strip_prefix("\"0x").or_else(|| value.trim().strip_prefix("\"0X"))
                .and_then(|v| v.strip_suffix('"'))
                .filter(|h| (1..=8).contains(&h.len()))
                .ok_or_else(|| format!("[symbian] uid3 must be a quoted hex number like \"0xe0000687\", not {}", value.trim()))?;
            return u32::from_str_radix(hex, 16).map_err(|e| format!("[symbian] uid3 {hex}: {e}"));
        }
        Err("no [symbian] uid3 (symdev new writes one)".into())
    }
}
```

`lib.rs` of `symbian-macros`:

```rust
mod manifest_uid3;

/// The application's UID3 from its `symdev.toml`, as a `u32` literal.
#[proc_macro]
pub fn uid3(input: TokenStream) -> TokenStream {
    if !input.is_empty() {
        return tokens(&compile_error("`symbian_std::uid3!()` takes no arguments"));
    }
    let Some(dir) = std::env::var_os("CARGO_MANIFEST_DIR") else {
        return tokens(&compile_error("`symbian_std::uid3!()` needs CARGO_MANIFEST_DIR, which cargo sets"));
    };
    match manifest_uid3::ManifestUid3::read(std::path::Path::new(&dir)) {
        Ok(uid3) => tokens(&format!("0x{uid3:08x}_u32")),
        Err(message) => tokens(&compile_error(&message)),
    }
}
```

`symbian-std/src/lib.rs`: `pub use symbian_macros::{main, strings, uid3};` and

```rust
/// A test report named by the application's own UID3 (`symdev.toml`), so the file
/// `symdev test` waits for is the one the application writes.
#[macro_export]
macro_rules! report {
    ($app:expr) => {
        $crate::test_report::Report::with_uid3($app, $crate::uid3!())
    };
}
```

In `test_report/mod.rs` delete `Report::new`, `uid3_from_env` and `parse_hex_u32` with their
doc comments and tests, and change the doc example to `symbian_std::report!("files")`. In
each of the 15 examples replace `Report::new("<app>")` by `symbian_std::report!("<app>")`, and
drop the `Report` import where nothing else uses it.

`arm-symbian-e32.json`: set `"executables": true`; add `"default-visibility": "hidden",`
after `"default-uwtable": false,` (keys stay sorted); set `metadata.description` to `"Symbian
OS 9.3 EKA2 (S60 3rd FP2) user-side E32 code, ARMv5TE soft-float EABI; cargo links through
symdev-ld"`.

`rust_build.rs`: delete `cmd.env("SYMDEV_UID3", …)` and its comment. No crate reads the
variable any more.

- [ ] **Step 4: Run the tests and rebuild the examples**

Run the macro tests (Step 2's command): all pass. Then `cargo test --workspace --offline`:
the host workspace still builds the examples through `symdev build`'s tests where it does.
Then check the bytes. The release images must equal experiment 114 §1.5's `out/q5d` set:
the same spec keys, and `report!` gives `with_uid3` the constant `new` used to compute. Run
`~/src/cargo-run-scratch/base.sh` style builds of `async`, `files`, `atomics` with the
branch's `symdev` (staticlib shape still; Task 8 converts) and `e32cmp.py` them against
`~/src/cargo-run-scratch/out/q5d/`. Expected: `EQUAL` ×3. A difference is a finding: record
it in `docs/research/wip/cargo-run.md` and explain it before going on.

- [ ] **Step 5: Commit**

```bash
git add symbian-rs/targets/arm-symbian-e32.json symbian-rs/crates/symbian-macros/src \
  symbian-rs/crates/symbian-std/src symbian-rs/examples/*/src/main.rs crates/symdev-build/src/driver/rust_build.rs
git commit -m "Let rustc link executables for the target and read UID3 from symdev.toml at compile time."
```

### Task 6: `symbian-test`, the harness of a `harness = false` test

**Files:**
- Create: `symbian-rs/crates/symbian-test/{Cargo.toml,src/lib.rs,src/evidence.rs}`
- Create: `symbian-rs/crates/symbian-macros/src/test_module.rs` (`TestModule`, the `#[symbian_test::tests]` expansion)
- Modify: `symbian-rs/crates/symbian-macros/src/lib.rs` (`#[proc_macro_attribute] pub fn tests`)
- Modify: `symbian-rs/crates/symbian-std/src/test_report/{mod.rs,json.rs}` (case `state`: `pending`, `running`)
- Modify: `symbian-rs/Cargo.toml` (member `crates/symbian-test`)
- Modify: `crates/symdev-sdk/src/rust_sdk_package.rs` and `~/projects/symdev-packages/recipes/symdev/0.4.0/recipe.toml` (Task 17) only if the recipe lists crates one by one; check with `grep -n symbian-ui` in both
- Test: `symbian-rs/crates/symbian-macros/src/test_module.rs` (`#[cfg(test)]`)

**Interfaces:**
- Consumes: Task 5 (`symbian_std::uid3!`).
- Produces (device side):
  - The attribute `#[symbian_test::tests]` on `mod <m> { … }`. It strips `#[test]` from each
    `fn` directly in the module, adds `pub(super) const __SYMBIAN_TESTS:
    &[::symbian_test::Case]` inside it, and writes `E32Main` beside it. That `E32Main` runs
    `::symbian_std::__start(|| ::symbian_test::__run(env!("CARGO_CRATE_NAME"),
    ::symbian_std::uid3!(), <m>::__SYMBIAN_TESTS))`.
  - `pub struct Case { pub name: &'static str, pub run: fn() -> Result<(), Evidence> }`.
  - `pub struct Evidence`, with `impl<E: symbian_std::test_report::Evidence> From<E>` (so `?`
    works on any error the report can show), `Evidence::msg(&str)`, and `ensure(ok: bool, what:
    &str) -> Result<(), Evidence>`.
  - The report file, schema 1 unchanged, gains an optional `"state"` on a case:
    `"pending"` (written for every case before the first runs) and `"running"` (written just
    before that case runs). A finished case has no `state`. symdev's reader ignores unknown
    fields today, so old readers still read the file.

- [ ] **Step 1: Write the failing tests** — end of `test_module.rs`

```rust
#[cfg(test)]
mod tests {
    use super::TestModule;

    fn expand(src: &str) -> Result<String, String> {
        TestModule::expand(src)
    }

    #[test]
    fn test_attributes_are_stripped_and_listed_in_order() {
        let out = expand("mod checks { #[test] fn first() -> R { Ok(()) } fn helper() {} \
                          #[test] fn second() -> R { Ok(()) } }").unwrap();
        assert!(!out.contains("#[test]"), "{out}");
        let (a, b) = (out.find("\"first\"").unwrap(), out.find("\"second\"").unwrap());
        assert!(a < b && !out.contains("\"helper\""), "{out}");
        assert!(out.contains("__SYMBIAN_TESTS") && out.contains("_Z7E32Mainv"), "{out}");
        assert!(out.contains("checks::__SYMBIAN_TESTS"), "{out}");
    }

    #[test]
    fn a_module_with_no_test_is_an_error() {
        assert!(expand("mod m { fn f() {} }").unwrap_err().contains("no `#[test]` fn"));
    }

    #[test]
    fn anything_but_an_inline_module_is_an_error() {
        assert!(expand("fn f() {}").unwrap_err().contains("`mod <name> { … }`"));
        assert!(expand("mod m;").unwrap_err().contains("`mod <name> { … }`"));
    }
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo +nightly-2026-09-19 test --offline --manifest-path symbian-rs/crates/symbian-macros/Cargo.toml`
Expected: `TestModule` not found.

- [ ] **Step 3: Implement**

`test_module.rs` works on the item's text, like `entry.rs` (`Entry::parse` takes
`item.to_string()`), so its unit tests run outside a macro expansion. A small scanner skips
string and char literals and accepts any spacing rustc prints between `#`, `[`, `test` and
`]`:

```rust
//! `#[symbian_test::tests]`: a module of `#[test] fn`s becomes a program (design spec §7).
pub struct TestModule;

const SHAPE: &str = "`#[symbian_test::tests]` goes on an inline `mod <name> { … }`";

impl TestModule {
    /// The expanded source for `item` (the module's text), or the message for
    /// `compile_error!`.
    pub fn expand(item: &str) -> Result<String, String> {
        let rest = item.trim_start();
        let after_mod = rest.strip_prefix("pub ").unwrap_or(rest).trim_start()
            .strip_prefix("mod").ok_or_else(|| SHAPE.to_string())?;
        let name: String = after_mod.trim_start().chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        let open = item.find('{').ok_or_else(|| SHAPE.to_string())?;
        let close = item.rfind('}').filter(|c| *c > open).ok_or_else(|| SHAPE.to_string())?;
        if name.is_empty() || item[..open].contains(';') {
            return Err(SHAPE.into());
        }
        let (body, tests) = Self::strip_tests(&item[open + 1..close]);
        if tests.is_empty() {
            return Err(format!("`mod {name}` has no `#[test]` fn for `#[symbian_test::tests]` to run"));
        }
        let cases: Vec<String> = tests.iter()
            .map(|t| format!("::symbian_test::Case {{ name: \"{t}\", run: {t} }}")).collect();
        Ok(format!(
            "{head}{{{body}\npub(super) const __SYMBIAN_TESTS: &[::symbian_test::Case] = &[{list}];\n}}\n\
             #[unsafe(export_name = \"_Z7E32Mainv\")]\n\
             pub extern \"C\" fn __symbian_e32main() -> i32 {{\n\
             ::symbian_std::__start(|| ::symbian_test::__run(env!(\"CARGO_CRATE_NAME\"), \
             ::symbian_std::uid3!(), {name}::__SYMBIAN_TESTS))\n}}\n",
            head = &item[..open], list = cases.join(", ")))
    }

    /// The body without its `#[test]` attributes, and the fns that carried one, in order.
    fn strip_tests(body: &str) -> (String, Vec<String>) {
        let (mut out, mut names) = (String::new(), Vec::new());
        let b = body.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'"' | b'\'' => {
                    let end = Self::literal_end(b, i);
                    out.push_str(&body[i..end]);
                    i = end;
                }
                b'#' => match Self::test_attribute_end(body, i) {
                    Some(end) => {
                        if let Some(name) = Self::next_fn_name(&body[end..]) { names.push(name); }
                        i = end;
                    }
                    None => { out.push('#'); i += 1; }
                },
                _ => {
                    let ch = body[i..].chars().next().unwrap_or(' ');
                    out.push(ch);
                    i += ch.len_utf8();
                }
            }
        }
        (out, names)
    }

    /// `#` `[` `test` `]` with any whitespace between: the index after `]`.
    fn test_attribute_end(body: &str, at: usize) -> Option<usize> {
        let rest = body[at + 1..].trim_start().strip_prefix('[')?.trim_start().strip_prefix("test")?;
        let rest = rest.trim_start().strip_prefix(']')?;
        Some(body.len() - rest.len())
    }

    fn next_fn_name(text: &str) -> Option<String> {
        let at = text.find("fn ")?;
        let name: String = text[at + 3..].trim_start().chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        (!name.is_empty()).then_some(name)
    }

    /// The index just past the string or char literal starting at `at`.
    fn literal_end(b: &[u8], at: usize) -> usize {
        let quote = b[at];
        let mut i = at + 1;
        while i < b.len() && b[i] != quote {
            i += if b[i] == b'\\' { 2 } else { 1 };
        }
        (i + 1).min(b.len())
    }
}
```

A `'` that starts a lifetime (`'a`) is not a char literal. `literal_end` would then run to the
next `'`. Guard it: treat `'` as a literal only when the byte two or three on is a `'` (a char
literal is `'x'`, `'\n'` or `'\u{…}'`); add a test with `fn f<'a>(x: &'a str)` in the module.

The unit tests in Step 1 call `TestModule::expand(src)` directly with a `&str` (drop the
`.parse().unwrap()` and `.to_string()` there). Assert on `"#[test]"` absent rather than
`"# [test]"`, and on `checks::__SYMBIAN_TESTS` without spaces.

`lib.rs`:

```rust
mod test_module;

/// A module of `#[test] fn name() -> Result<(), symbian_test::Evidence>` becomes the
/// program's `E32Main`, run on the device by `cargo test` (design spec §7).
#[proc_macro_attribute]
pub fn tests(_attribute: TokenStream, item: TokenStream) -> TokenStream {
    match test_module::TestModule::expand(&item.to_string()) {
        Ok(out) => tokens(&out),
        Err(message) => tokens(&compile_error(&message)),
    }
}
```

`symbian-test/Cargo.toml`: `[package] name = "symbian-test"`, workspace version/edition/
licence, `[dependencies] symbian-std = { path = "../symbian-std" }`, `symbian-macros = {
path = "../symbian-macros" }`. `src/lib.rs`:

```rust
//! The harness of a `tests/*.rs` with `harness = false` (design spec §7): `libtest` needs
//! `std` and a host; this runs the cases on the phone and writes the report `symdev`
//! reads, marking each case before it runs so a panic is attributed.
#![no_std]
extern crate alloc;

mod evidence;

pub use evidence::{Evidence, ensure};
pub use symbian_macros::tests;

use symbian_std::test_report::Report;

pub struct Case {
    pub name: &'static str,
    pub run: fn() -> Result<(), Evidence>,
}

#[doc(hidden)]
pub fn __run(app: &str, uid3: u32, cases: &[Case]) -> i32 {
    let mut report = Report::with_uid3(app, uid3);
    for case in cases { report.pending(case.name); }
    let _ = report.save();
    for case in cases {
        report.running(case.name);
        let _ = report.save();
        match (case.run)() {
            Ok(()) => report.settle(case.name, true, ""),
            Err(e) => report.settle(case.name, false, e.text()),
        }
        let _ = report.save();
    }
    0 // the verdict is the report's; the runner reads it after the process ends
}
```

`src/evidence.rs`: `pub struct Evidence(alloc::string::String)` with
`impl<E: symbian_std::test_report::Evidence> From<E> for Evidence` (uses `e.shown()`),
`pub fn msg(what: &str) -> Self`, `pub fn text(&self) -> &str`, and `pub fn ensure(ok: bool,
what: &str) -> Result<(), Evidence>`.

In `symbian-std/src/test_report/mod.rs`, `Case` gains `state: Option<&'static str>`.
`Report` gains `pending(&mut self, name)` (pushes a case with `ok: false`, `state:
Some("pending")`), `running(&mut self, name)` (sets that case's state to `"running"`),
`settle(&mut self, name, ok, detail)` (sets `ok`, `detail`, `state: None`), and `save(&self) ->
Result<()>` (what `finish` does, returning nothing). `finish` calls `save`. `json.rs` writes
`,"state":"<s>"` after `ok` when the case has one. `passed`/`failed`/`is_pass` count only
cases without a state.

- [ ] **Step 4: Run the tests and build the crate for the phone**

Run Step 2's command, expected all pass. Then, in `symbian-rs`: `cargo build --release -p
symbian-test` (the workspace config targets the phone). Expected: builds.

- [ ] **Step 5: Commit**

```bash
git add symbian-rs/Cargo.toml symbian-rs/Cargo.lock symbian-rs/crates/symbian-test \
  symbian-rs/crates/symbian-macros/src symbian-rs/crates/symbian-std/src/test_report
git commit -m "Add symbian-test, which runs a module of tests on the phone and marks each before it runs."
```

### Task 7: The project shape, and `symdev new` writes it

**Files:**
- Modify: `crates/symdev-cli/src/scaffold_rust.rs` (`cargo_manifest`, `cargo_config`, a `tests/smoke.rs`)
- Create: `crates/symdev-cli/src/scaffold_rust/tests.rs` (the module's tests move here; the file must stay ≤ 300 lines)
- Modify: `symbian-rs/examples/hello/{Cargo.toml,src/main.rs}` (the scaffold's `src/main.rs` is `include_str!` of it: `RustSdk::HELLO_MAIN`)
- Modify: `crates/symdev-build/src/rust_sdk.rs` (`HELLO_MAIN`'s doc: no longer "no `#![no_main]`")
- Modify: `symbian-rs/.cargo/config.toml` (the examples' shared config: linker, runner, `panic-abort-tests`)

**Interfaces:**
- Consumes: Task 4 (`symdev-ld` on `PATH`), Task 5 (spec), Task 6 (`symbian-test`).
- Produces: the project shape every later task assumes:
  - `Cargo.toml`: `[[bin]] name = "<name>"`, `path = "src/main.rs"`, `test = false`;
    `[[test]] name = "smoke"`, `harness = false`; `[dev-dependencies] symbian-test = { path
    = "build/rust-sdk/symbian-rs/crates/symbian-test" }`; the release profile as today.
  - `.cargo/config.toml`: today's `[build]` and `[unstable]` plus `panic-abort-tests = true`,
    and `[target.arm-symbian-e32] linker = "symdev-ld"`, `runner = "symdev run --exe"`.
  - `src/main.rs` with `#![no_main]`; `tests/smoke.rs`.

- [ ] **Step 1: Write the failing test** — in `scaffold_rust/tests.rs`, change
  `rust_project_has_cargo_files_and_no_mmp` to:

```rust
    let cargo = read("Cargo.toml");
    assert!(!cargo.contains("staticlib") && !cargo.contains("autobins"), "{cargo}");
    assert!(cargo.contains("[[bin]]\nname = \"hello\"\npath = \"src/main.rs\"\ntest = false\n"), "{cargo}");
    assert!(cargo.contains("[[test]]\nname = \"smoke\"\nharness = false\n"), "{cargo}");
    assert!(cargo.contains("symbian-test = { path = \"build/rust-sdk/symbian-rs/crates/symbian-test\" }"));
    let config = read(".cargo/config.toml");
    for line in ["panic-abort-tests = true", "[target.arm-symbian-e32]",
                 "linker = \"symdev-ld\"", "runner = \"symdev run --exe\""] {
        assert!(config.contains(line), "{line}: {config}");
    }
    assert!(read("src/main.rs").contains("#![no_main]"));
    assert_eq!(read("src/main.rs"), RustSdk::HELLO_MAIN);
    let smoke = read("tests/smoke.rs");
    assert!(smoke.contains("#[symbian_test::tests]") && smoke.contains("#![no_main]"), "{smoke}");
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p symdev-cli --offline scaffold_rust`
Expected: FAIL on `staticlib`.

- [ ] **Step 3: Implement**

`cargo_manifest(name)` writes, in place of `autobins = false` and the `[lib]` table:

```toml
[[bin]]
name = "{name}"
path = "src/main.rs"
# No libtest on the phone: tests are tests/*.rs with harness = false (symbian-test).
test = false

[[test]]
name = "smoke"
harness = false
```

and after `[dependencies]`:

```toml
[dev-dependencies]
symbian-test = { path = "build/rust-sdk/symbian-rs/crates/symbian-test" }
```

(via `RustSdkLink::crate_dir("symbian-test")`). `cargo_config()` adds `panic-abort-tests =
true` to `[unstable]`, with the comment `# cargo test builds core twice without it
(E0152, experiment 114 §1.1)`, and:

```toml
# cargo links through symdev (signed .sisx beside the image) and runs on a device.
[target.arm-symbian-e32]
linker = "symdev-ld"
runner = "symdev run --exe"
```

`write_rust` writes `tests/smoke.rs`:

```rust
//! A test on the device: `cargo test` builds it, symdev installs and runs it, and prints
//! what it reports (symbian-test).
#![no_std]
#![no_main]

#[symbian_test::tests]
mod smoke {
    use symbian_test::{Evidence, ensure};

    #[test]
    fn arithmetic() -> Result<(), Evidence> {
        ensure(2 + 2 == 4, "2 + 2 is 4")
    }
}
```

`symbian-rs/examples/hello/Cargo.toml`: replace the comment line, `autobins = false` and the
`[lib]` table with the `[[bin]]` table above (`name = "hello"`, no `[[test]]`).
`symbian-rs/examples/hello/src/main.rs`: `#![no_main]` on the line after `#![no_std]`, and
the doc comment's last paragraph says the bin needs it because the attribute keeps `fn main`.
`symbian-rs/.cargo/config.toml`: add `panic-abort-tests = true` and the `[target.arm-symbian-e32]`
table above.

- [ ] **Step 4: Run the tests and a real project**

Run: `cargo test -p symdev-cli --offline`; expected: pass. Then, with the GCCE environment of
experiment 114 (`~/src/cargo-run-scratch/env.sh`, `SYMDEV_RUST_SDK` = this worktree's
`symbian-rs`, the branch's `symdev` and its `setup-linker` links first on `PATH`):

```bash
cd ~/src/cargo-run-scratch && rm -f t7.ok && symdev new t7 --lang rust && cd t7 \
  && cargo build --release && cargo test --no-run && touch ../t7.ok
ls build/t7.sisx build/cargo/arm-symbian-e32/release/t7.sisx
```

Expected: both `.sisx` exist; `cargo test --no-run` prints `Executable tests/smoke.rs
(…/out/smoke-<hash>)` and a `…/out/smoke-<hash>.sisx` exists beside it.

- [ ] **Step 5: Commit**

```bash
git add crates/symdev-cli/src/scaffold_rust.rs crates/symdev-cli/src/scaffold_rust \
  crates/symdev-build/src/rust_sdk.rs symbian-rs/examples/hello symbian-rs/.cargo/config.toml
git commit -m "Make symdev new write a binary crate that cargo links, runs and tests through symdev."
```

### Task 8: The other 20 examples in the new shape

**Files:**
- Modify: `symbian-rs/examples/<ex>/Cargo.toml` and `src/main.rs` for `alloc async atomics
  cleanup files fmt hello-raw locale net notes panic query shim spawnee time tls ui ui-list`
  (`std-hello` and `std-net` are Task 9)
- Modify: `symbian-rs/examples/README.md` (how to build: `cargo build --release`, `cargo run`)

**Interfaces:**
- Consumes: Tasks 4, 5, 7.
- Produces: every `no_std` example builds with plain `cargo build --release` in its directory.

- [ ] **Step 1: Convert**

`~/src/cargo-run-scratch/toshape.py <dir>…` (experiment 114) does exactly this edit and
asserts it happened: the `[lib]` staticlib table becomes `[[bin]]` named after the package
with `test = false`, and `#![no_main]` goes after `#![no_std]`. Run it on the 18 directories.
Then check by hand that `#![no_main]` sits after every crate-level attribute
(`examples/atomics` has `#![forbid(unsafe_code)]`, and an inner attribute after an item is
an error).

- [ ] **Step 2: Build every one with cargo and compare with experiment 114**

```bash
. ~/src/cargo-run-scratch/env.sh; export SYMDEV_RUST_SDK=$PWD/symbian-rs SYMDEV_SIGN_PASSWORD=scratch
for ex in alloc async atomics cleanup files fmt hello hello-raw locale net notes panic query \
          shim spawnee time tls ui ui-list; do
  (cd symbian-rs/examples/$ex && env -u RUSTUP_TOOLCHAIN cargo build --release >/dev/null 2>&1) ; echo "$ex rc=$?"
done
```

Expected: rc=0 for all 19. Each `build/<name>.exe` has the uncompressed size in experiment
113's "rust-lld, checkout" column (`e32cmp.py` prints it). This tree lies at another path
than experiment 114's, so only path-dependent bytes may differ (`net`, `tls`). The
byte-for-byte comparison at one path is Task 18's.

- [ ] **Step 3: Commit**

```bash
git add symbian-rs/examples/*/Cargo.toml symbian-rs/examples/*/src/main.rs symbian-rs/examples/README.md
git commit -m "Build the no_std examples as binaries that cargo links through symdev-ld."
```
