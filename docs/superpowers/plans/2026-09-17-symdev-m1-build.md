# M1 `symdev build` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `symdev build` parse `bld.inf` / `.mmp` and produce an E32 EXE on this Linux host using the Wave 0 argv recorded in experiments 5–6.

**Architecture:** One new crate `symdev-build` holds the fail-closed parser and the `BuildBackend` that shells out to `g++` → `ld` 2.29.1 → `elf2e32`. `symdev-core` gains a local `ExecutionEnvironment`. The CLI loads `symdev.toml`, then calls `BuildBackend::build`. `package` / `deploy` / `new` stay not-implemented. No `abld` / `makmake`. Never `-fPIC` / `-fPIE`.

**Tech Stack:** Rust 1.98.1, edition 2024, workspace resolver `"3"`. Existing: `clap`, `serde`, `toml`, `thiserror`. Add `pollster` only in `symdev-cli` to block on `ExecutionEnvironment::run`. No `tokio`. No extra clap verbs.

## Global Constraints

- `rust-version = "1.98.1"`, edition `2024`, workspace `resolver = "3"`.
- Do not invent argv. Compile / link / `elf2e32` flags are **exactly** the recorded experiment 5–6 sequences with path/UID/name substitution only ([experiment-backlog.md](../../research/experiment-backlog.md)).
- Linker binary must be GNU ld **2.29.1** (`arm-none-symbianelf-ld`). ld 2.35 fails on SDK `euser.dso`.
- Never pass `-fPIC` or `-fPIE`.
- Never commit SDK, ROM, `.cer`, `.key`, `.sis`, `.sisx`.
- User-grantable caps only; privileged caps already hard-error in the manifest.
- `target.device` only `nokia-e52`; `language.name` only `cpp`; `signing.mode` only `self-signed`.
- Do not claim E52 support. Experiments 10–11 remain skip.
- Parser: fail closed on unknown directives ([mmp-bld-inf-parser.md](../../research/mmp-bld-inf-parser.md)).
- `#ifdef` / `#ifndef` / `#elif` / `#include` stay parse errors (spec). Experiment 12 saw `#ifdef` in some SDK examples; M1 hello `bld.inf` must not use them.
- `PRJ_TESTMMPFILES` parsed, not built.
- Do not generate Makefiles.
- TDD: failing test first, watch it fail, then minimal code.
- Commit after each task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` (and matching `GIT_COMMITTER_*`). Do not write git config.
- Work on branch `m1-build`. Do not merge to `main` in this plan.

## File structure

- Modify: `Cargo.toml` — add member `crates/symdev-build`
- Create: `crates/symdev-build/Cargo.toml`
- Create: `crates/symdev-build/src/lib.rs`
- Create: `crates/symdev-build/src/bld.rs` — `bld.inf` parser
- Create: `crates/symdev-build/src/mmp.rs` — `.mmp` parser
- Create: `crates/symdev-build/src/model.rs` — structured model types
- Create: `crates/symdev-build/src/driver.rs` — `GcceBuild` (`BuildBackend`)
- Create: `crates/symdev-build/src/toolchain.rs` — tool paths from env
- Create: `crates/symdev-core/src/local_env.rs` — `LocalEnv`
- Modify: `crates/symdev-core/src/lib.rs` — export `LocalEnv`
- Modify: `crates/symdev-core/src/error.rs` — `Other` already exists; add `Io` if needed via `Other(String)`
- Modify: `crates/symdev-cli/Cargo.toml` — depend on `symdev-build`, `pollster`
- Modify: `crates/symdev-cli/src/main.rs` — `build` calls driver
- Modify: `crates/symdev-cli/tests/cli.rs` — `build` with valid toml no longer prints M1 not-implemented when tools/SDK missing: it must fail with a **configured** error, not `not implemented`
- Do not create `golden/`, `fixtures/` at repo root. Parser tests use `tempfile` strings. Do not vendor SDK headers.

## Toolchain env (recorded host values; driver reads env, never hardcodes `/home/genius`)

| Env | Role | Recorded on this host |
|---|---|---|
| `SYMDEV_EPOCROOT` | SDK root that contains `epoc32/` | `/home/genius/sdk/S60_3rd_FP2` |
| `SYMDEV_GXX` | `arm-none-symbianelf-g++` | `/home/genius/gcc-builds/gcc-12.1.0/bin/arm-none-symbianelf-g++` |
| `SYMDEV_LD` | `arm-none-symbianelf-ld` **2.29.1** | `/home/genius/gcc-builds/binutils-2.29.1/bin/arm-none-symbianelf-ld` |
| `SYMDEV_ELF2E32` | Linux elf2e32 | `/home/genius/src/elf2e32_next/bin/Release/elf2e32` |
| `SYMDEV_GCC_LIB` | gcc lib dir for `-L` and libgcc include | `/home/genius/gcc-builds/gcc-12.1.0/lib/gcc/arm-none-symbianelf/12.1.0` |
| `SYMDEV_GCC_TARGET_LIB` | `arm-none-symbianelf/lib` | `/home/genius/gcc-builds/gcc-12.1.0/arm-none-symbianelf/lib` |

Missing env → `symdev-core::Error::Other` with `missing toolchain: SYMDEV_*`. Do not search `PATH` for ld 2.35.

`EPOCROOT` trailing `/` optional; join with `epoc32/include/gcce/gcce.h` etc. as in experiment 1.

---

### Task 1: Local `ExecutionEnvironment`

**Files:**
- Create: `crates/symdev-core/src/local_env.rs`
- Modify: `crates/symdev-core/src/lib.rs`

**Interfaces:**
- Consumes: `ExecutionEnvironment`, `RemotePath`, `Output`, `PathStyle`, `Result`, `Error`
- Produces: `pub struct LocalEnv;` impl `ExecutionEnvironment` with `path_style() -> PathStyle::Posix`. `run` executes `cmd` with `current_dir = cwd` (posix string). `push` copies file `local` → `remote` path. `pull` copies `remote` → `local`.

- [ ] **Step 1: Write the failing test** in `crates/symdev-core/src/lib.rs` (or `local_env.rs` under `#[cfg(test)]`):

```rust
#[test]
fn local_env_run_echo_status_zero() {
    let env = LocalEnv;
    let mut cmd = std::process::Command::new("true");
    let out = pollster::block_on(env.run(cmd, &RemotePath::new("/tmp"))).unwrap();
    assert_eq!(out.status, 0);
}
```

Do **not** add `pollster` to `symdev-core`. In core tests, call a `std` helper: make `LocalEnv::run_blocking` used by the async trait method, and test `run_blocking` directly so core stays tokio/pollster-free:

```rust
#[test]
fn local_env_run_true_status_zero() {
    let out = LocalEnv.run_blocking(
        std::process::Command::new("true"),
        &RemotePath::new("/"),
    )
    .unwrap();
    assert_eq!(out.status, 0);
}

#[test]
fn local_env_push_copies_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("a.txt");
    let dst = dir.path().join("b.txt");
    std::fs::write(&src, b"hi").unwrap();
    LocalEnv
        .push_blocking(&src, &RemotePath::new(dst.to_str().unwrap()))
        .unwrap();
    assert_eq!(std::fs::read(dst).unwrap(), b"hi");
}
```

Add `tempfile` as **dev-dependency** of `symdev-core` only.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p symdev-core local_env_run_true_status_zero --offline`
Expected: FAIL (type `LocalEnv` not found)

- [ ] **Step 3: Write minimal implementation**

```rust
pub struct LocalEnv;

impl LocalEnv {
    pub fn run_blocking(
        &self,
        mut cmd: std::process::Command,
        cwd: &RemotePath,
    ) -> Result<Output> {
        let out = cmd
            .current_dir(cwd.to_string())
            .output()
            .map_err(|e| Error::Other(e.to_string()))?;
        Ok(Output {
            status: out.status.code().unwrap_or(1),
            stdout: out.stdout,
            stderr: out.stderr,
        })
    }

    pub fn push_blocking(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()> {
        std::fs::copy(local, remote.to_string()).map(|_| ()).map_err(|e| Error::Other(e.to_string()))
    }

    pub fn pull_blocking(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()> {
        std::fs::copy(remote.to_string(), local).map(|_| ()).map_err(|e| Error::Other(e.to_string()))
    }
}

impl ExecutionEnvironment for LocalEnv {
    async fn run(&self, cmd: std::process::Command, cwd: &RemotePath) -> Result<Output> {
        self.run_blocking(cmd, cwd)
    }
    async fn push(&self, local: &std::path::Path, remote: &RemotePath) -> Result<()> {
        self.push_blocking(local, remote)
    }
    async fn pull(&self, remote: &RemotePath, local: &std::path::Path) -> Result<()> {
        self.pull_blocking(remote, local)
    }
    fn path_style(&self) -> PathStyle {
        PathStyle::Posix
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p symdev-core --offline`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add crates/symdev-core/src/local_env.rs crates/symdev-core/src/lib.rs crates/symdev-core/Cargo.toml
git commit -m "Add a local ExecutionEnvironment that runs host commands."
```

---

### Task 2: `bld.inf` parser

**Files:**
- Create: `crates/symdev-build/Cargo.toml` (deps: `symdev-core`, `thiserror`)
- Create: `crates/symdev-build/src/model.rs`
- Create: `crates/symdev-build/src/bld.rs`
- Create: `crates/symdev-build/src/lib.rs`
- Modify: `Cargo.toml` workspace members

**Interfaces:**
- Consumes: parser spec support list
- Produces: `pub fn parse_bld_inf(text: &str) -> Result<BldInf, ParseError>`
- `BldInf { mmp_files: Vec<PathBuf>, test_mmp_files: Vec<PathBuf>, exports: Vec<String> }`

`PRJ_PLATFORMS` evaluation (from spec + experiment 12 tokens, no invented aliases):

- Directive omitted → accept (default ARMV5 UREL GCCE).
- Empty token list → error `PRJ_PLATFORMS has no usable platform`.
- Accept if any token (ASCII case-insensitive) is `GCCE`, `ARMV5`, `ARMV5_ABIV2`, or `DEFAULT`. Ignore extras (`WINSCW`, …).
- Otherwise error.

Lexical starting point (Likely): directive names case-insensitive; `#` line that is not `#if`/`#else`/`#endif` is a comment; `//` comment to end of line. `#ifdef` is a **parse error**.

`PRJ_MMPFILES` / `PRJ_TESTMMPFILES`: one path per subsequent non-empty line until the next directive.

- [ ] **Step 1: Write failing tests** in `crates/symdev-build/src/bld.rs`:

```rust
#[test]
fn omitted_platforms_collects_mmp() {
    let b = parse_bld_inf("PRJ_MMPFILES\nhello.mmp\n").unwrap();
    assert_eq!(b.mmp_files, [std::path::PathBuf::from("hello.mmp")]);
    assert!(b.test_mmp_files.is_empty());
}

#[test]
fn gcce_token_accepted() {
    parse_bld_inf("PRJ_PLATFORMS\nWINSCW ARMV5 GCCE\nPRJ_MMPFILES\na.mmp\n").unwrap();
}

#[test]
fn winscw_only_rejected() {
    assert!(parse_bld_inf("PRJ_PLATFORMS\nWINSCW\nPRJ_MMPFILES\na.mmp\n").is_err());
}

#[test]
fn ifdef_is_parse_error() {
    assert!(parse_bld_inf("#ifdef EKA2\nPRJ_MMPFILES\na.mmp\n#endif\n").is_err());
}

#[test]
fn unknown_directive_errors() {
    assert!(parse_bld_inf("PRJ_EXTENSIONS\n").is_err());
}
```

- [ ] **Step 2: Run tests — expect FAIL** (`parse_bld_inf` missing)

Run: `cargo test -p symdev-build --offline`

- [ ] **Step 3: Minimal parser** matching the tests and spec support list only.

- [ ] **Step 4: `cargo test -p symdev-build --offline` PASS** plus `cargo test --workspace --offline`

- [ ] **Step 5: Commit** `Add a fail-closed bld.inf parser for EXE GCCE builds.`

---

### Task 3: `.mmp` parser

**Files:**
- Create: `crates/symdev-build/src/mmp.rs`
- Modify: `crates/symdev-build/src/model.rs`
- Modify: `crates/symdev-build/src/lib.rs`

**Interfaces:**
- Produces: `pub fn parse_mmp(text: &str) -> Result<Mmp, ParseError>`
- `Mmp { target: String, target_type: String, uid: Vec<u32>, source: Vec<String>, sourcepath: Vec<String>, systeminclude: Vec<String>, userinclude: Vec<String>, library: Vec<String>, staticlibrary: Vec<String>, capability: Vec<String>, … }` — retain listed fields; `target_type` must be `EXE` (case-insensitive) or error.

`UID` optional; if present, two or three integers (hex `0x` or decimal).

`START RESOURCE` … `END`: parse as blocks, retain inner lines as `Vec<Vec<String>>`. Do not compile resources in M1.

Unknown directive → error. `TARGETTYPE DLL` → error.

- [ ] **Step 1: Failing tests**

```rust
#[test]
fn parse_exe_with_source() {
    let m = parse_mmp("TARGET hello.exe\nTARGETTYPE EXE\nSOURCE hello.cpp\n").unwrap();
    assert_eq!(m.target, "hello.exe");
    assert_eq!(m.source, ["hello.cpp"]);
}

#[test]
fn dll_rejected() {
    assert!(parse_mmp("TARGET x.dll\nTARGETTYPE DLL\n").is_err());
}

#[test]
fn option_gcce_rejected() {
    assert!(parse_mmp("TARGET x.exe\nTARGETTYPE EXE\nOPTION_GCCE -O3\n").is_err());
}
```

- [ ] **Step 2–4:** TDD as Task 2
- [ ] **Step 5: Commit** `Parse EXE MMP files and reject unknown directives.`

---

### Task 4: GCCE driver (`BuildBackend`)

**Files:**
- Create: `crates/symdev-build/src/toolchain.rs`
- Create: `crates/symdev-build/src/driver.rs`
- Modify: `crates/symdev-build/src/lib.rs`

**Interfaces:**
- Consumes: `Toolchain` from env, `LocalEnv.run_blocking`, parsed `BldInf`+`Mmp`, `symdev_manifest` values for uid3 + capabilities
- Produces: `pub struct GcceBuild { pub env: LocalEnv, pub tools: Toolchain, pub uid3: u32, pub capabilities: Vec<String> }` impl `BuildBackend`

`Toolchain::from_env() -> Result<Toolchain, Error>` reads the six `SYMDEV_*` vars.

`build(&Project)`:

1. Read `project.root/group/bld.inf` **or** `project.root/bld.inf` — first that exists. Else error `no bld.inf`.
2. Parse; for each `mmp_files` entry resolve relative to the `bld.inf` directory.
3. Parse MMP. Compile each `SOURCE` (joined with last `SOURCEPATH` if present, else MMP dir / project root — retain both; **try SOURCEPATH/SOURCE then MMP-dir/SOURCE then project-root/SOURCE**, first existing file. Record which dialect worked in a comment in the driver, not a new flag).
4. Compile argv = experiment 5 `g++` list, substituting:
   - include `gcce.h` = `$EPOCROOT/epoc32/include/gcce/gcce.h`
   - `__PRODUCT_INCLUDE__` = `$EPOCROOT/epoc32/include/variant/symbian_os_v9.3.hrh`
   - `-I` user source dir, `$EPOCROOT/epoc32/include`, `$EPOCROOT/epoc32/include/variant`, `$SYMDEV_GCC_LIB/include`
   - `-o` dest in `project.root/build/<stem>.o`
5. Link argv = experiment 5 **ld 2.29.1** list, substituting soname ` {name}{000a0000}[{uid3 hex lowercase 8}].exe ` where `name` is MMP `TARGET` stem (strip `.exe`). `-o project.root/build/<stem>.elf`. `-L` urel + lib as recorded. Never use gcc-12.1.0 `ld`.
6. elf2e32 argv = experiment 6, substituting uid3, `--linkas` matching soname, `--output=project.root/build/<stem>.exe`, `--elfinput=…elf`, `--libpath=$EPOCROOT/epoc32/release/armv5/lib`. `--capability=` join manifest caps with `+`. If caps empty, pass `--capability=` with empty value (do not invent the six).
7. Return `vec![Artifact { path: exe }]`.
8. Non-zero tool status → `Error::Other` including stderr.

Unit tests for argv construction **without** running the cross compiler: `GcceBuild::compile_args(...)` / `link_args(...)` / `elf2e32_args(...)` return `Vec<String>` asserted equal to the recorded fragments (with fake prefix `/sdk` and `/gcc`).

A separate `#[ignore]` or env-gated test `gcce_builds_hello_when_toolchain_present` runs only if `SYMDEV_EPOCROOT` is set.

- [ ] **Step 1: Failing tests for argv helpers** (no `/home/genius` in asserts; use `/sdk` fixtures)
- [ ] **Step 2:** FAIL
- [ ] **Step 3:** Implement `toolchain.rs` + `driver.rs`
- [ ] **Step 4:** `cargo test -p symdev-build --offline` PASS
- [ ] **Step 5: Commit** `Drive g++, ld 2.29.1, and elf2e32 from recorded Wave 0 argv.`

---

### Task 5: Wire `symdev build`

**Files:**
- Modify: `crates/symdev-cli/Cargo.toml`
- Modify: `crates/symdev-cli/src/main.rs`
- Modify: `crates/symdev-cli/tests/cli.rs`

**Interfaces:**
- Consumes: `symdev_manifest::load`, `GcceBuild`, `Toolchain::from_env`, `LocalEnv`
- Produces: `symdev build` on valid toml either builds or errors `missing toolchain: …` / `no bld.inf` — **never** `not implemented` for `build`.

Change `build_valid_manifest_not_implemented` to `build_valid_manifest_missing_toolchain` expecting stderr `missing toolchain` (exit 1) when env is unset. Unset `SYMDEV_*` in the test process.

`package` / `deploy` / `new` tests unchanged.

If toolchain env **is** set in the developer environment, CLI tests must still pass: the missing-toolchain test should `std::env::remove_var` those keys (or use a child-only env). `assert_cmd` does not inherit a custom wipe unless you `.env_remove("SYMDEV_EPOCROOT")` etc. for all six.

On success, print nothing extra required; exit 0. Optional: print the E32 path on stdout as one line (absolute). Do that: `println!("{}", artifact.path.display());`

- [ ] **Step 1: Change CLI test** (fail while main still returns not-implemented)
- [ ] **Step 2:** FAIL with unexpected `not implemented`
- [ ] **Step 3: Wire main.rs**

```rust
Some(Commands::Build) => match symdev_manifest::load(Path::new("symdev.toml")) {
    Ok(m) => match build_project(m) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    },
    Err(e) => { eprintln!("error: invalid manifest: {e}"); ExitCode::from(1) }
}
```

`uid3`: if `m.symbian.uid3` is `Some`, parse hex; if `None`, error `uid3 required for build (set symbian.uid3)` — do not generate UIDs (M4).

- [ ] **Step 4:** `cargo test --workspace --offline` PASS. If `SYMDEV_*` set, also run `symdev build` in a copy of experiment-5 hello **only as a manual check**; do not copy SDK into git. A tiny project under `tempfile` with hello.cpp from the public-domain listing is OK for an `#[ignore]` test.

- [ ] **Step 5: Commit** `Make symdev build run the Wave 0 GCCE driver.`

---

## Self-review

- Spec M1: parse bld.inf/mmp, E32 from driver, `symdev build` on this host — Tasks 2–5.
- No package/sign (M2). No deploy (M4). No E52 claim.
- Argv copied from experiments 5–6, not invented.
- `#ifdef` still fail-closed.
- No placeholders remaining.

## Execution

User already chose Subagent-Driven (option 1) for this repo and asked to continue without a phone. Execute this plan on `m1-build` without pausing between tasks.
