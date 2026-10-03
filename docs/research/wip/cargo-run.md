# cargo build / run / test — spike + plan (branch `cargo-run`)

Task (lead, 2026-10-03): phase 1 = spike of spec §11
(`docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`), recorded as experiment 114 §1
in `docs/research/experiment-backlog.md`; phase 2 = plan via superpowers:writing-plans at
`docs/superpowers/plans/2026-10-03-cargo-build-run.md`. Scratch: `~/src/cargo-run-scratch/`.
No push to main / merge / tag / publish; pushing `cargo-run` is allowed.

## Status
- [ ] Q1 linker argv + env (dev, release, harness=false test)
- [ ] Q2 same bytes from rustc's link inputs (hello, ui, async)
- [ ] Q3 libcalls as an ordinary dependency
- [ ] Q4 patched std from config alone (rust-std)
- [ ] Q5 dev profile
- [ ] Q6 test binary vs main binary signal
- [ ] exp 114 §1 written
- [ ] plan written + committed

## Facts
- Env: `~/src/cargo-run-scratch/env.sh` (GCCE route, scratch SYMDEV_HOME/XDG, TMPDIR in
  scratch, SYMDEV_RUST_SDK=tree/symbian-rs). `bin/symdev-030` = release build of this branch's
  code (= main 3086f1d). `tree/` = git archive of HEAD (baseline + new-shape builds, same path).
  `sdk1/` = SDK copy with `executables: true`; `q1/app` = new-shape spike project.
- Baseline (`base.sh out/base hello ui async`, symdev-030, rust-lld default, GCCE shims):
  rc=0 all three, 16–17 s each; rust-lld argv in `out/base/<ex>.lld.argv`.
- Q1 recorder `bin/rec-ld` (+ link `bin/symdev-ld`): argv/env/inputs copies in `rec/<run>/`.
  Trimmed fixtures `fixtures/{release,dev}-{bin,test}.{argv,env}`, `run.*`, `test-run.*`.
- Q1 facts: (a) a bin needs `#![no_main]` in src (the attribute keeps `fn main`, E0580
  "main has wrong type" without it). (b) cargo 1.100-nightly uses the NEW build-dir layout:
  `-o <target-dir>/<triple>/<profile>/build/<pkg>/<hash>/out/<name>` (bin: `<name>`, test:
  `<name>-<hash>`), uplifted bin = hardlink `<target-dir>/<triple>/<profile>/<name>`; no `deps/`.
  (c) release argv (13): `<obj>.rcgu.o --as-needed -Bstatic <compiler_builtins rlib> -L
  <out>/rustcXXXXXX/raw-dylibs -Bdynamic -z noexecstack -o <out> --gc-sections --strip-debug`.
  dev: `<out>/rustcXXXXXX/symbols.o`, 24 CGU objs, 11 rlibs (all crates incl core/alloc/cb),
  no `--strip-debug`. (d) linker named `symdev-ld` (stem ends `-ld`) → no `-flavor gnu`; a
  name like `reclinker` → argv starts `-flavor gnu`, else identical. (e) `cargo test` with
  build-std FAILS (E0152 duplicate lang item `sized`: core built twice, panic=abort and not)
  unless `[unstable] panic-abort-tests = true`. (f) env at the linker: main bin has
  CARGO_BIN_NAME=app; test has no CARGO_BIN_NAME but CARGO_TARGET_TMPDIR=<td>/<triple>/tmp
  and CARGO_BIN_EXE_app; both CARGO_CRATE_NAME (app / smoke), CARGO_MANIFEST_DIR,
  CARGO_MANIFEST_PATH, CARGO_PKG_NAME, CARGO_PRIMARY_PACKAGE=1, RUSTUP_TOOLCHAIN. No PROFILE,
  TARGET, OUT_DIR. (g) runner: `cargo run --release -- a b` → `rec-run --exe
  build/cargo/arm-symbian-e32/release/app a b` (RELATIVE uplifted path, cwd = project root);
  `cargo test` → absolute `…/build/app/<hash>/out/smoke-<hash>`; runner env has
  CARGO_MANIFEST_DIR (+ CARGO_BIN_EXE_app for tests), no CARGO_BIN_NAME. ⇒ `<exe>.sisx` next
  to `-o` is NOT next to what `cargo run` hands the runner (uplift is a hardlink of the image
  only). (h) `warning: profile package spec compiler_builtins … did not match any packages`.

## Dead ends

## Next
Read exp 109–113 format, `crates/symdev-build` Rust build path, set up env.

## Coordinator input (mid-task)
`symdev package` refuses `SYMDEV_SIGN_PASSWORD` < 4 chars even for self-signed
(`crates/symdev-build/src/package.rs`, `SisPackage::validate_password`, mirrors makekeys).
Spec says `cargo build` makes a signed `.sisx` → fresh project fails. Plan must carry an
OPEN DECISION for the owner (2–3 options + recommendation), mark the dependent task.
Options to weigh: per-project key password in an ignored file made by `symdev new`;
`cargo build` stops at `.exe` and prints how to set the variable; passwordless self-signed
key ONLY if the original makekeys/signsis are observed to allow it (check first).
