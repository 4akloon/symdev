# M4 `symdev new` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `symdev new` write a buildable project tree, and make `symdev deploy` print the absolute `.sisx` path if it exists.

**Architecture:** Stay in `symdev-cli`. A `scaffold` module hashes the package name into a test-range uid3, writes `symdev.toml` + `group/bld.inf` + `group/<name>.mmp`, and copies recorded `hello.cpp` / `hello.h` via `include_str!`. CLI `new` calls that helper. CLI `deploy` loads the manifest and prints `build/<name>.sisx`. No new crate. No Wine.

**Tech Stack:** Rust 1.98.1, edition 2024, resolver `"3"`. Existing crates only. No new Cargo dependencies.

## Global Constraints

- Spec: [2026-09-17-symdev-m4-new-design.md](../specs/2026-09-17-symdev-m4-new-design.md). Do not contradict it.
- Do not invent argv, `.pkg` lines, or a hello `_reg.rss`. Do not write a skeleton `.pkg`.
- `src/hello.cpp` and `src/hello.h` are byte copies of experiment 5. Do not rename or substitute `<name>` inside them.
- uid3: FNV-1a 32-bit as specified; `hello` → `0xef9f2cab`. Toml spelling `0x` + 8 lowercase hex digits.
- MMP `SOURCEPATH ../src` (POSIX). No `UID` / `LIBRARY` / `SYSTEMINCLUDE` / `START RESOURCE`.
- `bld.inf`: omit `PRJ_PLATFORMS`; no `#ifdef`; LF.
- If `./<name>` exists, `directory \`<name>\` already exists` and write nothing.
- `deploy` does not require uid3 / EPOCROOT / password. Missing sisx: `SISX not found: build/<name>.sisx (run symdev package)`.
- Never `-fPIC`. No E52 claim. Work on branch `m4-new`.
- TDD. Commit per task with `GIT_AUTHOR_NAME="Yevhenii Aleksiuk"` `GIT_AUTHOR_EMAIL="alexiuk.genius@gmail.com"` and matching `GIT_COMMITTER_*`. Do not write git config.
- Default tests do not spawn Wine or need the SDK.

## File structure

- Create: `crates/symdev-cli/src/scaffold.rs`
- Create: `crates/symdev-cli/templates/hello.cpp`
- Create: `crates/symdev-cli/templates/hello.h`
- Modify: `crates/symdev-cli/src/main.rs` — `mod scaffold`; `New` and `Deploy` arms
- Modify: `crates/symdev-cli/tests/cli.rs`

---

### Task 1: uid3 FNV-1a

**Files:** Create `crates/symdev-cli/src/scaffold.rs`; `mod scaffold;` in `main.rs` (no CLI behaviour change yet).

**Interfaces:**
- `pub fn uid3_for_name(name: &str) -> u32`
- `pub fn uid3_hex(name: &str) -> String` — `format!("0x{:08x}", uid3_for_name(name))`

- [ ] **Step 1: Failing test** in `scaffold.rs`

```rust
#[test]
fn uid3_hello_is_pinned() {
    assert_eq!(uid3_for_name("hello"), 0xef9f2cab);
    assert_eq!(uid3_hex("hello"), "0xef9f2cab");
}
```

- [ ] **Step 2:** `cargo test -p symdev-cli uid3_hello --offline` FAIL (`uid3_for_name` missing)
- [ ] **Step 3:** Minimal FNV-1a: offset `0x811c9dc5`, prime `0x01000193`, then `0xE0000000 | (h & 0x0FFFFFFF)`
- [ ] **Step 4:** PASS
- [ ] **Step 5: Commit** `Hash package names into a test-range uid3.`

---

### Task 2: `create_project`

**Files:** Extend `scaffold.rs`; add `crates/symdev-cli/templates/hello.cpp` and `hello.h`.

**Interfaces:**
- `pub fn create_project(cwd: &Path, name: &str) -> Result<PathBuf, Error>`
- Returns the absolute path of `cwd/name` (join `cwd` with `name`; if that is relative, wrap with `std::fs::canonicalize` after creating, or `cwd.join(name)` when `cwd` is already absolute — tests pass an absolute temp path and expect `dir.path().join("hello")` as stdout later).
- Existence: if `cwd.join(name).exists()`, `Error::Other(format!("directory `{name}` already exists"))` before any create.
- Writes:

`symdev.toml` (LF), with `{name}` and `{uid3}` = `uid3_hex(name)`:

```toml
[package]
name = "{name}"
version = "0.1.0"

[target]
device = "nokia-e52"

[language]
name = "cpp"

[symbian]
uid3 = "{uid3}"
capabilities = []
vendor = "symdev"

[signing]
mode = "self-signed"
```

`group/bld.inf`:

```
PRJ_MMPFILES
{name}.mmp
```

(trailing newline)

`group/{name}.mmp`:

```
TARGET {name}.exe
TARGETTYPE EXE
SOURCEPATH ../src
SOURCE hello.cpp
```

(trailing newline)

`src/hello.cpp` / `src/hello.h`: `include_str!("../templates/hello.cpp")` and `hello.h`. Copy those two files from `/home/genius/src/symdev-experiment-5/hello.cpp` and `hello.h` **unmodified**.

- [ ] **Step 1: Failing tests**

```rust
fn scratch() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "symdev-scaffold-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn create_project_writes_hello_tree() {
    let dir = scratch();
    let root = create_project(&dir, "hello").unwrap();
    assert_eq!(root, dir.join("hello"));
    let toml = std::fs::read_to_string(root.join("symdev.toml")).unwrap();
    assert!(toml.contains("name = \"hello\""));
    assert!(toml.contains("uid3 = \"0xef9f2cab\""));
    assert_eq!(
        std::fs::read_to_string(root.join("group/bld.inf")).unwrap(),
        "PRJ_MMPFILES\nhello.mmp\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("group/hello.mmp")).unwrap(),
        "TARGET hello.exe\nTARGETTYPE EXE\nSOURCEPATH ../src\nSOURCE hello.cpp\n"
    );
    assert_eq!(
        std::fs::read(root.join("src/hello.cpp")).unwrap(),
        include_bytes!("../templates/hello.cpp")
    );
    assert_eq!(
        std::fs::read(root.join("src/hello.h")).unwrap(),
        include_bytes!("../templates/hello.h")
    );
}

#[test]
fn create_project_existing_dir_errors() {
    let dir = scratch();
    std::fs::create_dir(dir.join("hello")).unwrap();
    let err = create_project(&dir, "hello").unwrap_err();
    assert_eq!(err.to_string(), "directory `hello` already exists");
}
```

Unit tests in `src/scaffold.rs` cannot use the CLI crate’s `tempfile` dev-dependency. The `scratch()` helper above is required.

- [ ] **Step 2–4:** TDD. `cargo test -p symdev-cli --offline`
- [ ] **Step 5: Commit** `Scaffold a hello project tree from recorded sources.`

---

### Task 3: Wire `symdev new`

**Files:** `crates/symdev-cli/src/main.rs`, `tests/cli.rs`

**Behavior:**
- `Commands::New { name, .. }` → `current_dir` → `create_project` → print absolute root, exit 0. Errors: `eprintln!("error: {e}")`, exit 1.
- Replace `new_not_implemented_creates_no_files`: success creates the §4.2 tree; stdout is the absolute `hello` path; `src/hello.cpp` equals the template; toml contains `uid3 = "0xef9f2cab"`.
- Keep `new_does_not_read_toml`: still must not print `invalid manifest`; now **succeeds** (creates `./hello` beside the bogus parent toml).
- Keep `new_rejects_invalid_name` / `new_rejects_java` unchanged (clap 2).
- Add: second `new hello` in the same parent fails `directory \`hello\` already exists`; first tree unchanged.
- `build` / `package` / `deploy` tests unchanged in this task (`deploy` still not-implemented).

- [ ] **Step 1:** Change CLI tests; watch FAIL on `not implemented`
- [ ] **Step 2–4:** Wire and PASS `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Make symdev new write a hello project tree.`

---

### Task 4: Wire `symdev deploy` print-path

**Files:** `crates/symdev-cli/src/main.rs`, `tests/cli.rs`

**Behavior:**
- Load `symdev.toml`. Invalid/missing: same as `build` (`error: invalid manifest: …`).
- `let sisx = PathBuf::from("build").join(format!("{}.sisx", m.package.name));`
- If not a file: `SISX not found: build/<name>.sisx (run symdev package)`
- Else print absolute path (`cwd.join(&sisx)`), exit 0.
- No uid3 / env / Wine.

Replace `deploy_valid_manifest_not_implemented` with missing-sisx (HELLO has no uid3 — still valid for deploy).

Add dummy-sisx success test: write HELLO + `build/hello.sisx`, assert stdout is that absolute path.

- [ ] **Step 1:** Change CLI test; watch FAIL on `not implemented`
- [ ] **Step 2–4:** Wire and PASS `cargo test --workspace --offline`
- [ ] **Step 5: Commit** `Make symdev deploy print an existing SISX path.`

---

## Self-review

- M4 this slice is scaffold + print-path only. No SSH. No `_reg.rss`. No `.pkg` file in the tree.
- Hello C++ is the recorded public-domain listing.
- uid3 pinned for `hello`.
- Default tests do not need Wine or SDK.
