# M2 `symdev package` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `symdev package` write a recorded `.pkg`, run Wine `makesis`, then self-sign with Wine `makekeys`/`signsis`, and print the absolute `.sisx` path.

**Architecture:** Stay in `symdev-build`: a `.pkg` renderer (pure string) plus `SisPackage` (`PackageBackend`) that shells out through `LocalEnv` with **recorded** Wine argv. CLI loads `symdev.toml`, locates `build/<name>.exe`, packages, prints the `.sisx`. `new` / `deploy` stay not-implemented. Never invent `.pkg` grammar or tool flags.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. Existing crates only. Wine PE tools from `SYMDEV_EPOCROOT`. No `tokio`.

## Global Constraints

- Do not invent argv or `.pkg` lines. Copy experiment 7–8 in [experiment-backlog.md](../../research/experiment-backlog.md).
- Relative host dialect only: `"hello.exe"` next to the `.pkg` (experiment 7). Do not emit Unix absolute or `Z:\` paths.
- `.pkg` CRLF like the SDK sample.
- Platform UID line exact: `[0x102752AE], 0, 0, 0, {"S60ProductID"}`
- `TYPE=SA`, `&EN`, EXE → `!:\sys\bin\<name>.exe`
- Version: toml `MAJOR.MINOR.PATCH` → `.pkg` `major,minor,patch` (spec §7.1). Experiment 7 used `1,0,24` from the SDK sample; **do not freeze 1,0,24**.
- Vendor lines keep recorded grammar; substitute toml `symbian.vendor` into both vendor strings.
- UID3 required (same as `build`). Hex in `.pkg` as `(0x` + 8 lowercase digits + `)`.
- Wine: `wine PROGRAM [ARGUMENTS...]`. Tools: `$EPOCROOT/epoc32/tools/{makesis,signsis,makekeys}.exe`
- makesis (recorded): `wine <makesis.exe> -v <name>.pkg <name>.sis`
- makekeys (recorded): `-cert -expdays 3650 -password <pw> -len 2048 -dname "CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com" <key> <cer>` — DN is the tool’s own Example Usage, not invented.
- signsis (recorded): `wine <signsis.exe> <sis> <sisx> <cer> <key> <password>`
- Password from `SYMDEV_SIGN_PASSWORD` (≥4 characters as the tool requires). Never commit password/cer/key/sis/sisx.
- If `signing.cert` and `signing.key` are both set and the files exist, skip `makekeys` and pass those paths to `signsis`.
- `SYMDEV_WINE` optional; default `/usr/bin/wine`. `SYMDEV_EPOCROOT` required.
- Never `-fPIC`. No E52 claim. Work on branch `m2-package`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="38440891+4akloon@users.noreply.github.com"` and matching `GIT_COMMITTER_*`. Do not write git config.

## File structure

- Create: `crates/symdev-build/src/pkg.rs` — `render_pkg(...)` → `String` (CRLF)
- Create: `crates/symdev-build/src/sis.rs` — `SisTools`, `SisPackage`
- Modify: `crates/symdev-build/src/lib.rs` — exports
- Modify: `crates/symdev-cli/src/main.rs` — `package` command
- Modify: `crates/symdev-cli/tests/cli.rs` — `package` no longer `not implemented`

Env:

| Var | Role |
|---|---|
| `SYMDEV_EPOCROOT` | SDK root (`epoc32/tools/*.exe`) |
| `SYMDEV_WINE` | Wine binary; default `/usr/bin/wine` |
| `SYMDEV_SIGN_PASSWORD` | makekeys/signsis password |

---

### Task 1: `.pkg` renderer

**Files:** Create `crates/symdev-build/src/pkg.rs`; export from `lib.rs`.

**Interfaces:**
- Produces: `pub fn render_pkg(name: &str, uid3: u32, version: (u32, u32, u32), vendor: &str) -> String`

Recorded skeleton with substitutions only:

```
&EN
#{"<name>"},(0x<uid3 8 hex lowercase>),<maj>,<min>,<pat>,TYPE=SA
%{"<vendor>"}
:"<vendor>"
[0x102752AE], 0, 0, 0, {"S60ProductID"}
"<name>.exe"		-"!:\sys\bin\<name>.exe"
```

Use `\r\n`. The file line uses two tabs before `-` as in experiment 7.

- [ ] **Step 1: Failing test**

```rust
#[test]
fn render_pkg_matches_experiment_7_grammar() {
    let s = render_pkg("hello", 0xe79e4cf9, (0, 1, 0), "symdev");
    assert!(s.contains("\r\n"));
    assert_eq!(
        s.replace("\r\n", "\n"),
        "&EN\n#{\"hello\"},(0xe79e4cf9),0,1,0,TYPE=SA\n%{\"symdev\"}\n:\"symdev\"\n[0x102752AE], 0, 0, 0, {\"S60ProductID\"}\n\"hello.exe\"\t\t-\"!:\\sys\\bin\\hello.exe\"\n"
    );
}
```

- [ ] **Step 2:** `cargo test -p symdev-build render_pkg --offline` FAIL (`render_pkg` missing)
- [ ] **Step 3:** Minimal `render_pkg`
- [ ] **Step 4:** PASS
- [ ] **Step 5: Commit** `Render a MakeSIS pkg from the recorded experiment 7 grammar.`

---

### Task 2: Wine SIS argv helpers

**Files:** Create `crates/symdev-build/src/sis.rs` (argv only + `SisTools::from_env`).

**Interfaces:**
- `SisTools { wine, makesis, signsis, makekeys }`
- `from_env()`: `SYMDEV_EPOCROOT` required; `SYMDEV_WINE` default `/usr/bin/wine`; exe paths `$EPOCROOT/epoc32/tools/{makesis,signsis,makekeys}.exe`
- `makesis_args(pkg, sis) -> Vec<String>` = wine, makesis.exe, `-v`, pkg, sis (all as recorded; pkg/sis are **filenames** not directories)
- `makekeys_args(password, key, cer) -> Vec<String>`
- `signsis_args(sis, sisx, cer, key, password) -> Vec<String>`
- `dname()` constant: `CN=Joe Bloggs OU=Development O=Acme Ltd C=GB EM=noone@nowhere.com`

- [ ] **Step 1: Failing tests** asserting exact argv with `/sdk` + `/usr/bin/wine` fixtures (no `/home/genius`)
- [ ] **Step 2–4:** TDD
- [ ] **Step 5: Commit** `Copy recorded Wine makesis, makekeys, and signsis argv.`

---

### Task 3: `SisPackage` (`PackageBackend`)

**Files:** Extend `sis.rs`.

**Interfaces:**
- `pub struct SisPackage { pub env: LocalEnv, pub tools: SisTools, pub name: String, pub uid3: u32, pub version: (u32, u32, u32), pub vendor: String, pub password: String, pub cert: Option<PathBuf>, pub key: Option<PathBuf> }`
- `package(&[Artifact])`:
  1. Require exactly one artifact; its file name must be `<name>.exe` and it must exist.
  2. Workdir = artifact parent (so `"hello.exe"` is a relative name).
  3. Write `<name>.pkg` (CRLF) next to the exe.
  4. `run_blocking` makesis argv (cwd = workdir).
  5. If cert+key both `Some` and both files exist (resolve relative to **project** — pass already-absolute paths from CLI): skip makekeys. Else run makekeys into workdir `<name>.key` / `<name>.cer`.
  6. signsis → `<name>.sisx`
  7. Non-zero → `Error::Other` with stderr.
  8. Return `Package { primary: sisx, companions: vec![sis] }`

Password empty or `< 4` chars → error `SYMDEV_SIGN_PASSWORD must be at least 4 characters` before spawning.

Unit tests: argv/file-write without Wine using a stub? Keep `package()` integration `#[ignore]` or skip if wine missing. **Required tests that do not need Wine:** `from_env` missing EPOCROOT; password too short via a `validate_password` helper; `package` errors if artifact missing (`no E32 artifact`). For missing artifact, `package(&[])` or missing file → `Error::Other("no E32 artifact")` / `"E32 not found"`.

Do **not** spawn Wine in default `cargo test`.

- [ ] **Step 1–4:** TDD those error paths + write-pkg to a temp dir by testing a `write_pkg_file` helper if `package()` always wants tools.
- [ ] **Step 5: Commit** `Package E32 into SISX via recorded Wine SIS tools.`

---

### Task 4: Wire `symdev package`

**Files:** `crates/symdev-cli/src/main.rs`, `tests/cli.rs`

**Behavior:**
- Valid toml, missing `uid3` → same error as build (`uid3 required for build (set symbian.uid3)` is wrong wording). Use `uid3 required for package (set symbian.uid3)`.
- Missing `SYMDEV_EPOCROOT` → `missing toolchain: SYMDEV_EPOCROOT`
- Missing/short password → the password error (test with EPOCROOT set to `/sdk` dummy and password unset)
- Success: print absolute `.sisx` path, exit 0
- Locate E32 at `./build/<package.name>.exe` (relative to cwd). If missing → `E32 not found: build/<name>.exe (run symdev build)`
- Resolve `signing.cert` / `signing.key` against cwd if relative.

Change `package_valid_manifest_not_implemented` to the missing-uid3 or missing-epocroot case (uid3 omitted in HELLO → uid3 required, even before env). Order: load manifest → uid3 → e32 path → env tools → password → package.

Test HELLO without uid3: `package` → uid3 required, not `not implemented`.

Test with uid3, no EPOCROOT: `missing toolchain: SYMDEV_EPOCROOT`.

`build` / `new` / `deploy` tests unchanged.

- [ ] **Step 1:** Change CLI test; watch FAIL on `not implemented`
- [ ] **Step 2–4:** Wire and PASS `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Make symdev package write a SISX with Wine SIS tools.`

---

## Self-review

- M2 is package+sign only. No deploy. No `new`.
- Argv and `.pkg` copied from experiments 7–8.
- Cert/key/password never committed.
- Default tests do not require Wine or the SDK tree.

User previously chose Subagent-Driven. Execute on `m2-package` without pausing between tasks.
