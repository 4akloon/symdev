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

- Q2 driver: `bin/symdev-spike` = 0.3.0 + `SPIKE_RUST_INPUTS` splice in `link_line`
  (`driver-src/`), `q2.sh <out> <release|dev> <ex>…` (cargo with symdev's flags + SYMDEV_UID3
  → rec-ld records inputs; then symdev-spike build with fake-cargo). `e32cmp.py` masks CRC+time.
  Tree examples hello/ui/async converted by `toshape.py` ([[bin]] test=false + `#![no_main]`),
  tree target spec executables=true, `tree/symbian-rs/.cargo/config.toml` linker=symdev-ld.
- Q2 result (release): hello 975/1348, ui 10288/17028, async 18360/33892 — all EQUAL
  (masked) to symdev-030's staticlib images. cargo 7.9–8.1 s (app crate only), link 0.5–1.6 s.
- `symbian-std` reads `option_env!("SYMDEV_UID3")` (test_report) — a plain cargo build has no
  such env; a dependency's build script sees ITS OWN CARGO_MANIFEST_DIR, so only a macro
  expanded in the app crate (or a link-time symbol like `--defsym=symrs_uid3`) can see the
  app's symdev.toml. To verify for the plan.

- Q4 (in `sdk1/symbian-rs/examples/std-hello`, old shape; baseline symdev-030 build 21 s
  materialised `build/rust-src` 82 MB; `out/q4-base.*`):
  H1 `[env] __CARGO_TESTS_ONLY_SRC_ROOT = {value=…, relative=true}` → NO (std from toolchain
  rust-src, cfg_select errors). H2 `[target.arm-symbian-e32] rustflags=["--sysroot",<abs>]` →
  NO (same). H3 toolchain dir `q4/tc-copy`: rustc's sysroot = canonical dir of
  librustc_driver.so — symlinks resolve back to the nightly; copied `bin/rustc`+`bin/cargo`
  + HARDLINKED `lib/librustc_driver-*.so` + symlinks for the rest + `lib/rustlib/src/rust` →
  patched copy ⇒ `--print sysroot` = tc dir, cargo builds core/std from the patched source.
  Via rustup (scratch RUSTUP_HOME `q4/rustup`, `rustup toolchain link symdev-std …`):
  rust-toolchain.toml `channel = "symdev-std"` WORKS; `path = "<abs>"` WORKS (no link
  needed); `path = "<relative>"` → rustup `error: relative path toolchain`.
  H4 `[build] rustc = "./rustc-std"` (config-relative; works from `src/` too) + wrapper
  `exec rustc --sysroot <project>/build/sysroot "$@"`, sysroot = `lib/rustlib/<host>` → nightly's
  + `lib/rustlib/src/rust` → patched copy ⇒ WORKS (12 s), no rustup state, no abs path.
  Path strings in the image change: `build/rust-src/library/std/src/…` (0.3.0) vs
  `build/sysroot/lib/rustlib/src/rust/library/std/src/…` (H4) — staticlib 1 289 862 vs 1 290 330.

- Q4 full run (std-hello bin shape, `.cargo/config.toml` = H4 + linker, plain `cargo build
  --release` 12 s, then spike link): image 70 351 / 124 920 vs 0.3.0 70 441 / 124 212; .text
  equal (0x1902c), .rodata +0x2c8: 36 std path strings `build/sysroot/lib/rustlib/src/rust/
  library/…` vs `build/rust-src/library/…`. With `--remap-path-prefix` of that path in the
  wrapper: 124 200 vs 124 212, .rodata −8 (string-merge order) — the SAME −12 when the OLD
  staticlib shape goes through the wrapper (`out/q4/h4r-lib-stdhello.exe`) ⇒ not bin-vs-lib:
  `cargo -v` shows `-C metadata` of std/core/compiler_builtins/stdhello differ between
  `__CARGO_TESTS_ONLY_SRC_ROOT` and sysroot routes (cargo hashes the std source path) — the
  path-dependence of exp 113 §2. ⇒ rust-std images differ from 0.3.0 by path-dependent bytes.
- Q2 rest (16 no_std, `all.sh`: baseline with the 0.3.0 spec, then bin shape): 14 EQUAL;
  `net` 10 504 vs 10 506 and `tls` 13 938 vs 13 920 — uncompressed EQUAL, compressed differ
  (first diff in code). CAUSE (verified): the target-spec edit. Every crate's unit hash
  changes (two `core` unit dirs in net's build: 268f… old spec, 0b2c… new spec) ⇒ v0 symbol
  hashes change ⇒ .text item order (net: `with_session` ↔ `to_socket_addrs::first` swap);
  sections equal in size, .rodata/.data equal. net & tls rebuilt by symdev-030 in the OLD
  staticlib shape with the NEW spec (`out/spec/`) = the bin-shape images EXACTLY. So rustc's
  inputs + bin shape = staticlib bytes, 19/19 no_std examples; the spec edit alone moves 2.

- Q3 (`q3.sh` = q2 + `SPIKE_NO_LIBCALLS=1`, ref out/q2; `addlibcalls.py` adds the path dep +
  `use symbian_libcalls as _;`; tree restored after). Exe compressed/uncompressed vs today:
  | var | hello | files | atomics | async | tls |
  | V1 plain dep (LTO'd) | 1708/3384 (+2036) | 9036/16276 | 8891/15944 | 18461/33976 | 14002/25968 |
  | V2 + `#![no_builtins]` | EQUAL | 8976/16324 | 8841/16000 | 18447/34048 | 13976/26008 |
  | V3 + `[profile.release.package.symbian-libcalls] codegen-units=16` | EQUAL | 8300/14356 | 8732/16000 | 18356/34048 | 13913/26140 |
  | V4 + `-Zprofile-rustflags` hidden | = V3 (flag reached rustc, -v) |
  today: hello 975/1348, files 8289/14292, atomics 8796/15848, async 18360/33892, tls 13938/25920.
  Why: V1 its no_mangle entry points are exported globals of the `-shared` link → gc roots.
  V2 rustc then passes `libsymbian_libcalls-<h>.rlib` on the line (not LTO'd) but built with the
  app profile (cgu=1). V3/V4: cargo gives every dep of a fat-LTO bin `-C linker-plugin-lto`
  (seen in -v) ⇒ the excluded crate's object code comes from the pre-link pipeline: members
  carry `.llvmbc`, `AtomicLock::drop` is no longer inlined, each `__atomic_*` +4 B (objdump of
  `__atomic_exchange_1`: `bl …AtomicLock…drop` vs inlined `RFastLock::Signal`). `lto` cannot be
  set per package (cargo). ⇒ NOT an ordinary dependency without byte changes.
  Alternative tested: the linker runs 0.3.0's own `cargo rustc --profile libcalls …` NESTED
  inside cargo's build, same `--target-dir build/cargo` (`bin/nested-libcalls` via
  REC_THEN): no lock wait, no deadlock; 7.1 s first time (core/alloc for the libcalls profile),
  0.03 s fresh; outer build 14 s / 0.2 s. Same invocation ⇒ same rlib ⇒ same bytes.

## Dead ends
- (none yet beyond Q4's H1/H2)

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
