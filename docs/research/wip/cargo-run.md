# cargo build / run / test — spike + plan (branch `cargo-run`)

Task (lead, 2026-10-03): phase 1 = spike of spec §11
(`docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`), recorded as experiment 114 §1
in `docs/research/experiment-backlog.md`; phase 2 = plan via superpowers:writing-plans at
`docs/superpowers/plans/2026-10-03-cargo-build-run.md`. Scratch: `~/src/cargo-run-scratch/`.
No push to main / merge / tag / publish; pushing `cargo-run` is allowed.

## Status
- [x] Q1 linker argv + env   - [x] Q2 same bytes (19/19 no_std)   - [x] Q3 libcalls
- [x] Q4 patched std (H4)    - [x] Q5 dev profile                 - [x] Q6 test signal
- [x] exp 114 §1 written (experiment-backlog.md, end of file)
- [x] plan written + committed: docs/superpowers/plans/2026-10-03-cargo-build-run.md, 19 tasks,
  D1 (signing) open for the owner. NEXT: hand back to the lead; execution waits for the
  owner's review of the plan and D1.

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

- Q5 dev profile (`q2.sh … dev`; tree [profile.dev] = panic abort only):
  * default dev: argv = symbols.o + 24 CGU objs + 11 rlibs (core/alloc/cb too); objects carry
    DWARF (`-C debuginfo=2`). 0.3.0's line FAILS: `undefined symbol: strlen` from
    `alloc::ffi::c_str::CString::from_raw` (hello, async), `symrs_list_destroy` (ui): every
    GLOBAL DEFAULT symbol of a pulled rlib member is a gc root in the `-shared` link.
  * dev + `lto = true` (`out/q5a`): links; hello 16 940/44 772, ui 49 420/131 308, async
    80 104/230 516 (release 975/1 348 …). ELF has no .debug_* (0.3.0's line has `--strip-debug`).
    Kept DWARF (`SPIKE_KEEP_DEBUG`, `out/q5a-dbg`): hello.elf 3 751 928 B vs 202 656 (.debug_info
    1 082 485, .debug_str 1 376 786, .debug_line 572 056, …); OUR elf2e32 ACCEPTS it and the
    .exe is byte-EQUAL to the stripped one (hello, ui).
  * target spec `"default-visibility": "hidden"` (key name per rustc's list; `default-hidden-
    visibility` is refused as unknown field): release images EQUAL to out/q2 for hello, ui,
    async, net, tls (`out/q5d`); default dev WITHOUT lto now LINKS (`out/q5e`): hello
    16 908/44 568, ui 49 390/131 140, async 79 878/230 504.
  * EKA2L1: dev hello (q5e image) → `Trying to display: Hello from Rust SDK (19 chars)`,
    `shots/dev-hello-1.png`, pid 71654 killed by runshot (`emu.sh`, `runshot.py` in scratch).

- Q5d rest: `default-visibility: hidden` release images EQUAL to out/q2 for all 19 no_std
  (`q5d-rest.txt` + the 5 above).
- Q6: example target (`--example demo`, `rec/q6-example`): CARGO_BIN_NAME=demo, no
  CARGO_TARGET_TMPDIR, `-o <out>/demo` (no hash). Fixtures `release-example.*`,
  `release-bin-flavor.argv` (`-flavor gnu` first). Rule: CARGO_BIN_NAME set ⇒ bin or example
  (main = the [[bin]] = package name); unset + CARGO_TARGET_TMPDIR set ⇒ integration test.
- UID3 at compile time: `strings!()` (symbian-macros/src/lib.rs:134) already reads the APP's
  CARGO_MANIFEST_DIR in a proc macro (observed working, exp 96/99) ⇒ a macro can read
  symdev.toml; `test_report::Report::new` uses `option_env!("SYMDEV_UID3")` (→ 0 without it).
- Signing (coordinator item), ORIGINAL tools via Wine (`~/src/cargo-run-scratch/signing/`):
  `makekeys -cert -expdays 3650 -len 2048 -dname … nopw.key nopw.cer` WITHOUT -password →
  "Warning: the private key should be encrypted with the -password option" / "Do you want to
  use a password (y/n)?"; stdin EOF → "Enter PEM pass phrase:" → "** Error writing to key
  file"; answering `n` → "Created key/certificate", nopw.key 1 192 B `BEGIN DSA PRIVATE KEY`
  with NO `Proc-Type: 4,ENCRYPTED`. `signsis hello.sis hello-nopw.sisx nopw.cer nopw.key`
  (4 positionals, no passphrase) → exit 0, 5 180 B, `file`: Symbian installation file. Usage
  prints `[-password <password> <At least 4 characters>]` (optional). Not installed anywhere.

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

## Phase 2 — plan (docs/superpowers/plans/2026-10-03-cargo-build-run.md), written in chunks
Code facts gathered: CLI = `crates/symdev-cli/src/main.rs` (209 l, `Commands` in cli.rs;
`package_project`, `run_project` in main.rs; `package_artifacts` in artifacts.rs assumes
`cwd/build`); `RustBuild` (driver/rust_build.rs) fields gcce/sdk/cargo/rustc/name/std/linker/ui;
`LinkInputs{archive,shims,libcalls}`; `link_line(linker, archive, shims, libcalls, elf, map)`;
`shim_archives(project,cwd,prebuilt)` writes under build/; `SisPackage` (package.rs) generates
an UNENCRYPTED PKCS#8 key per package when no pair — password unused for it, yet
`validate_password` demands ≥4. Emulator crate: `Eka2l1Backend`, `EmulatorData`, `TestReport`
(schema 1, ignores unknown fields), crate-private `Json` parser. No `unsafe` in host crates;
`ctrlc` not in the offline cache (libc 0.2.189 and toml 1.1.6 are). install.sh lives in
`~/projects/symdev-packages`. EKA2L1: control README `~/src/EKA2L1-wt/control-server/src/emu/
control/README.md` (dev/control-events; protocol 1: emulator.info, apps.list, app.launch,
app.kill, package.install/remove, events.subscribe app_exited, exit_type kill/terminate/panic);
`--data-dir` on dev/data-dir (a3ec972); the `symdev` integration branch has NEITHER yet.
Log filter key `log-filter`, `Emulated.Stdout:trace` for RDebug, `Kernel:trace` for panics.
Task list (19): 1 fixtures+LinkerArgs/LinkKind/CargoOutput; 2 link rustc inputs (symdev-build);
3 packaging type + D1 signing decision; 4 symdev-ld role + setup-linker; 5 target spec +
uid3!/report! macros; 6 symbian-test crate; 7 project shape + symdev new; 8 migrate 21 examples;
9 rust-std via symdev-rustc + sysroot; 10 EKA2L1 build with --control/--data-dir + profiles;
11 device registry + choice + `symdev devices/emulator`; 12 control client; 13 runner
(`symdev run --exe`, `symdev run`); 14 cargo test path (report, libtest printer, symdev test);
15 symdev build = cargo build; 16 old-shape error; 17 CI; 18 exp 114 real runs; 19 acceptance.
- Plan progress: header, D1, review focus, file map, Tasks 1–6 written and committed.
  NEXT: Task 7 (scaffold) … 19. Design notes for the rest are in the task list above;
  runner liveness test trick: a symlink named `eka2l1-fake` → /bin/sleep gives /proc comm.

## Phase 3 — execution (D1 = A; one agent, superpowers:executing-plans, tasks 1 → 19)
Ledger (git-ignored): `.superpowers/sdd/2026-10-03-cargo-build-run/progress.md`. Tick helper:
`~/src/cargo-run-scratch/exec/tick.sh N`. Scratch for this phase: `~/src/cargo-run-scratch/exec/`.
- Task 1 done: `ld/` types + fixtures (hashes match). Ruling: `mod ld;` gets
  `#[allow(dead_code, unused_imports)]` (plan said dead_code only; the re-exports warn) — Task 4
  removes it.
- Task 2 done: `RustcLink` + `link_rustc_output`; `link_args`/`link_line` take `&[PathBuf]`;
  shims under a work dir. `exec/t2-images.sh`: hello/ui/async 0.3.0 vs t2 EQUAL. Ruling:
  `shim_object`/`shim_archive` take `work: &Path` and `shim_archives`/`build_shims` drop
  `project` (would be unused) — the plan kept `project` in their signatures.
- Task 3 done: `ProjectPackage` (sisx.rs), `package_artifacts` takes resources from the image's
  directory; D1 = A in `SisPackage::package` (password only for an encrypted `[signing] key`).
  Old test `package_short_password_errors_before_tools` replaced by the two D1 tests; cli
  `package_missing_sign_password` → `package_without_a_password_signs_with_a_generated_pair`.
  READMEs (root, examples, symdev-cli) updated to the new rule.
- Task 4 done: `Role`, `RustProject`, `LinkRecord`, `LinkRun`, `setup-linker`. Real link
  (`exec/t4-link.sh`, bin4/ = setup-linker of the release build): plain cargo build of the
  spike's bin-shape hello, 11 s, EQUAL 975/1348 to out/q2, `.sisx` at `-o`, profile dir and
  build/ — with NO SYMDEV_SIGN_PASSWORD (D1 = A). Rulings: help test now lists setup-linker;
  `LinkRecord::read` carries `#[allow(dead_code)]` until Task 13; step 5 adds
  `--target-dir build/cargo` + `SYMDEV_UID3` (q2's flags; the spike tree's config has no
  target-dir, and uid3!() comes in Task 5).
- Task 5 done: spec `executables: true` + `default-visibility: hidden`; `uid3!()` (proc macro,
  `ManifestUid3`) + `report!`; `Report::new`/`uid3_from_env`/`parse_hex_u32` gone; 15 examples
  use `report!`; `symdev build` no longer sets SYMDEV_UID3. Byte check (`exec/t5-images.sh`,
  built at the spike tree's path, tree restored; new tree kept as `tree-t5/`): async
  18284/33772 vs q5d 18360/33892, files 8189/14176 vs 8289/14292, atomics 8729/15728 vs
  8796/15848 — NOT equal, EXPLAINED: only `E32Main` differs (async 0x6d4 → 0x66c, files
  0x148c → 0x1428, atomics 0x1494 → 0x142c; every other symbol equal in size) and .rodata −12.
  `Report::new` called `uid3_from_env()` in a non-const context, so the optimiser left a
  runtime hex parse of the string "0xe0000687" (present in q5d's ELF, gone now); `uid3!()` is
  a literal (the constant 0xe0000687 now appears once in the ELF, 0 times before). Smaller,
  same value. Ruling: `uid3!()` expands to `{ const _: &str = include_str!("<dir>/symdev.toml");
  0x…_u32 }` so rustc tracks symdev.toml (as `strings!()` tracks locales).
- OPEN (found in Task 5): plain cargo does not relink when only symdev.toml changes
  (capabilities, vendor, version, [ui] text): cargo sees no input change, and symdev-ld reads
  symdev.toml. Candidate fix: `#[symbian_std::main]` emits the same discarded
  `include_str!` of symdev.toml, so every app crate depends on it. Decide in Task 7.
- Task 6 done: `symbian-test` (Case, Evidence, ensure, __run), `#[symbian_test::tests]`
  (`TestModule`, text scanner; lifetimes and strings guarded, 2 extra tests), report case
  `state` pending/running + `pending/running/settle/save`. Compile check of the expansion in
  `exec/t6check` (scratch lib crate, phone target): builds. Recipes list no crates → unchanged.
- Task 7 done: scaffold writes [[bin]] (test = false), [[test]] smoke (harness = false),
  dev-dep symbian-test, config panic-abort-tests + linker/runner, tests/smoke.rs; hello example
  is a bin with `#![no_main]`; symbian-rs/.cargo/config.toml has linker/runner. Real project
  (`exec/t7-project.sh`, bin7/ = branch symdev + setup-linker): `symdev new t7`, `cargo build
  --release` 15.8 s, `cargo test --no-run` 8.7 s; `.sisx` at build/, profile dir, and
  `out/smoke-<hash>.sisx`. Findings → two extra changes (commit after 764d94e):
  (1) `cargo test` links the main bin in the dev profile too and overwrote build/t7.sisx
  (18 152 B debug over 2 300 B release) → `CargoOutput::is_release`; build/ copies only from
  release. (2) OPEN closed: `ManifestDependency` — `#[main]` and `uid3!()` emit a discarded
  `include_str!` of symdev.toml; `exec/t7-relink.sh`: editing only `vendor` → `Compiling t7`,
  new .sisx; image EQUAL 975/1348.
- Task 8 done: toshape.py on the 18; plain `cargo build --release` (`exec/t8-build.sh`, log
  `exec/t8-build.log`): rc=0 for all 19, 8–16 s each. vs out/q2 (spike, other path): EQUAL
  alloc hello hello-raw panic shim spawnee (no report); the 13 report examples DIFF by Tasks
  5+6 (measured on async: E32Main 0x66c (T5) → 0x720 (+180: report `state` in json, finished()
  filters), `Report::record` +8, .rodata +12 `,"state":"`; net vs q2: +60 uncompressed).
  `symbian-rs/examples/README.md` did not exist → created (short how-to; Task 19 updates it).
- Task 9 done. Step 1 observation (sdk1 std-hello, wrapper script logging $0, `[build] rustc =
  "build/symdev-rustc"`): cargo starts it as the ABSOLUTE `<project>/build/symdev-rustc`, 43×
  from the project root and 3× from `src/` (touch + rebuild). `StdSysroot` (build/sysroot:
  `StdSrc::materialise_into` lib/rustlib/src/rust + link lib/rustlib/<host> via
  `RustLld::in_sysroot`'s host parse; link build/symdev-rustc → current_exe), `RustcWrapper`
  (exec rustc --sysroot <dir of argv0>/sysroot; errors without a sysroot, naming `symdev
  build`); std-hello/std-net in bin shape with the plan's config. Real std build is Task 15.
NEXT: Task 10 — `task-start <plan> 10` (EKA2L1 with --control/--data-dir; profiles).
- Task 10 in progress. EKA2L1: own copy `~/src/EKA2L1-wt/cargo-run` (cp -a of integration,
  branch `cargo-run` = symdev d07d5ac + merge dev/data-dir a3ec972 (a023886) + merge
  dev/control-events c323b64 (a5d9df3); conflicts only in qt/src/thread.cpp and
  qt/include/qt/cmdhandler.h option lists — both options kept). Ruling: the user's limit (own
  copy per topic) overrides the plan's "merge into integration's symdev". Build dir
  `~/src/EKA2L1-wt-build/cargo-run`, script `build.sh` there (configure like integration),
  background PID 130544, log `build.log` (ends EXIT=<rc>).
  Build EXIT=0 (1458 steps). Wrapper `~/src/cargo-run-scratch/bin/eka2l1-symdev` (= ~/.local/bin/eka2l1
  env, exec the cargo-run build); `--help` lists `--data-dir` and `--control`.
  INCIDENT: that `--help` ran with no `--data-dir`, so the frontend used the user's default
  folder and ROTATED its logs (~/.local/share/EKA2L1/EKA2L1.log rewritten 18:36:01; the old
  EKA2L1_TakeThis.log of 16:39, 48 067 B, was replaced by the 17:03 log). config.yml and data/
  untouched. Lesson: always pass `--data-dir <scratch>`, even for `--help`. Report to the owner.
  Step 2 observed (exp 114 §2 written): the hand profile (links for ROM + Z, copied C,
  empty D/E, config with our log-filter) works; user's folder untouched in both runs; hello
  exits by itself (kill/0/None), panic example → exit_type panic, -2, RUST; panic log line
  `T …thread.cpp:542 [Kernel]: Thread Main panicked with category: RUST and exit code: -2 `.
  No Rust code calls RDebug → no Emulated.Stdout line seen. Socket file stays after kill -9.
- Task 10 done: `device::EmulatorProfile` (at/dir/name/log_file/data/create/root_from_env),
  `EmulatorData::root()`. Extra tests: root/xdg, drive-C link leaving C refused.
- Task 11 done: DeviceId, RegistryEntry (TOML via toml::Table), DeviceRegistry (live never
  signals; drops non-EKA2L1 PIDs and silent sockets), is_eka2l1, DeviceChoice/Choice/Offer,
  DevicePrompt. Tests split into device/tests/{choice,profile}.rs (+2 extra: id, round trip).
- Task 12 done: control/{Request,Param,ControlClient,AppExited,ExitType,EmulatorInfo},
  json::quote, device::EmulatorInstance (argv/start/stop/has_control), CLI `devices`,
  `emulator start|stop` (devices_cmd.rs, `Devices` shared with the runner). Observed: `--help`
  prints the options and then does NOT exit (40 s, killed); any run without --data-dir uses the
  default folder → has_control runs `--data-dir <tmp> --help` with HOME/XDG_* in tmp, reads ≤15 s,
  kills its own child. Real run (`exec/t12-real.sh`, flock, XDG_DATA_HOME=scratch/xdg12):
  `emulator start rm-469` → "created profile rm-469", `emulator-1` in 0.5 s; `devices` →
  `emulator-1  Nokia N00 (RM-469)  pid 161383  profile rm-469`; `emulator stop emulator-1` →
  pgrep empty; user's EKA2L1 folder unchanged.
- Task 13 done: ctrlc 3.5.2 fetched online once (commit "Add ctrlc"); run/{ExeTarget, AppExit,
  Interrupt, LogTail, Runner, pick_device}; `symdev run [--exe <image>] [args→refused]`;
  run_project gone. Fake device for tests: a `#!/bin/sh` script named eka2l1-fake doing `read`
  on a held pipe (this host's `sleep` is uutils multicall: refuses another name), spawn retried
  on ETXTBSY. Tests isolate XDG_RUNTIME_DIR now. Smoke (`exec/t13-smoke.sh`, flock): in t7,
  `cargo run --release` → created profile, started emulator-1, app ran, rc=0 in 6.8 s; second
  `cargo run` rc=0 in 5.3 s (hello waits 5 s itself); `symdev emulator stop` → no eka2l1_qt left.
- Task 14 done: CaseState in TestReport; TestOutcome/CaseLine/Verdict; LibtestPrint; Runner
  test path (clear report, follow, await_report 5 s, settle, libtest lines, status 0 iff pass;
  a report whose cases are all finished also ends the wait — Avkon examples report and keep
  running); `symdev test --emulator` on the same runner; Eka2l1Backend reduced to from_env;
  examples/async gets tests/executor.rs. Real (`exec/t14-real.sh`, flock): `cargo test
  --release` in examples/async → emulator-1 started, `test block_on_returns_what_the_future_produced
  ... ok`, `test result: ok. 1 passed; 0 failed`, rc=0 in 0.7 s; emulator stopped.
- Task 15 done: `CargoBuild` (args, linker_on_path, run); `symdev build` of a Rust project =
  resolve → prepare → check_link (rust-lld + lld script before cargo, as 0.3.0 did) → rust-std
  StdSysroot → symdev-ld on PATH → `cargo build --release` → print build/<name>.{exe,sisx}
  (missing after a fresh cargo → error). RustBuild lost BuildBackend/cargo_args/archive/
  run_cargo/build_std; StdSrc lost SRC_ROOT_ENV/dir_for/SYMDEV_RUST_STD_SRC (rust-src README
  rewritten). Tests updated: rust_linker (symdev-ld stub on PATH), rust_sdk (asserts the
  build/rust-sdk link instead of the old --target argv). Real (`exec/t15-real.sh`): hello,
  ui rc=0 (fresh), std-hello rc=0 19 s, `Compiling std v0.0.0 (…/build/sysroot/lib/rustlib/
  src/rust/library/std)` — Task 9's real check passes.
- Task 16 done: `OldShape` (detect on `"staticlib"`, numbered edits for Cargo.toml, main.rs,
  config); `symdev build` refuses it first, before provisioning or cargo (integration test with
  a marker-writing cargo on PATH).
- Flake seen once (gates run after b4c0c1e): run_exe's cargo_test_prints… and ctrl_c… failed
  with "SYMDEV_DEVICE=emulator-1 is neither a running emulator nor a profile; there are: none"
  (the runner's liveness dropped the fake). Not reproduced in 5 full runs since. Stress:
  `exec/stress-run-exe.sh 40` → `exec/stress.out`, failing logs in `exec/stress/`.
- Task 17 done. CI (4971763): examples job links symdev-ld/symdev-rustc with setup-linker
  into ~/.local/bin, builds C++ examples with build+package and every symbian-rs example with
  `symdev build` (= cargo), uploads symbian-rs/examples/*/build/*.sisx, and no longer sets
  SYMDEV_SIGN_PASSWORD (D1 = A); YAML validated. Packages: worktree
  ~/worktrees/symdev-packages/cargo-run, branch cargo-run (from main d81ddee), commit 3fe6a76
  (NOT pushed): install.sh checks then links symdev, symdev-ld, symdev-rustc → the installed
  binary (same replace/refuse rules; a foreign file at any of them leaves all three); tests
  for it in tests/install.sh.test (dash + bash: all passed); recipes/symdev/0.4.0 = 0.3.0's with
  0.4.0 ids/tag and commit 000… (build.sh refuses until set at release); README. Packages
  `cargo test`: 167 passed — but 1 fails when the developer's PUBLISH_SIGNING_KEY is in the env
  (publish's dry-run test expects an unsigned index; pre-existing, environmental).
NEXT: Task 18 — `task-start <plan> 18` (experiment 114 real runs).
- Task 18 in progress. Step 1 done (`t18/bytes.sh` → `t18/bytes.out`, `t18/symdiff.py` →
  `t18/symdiff.out`, `t18/cold.sh`): 21/21 built both ways at one path; EQUAL alloc hello
  hello-raw panic shim spawnee; 13 report-using no_std examples differ only in .text +60..+192
  (Report::record +8 and the function that inlines report!/finish: E32Main etc. — Task 5 −~110
  plus Task 6 +~170–300); std-hello .text −192 .rodata +672, std-net −204/+504 (path strings).
  Times: cold hello 15.5 s / warm 0.9 s; cold ui 16.5 s / warm 2.1 s. Next: `t18/runs.sh` under
  flock (steps 2–5), bin7 = HEAD 6d75fbb.
- Task 18 done: exp 114 §3 (bytes), §4 (run/test) and Conclusion written. Runs: hello 2.0 s
  to the note from cold (status 0, 6.6 s), second run 0.51 s; ui Bars → F1 F1 → cmd=1, Ctrl+C
  130 + emulator alive; cargo test broken: ok/FAILED/FAILED(panicked: RUST -2)/not run, 101;
  several devices error; SYMDEV_DEVICE=emulator-2 works. Runner now also prints InfoPrint notes
  (6d75fbb). run_exe flake root-caused (comm set after CLOEXEC pipe closes) and fixed (48a4873).
  OPEN: two instances of one profile share its data folder.
NEXT: Task 19 — `task-start <plan> 19` (acceptance, 0.4.0, gates, final review).

## Final review and fix pass (2026-10-03)
Fresh reviewer (opus): "With fixes". Fixed with RED→GREEN tests: Critical 1 (cargo test in an
Avkon project: test links packaged with [ui]), Important 1 (symdev package of a Rust
[ui]/locales project: release link now keeps resources in build/), Important 2 (control-client
call timeouts 120 s / probe 3 s; silent EKA2L1 kept registered, `devices` shows "(not
answering)", `emulator stop` reaches it; second Ctrl+C exits 130), re-graded minors: panic after
the last case fails the run; OldShape names symdev-rustc for rust-std; help probe scratch in
$XDG_RUNTIME_DIR. Real checks (`t18/fixes.sh`): `cargo test --release` of a smoke test in
examples/ui on the emulator → ok; after a relink `symdev build && symdev package` in ui → rc 0.
Final gates: 943 passed, clippy 0, fmt ok, macros 36 passed.
Deferred minors and every ruling: the ledger copy below.

### Ledger (copied from the git-ignored .superpowers/sdd/…/progress.md)
# SDD ledger — plan: docs/superpowers/plans/2026-10-03-cargo-build-run.md

Spec: docs/superpowers/specs/2026-10-03-cargo-build-run-design.md (read). D1 = option A (owner).
Executor: superpowers:executing-plans, one agent, tasks 1 → 19 in order.

Pre-flight (Interfaces blocks):
- T1 → T4: LinkerArgs/CargoLinkEnv/LinkKind/CargoOutput — names match.
- T2 → T4: RustcLink, RustBuild::link_rustc_output — names match.
- T3 → T4: ProjectPackage::new/package — names match.
- T5 → T6: symbian_std::uid3! — matches.
- T4 → T13: LinkRecord/LinkKind — matches.
- T10 → T11 → T12 → T13: EmulatorProfile, DeviceRegistry, DeviceChoice, ControlClient, EmulatorInstance — match.
- T6 + T13 → T14: report `state`, Runner/ExeTarget/AppExit — match.
- T15 → T16: CargoBuild → OldShape — match.
No conflicts found at block level; each brief is checked when read.
Task 1: Ruling: `mod ld;` needs #[allow(dead_code, unused_imports)], not dead_code alone — the pub(crate) re-exports warn as unused imports until Task 4 — cost if wrong: none, Task 4 removes it
Task 1: complete (commits d57eeaf..0d98b3c, tests: cargo test -p symdev-cli --offline --bin symdev ld::tests:: → test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 28 filtered out; finished in 0.00s)
Task 2: Ruling: shim_object/shim_archive take the work dir instead of &Project, and shim_archives/build_shims drop the project parameter — after the change it is unused (warning) — cost if wrong: a signature rename, no consumer outside symdev-build
Task 2: complete (commits 0d98b3c..0e7222a, tests: cargo test -p symdev-build --offline → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 3: Ruling: the old validation test package_short_password_errors_before_tools (refusal for a generated pair) is replaced by a_generated_self_signed_pair_needs_no_password; READMEs (root, examples, symdev-cli) now say the password is only for an encrypted [signing] key — follows D1 = A — cost if wrong: doc wording
Task 3: complete (commits 0e7222a..29fd384, tests: cargo test --workspace --offline → test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 4: Ruling: tests/help.rs's command list gains `setup-linker` (renamed help_lists_only_the_known_commands) — the plan adds a subcommand but did not name this test — cost if wrong: a test rename
Task 4: Ruling: LinkRecord::read carries #[allow(dead_code)] until Task 13's runner reads it; LinkRun::run uses Self::work_for so it is not test-only — zero-warning gate — cost if wrong: none
Task 4: Ruling: main's `exit` takes Result<ExitCode, Error> (not Result<()>) so the CLI tail reuses it; the linker branch maps () to SUCCESS — cost if wrong: none
Task 4: Ruling: step 5's real link adds `--target-dir build/cargo` and `SYMDEV_UID3` (as q2.sh did) — the spike tree's config sets no target-dir, so the plan's compared path would not exist — cost if wrong: none, verification only
Task 4: complete (commits 29fd384..e034d4e, tests: cargo test -p symdev-cli --offline → test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 5: Ruling: uid3!() expands to a block with a discarded include_str! of symdev.toml, not a bare literal — so rustc tracks the file and an edited UID3 recompiles — cost if wrong: none in bytes (const discarded); not usable as a pattern
Task 5: Ruling: step 4's byte check is NOT equal to q5d (E32Main −100..−104 B, .rodata −12) — explained: Report::new parsed SYMDEV_UID3's text at run time; uid3!() is a literal. Accepted as an improvement, recorded in the wip notes — cost if wrong: q5d no longer a byte reference for report-using examples
Task 5: Ruling: the t5 build copies the working tree with `git ls-files -co` (not archive+diff), because new files are untracked — verification only
Task 5: complete (commits e034d4e..b74bd5b, tests: cargo +nightly-2026-09-19 test --offline --manifest-path symbian-rs/crates/symbian-macros/Cargo.toml → all doctests ran in 0.09s; merged doctests compilation took 0.08s)
Task 6: Ruling: added tests a_lifetime_is_not_a_char_literal and a_test_attribute_inside_a_string_is_left_alone (the brief asked for the lifetime one); the scanner works on chars, so 'é' is a char literal too — cost if wrong: none
Task 6: complete (commits b74bd5b..f036b06, tests: cargo +nightly-2026-09-19 test --offline --manifest-path symbian-rs/crates/symbian-macros/Cargo.toml --lib → test result: ok. 35 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 7: Ruling: the `# No libtest on the phone` comment sits above [[bin]], not inside it — the plan's own test expects the four [[bin]] lines contiguous — cost if wrong: comment placement
Task 7: Ruling: build/<app>.exe and build/<name>.sisx are written only by a release link (CargoOutput::is_release) — observed: `cargo test` links the main bin in the dev profile and overwrote build/t7.sisx with the 18 KB debug package — cost if wrong: a dev-only workflow gets no build/<name>.sisx (the profile dir still has <bin>.sisx)
Task 7: Ruling: #[symbian_std::main] and uid3!() emit a discarded include_str! of symdev.toml (ManifestDependency) — without it an edit to symdev.toml alone never relinks under plain cargo (observed fix: vendor edit → Compiling t7, image bytes equal) — cost if wrong: none in bytes; a crate with #[main] recompiles on every symdev.toml edit
Task 7: complete (commits f036b06..90fa497, tests: cargo test -p symdev-cli --offline --bin symdev → test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 7: Ruling: rust_sdk::tests::checkout_sdk_is_found_and_has_the_target pinned the ABSENCE of no_main in HELLO_MAIN; flipped to require #![no_main] (the plan changed HELLO_MAIN). Found by the workspace gate after the Task 7 commit; fixed in a follow-up commit — cost if wrong: none
Task 8: Ruling: symbian-rs/examples/README.md did not exist; created a short how-to (the plan lists it as Modify, Task 19 updates it) — cost if wrong: one small doc file
Task 8: Ruling: the plan's "size equals experiment 113's column" no longer holds for the 13 report-using examples: Task 5 (−100..−120 B, uid3! literal) and Task 6 (+~200 B, report case state) moved them; the 6 others are EQUAL to exp 114's images even at another path. Measured and recorded; Task 18 does the one-path comparison — cost if wrong: none
Task 8: complete (commits 3ecda90..6b819ab, tests: bash -c 'grep -c "rc=0" ~/src/cargo-run-scratch/exec/t8-build.log | grep -qx 19 && echo "19/19 examples rc=0"' → 19/19 examples rc=0)
Task 9: Ruling: the fixture returns its TempDir as a 4th value (the plan's 3-tuple would drop it), fakes the SDK (RustSdk::REQUIRED + an overlay that replaces nothing) as well as the toolchain — the real overlay's SHA-1 check needs the real toolchain — cost if wrong: none
Task 9: Ruling: added tests a_second_materialise_replaces_the_first and symdev_rustc_without_a_sysroot_says_how_to_make_one; RustcWrapper refuses a missing sysroot by name rather than letting rustc fail obscurely — cost if wrong: none
Task 9: Ruling: argv0 observed absolute (both from root and src/); RustcWrapper still joins it to cwd (a no-op for an absolute path) — cost if wrong: none
Task 9: complete (commits 6b819ab..1dab510, tests: bash -c 'cargo test -p symdev-build --offline std_sysroot 2>&1 | grep -m1 "test result" && cargo test -p symdev-cli --offline --test rustc_wrapper 2>&1 | grep -m1 "test result"' → test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 10: Ruling: EKA2L1 built in an own copy ~/src/EKA2L1-wt/cargo-run (branch cargo-run = integration's symdev + both merges), build ~/src/EKA2L1-wt-build/cargo-run — the user's limit (own copy per topic) overrides the plan's "merge into integration/symdev" — cost if wrong: one more 1.6 GB tree
Task 10: Ruling: running `eka2l1-symdev --help` (the plan's check) without --data-dir rotated the user's EKA2L1 logs (EKA2L1.log → EKA2L1_TakeThis.log; the previous TakeThis of 16:39 is gone); config and data untouched. Reported; every later run passes --data-dir — cost: one lost old log of the user's
Task 10: Ruling: the probe ran the emulator inside one flock'd script (start, probe, kill -9 own PID) instead of the plan's flock around a backgrounding sh, so the lock covers the whole run — cost if wrong: none
Task 10: Ruling: RDebug line shape recorded from the log pattern + svc debug_print class (T … [Emulated.Stdout]: text), not observed — no Rust SDK code calls RDebug — cost if wrong: the runner's stdout filter may need adjusting when a C++ app prints
Task 10: Ruling: EmulatorData gains root(); EmulatorProfile::root_in is the pure half of root_from_env (tested) — cost if wrong: none
Task 10: complete (commits 1dab510..d374c7e, tests: cargo test -p symdev-emulator --offline device → test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.00s)
Task 11: Ruling: device tests split into device/tests/{choice,profile}.rs now (not only past 300 lines) so each task's tests stand alone; two extra tests (DeviceId edge cases, RegistryEntry TOML round trip) — cost if wrong: none
Task 11: complete (commits d374c7e..da8ed2c, tests: cargo test -p symdev-emulator --offline --lib → test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s)
Task 12: Ruling: has_control does not run a bare `<eka2l1> --help`: observed, --help prints and then never exits, and any run without --data-dir rotates the default folder's logs. It runs `--data-dir <tmp> --help` with HOME/XDG_* in tmp, reads stdout ≤15 s for a `--control` line, then kills its own child — cost if wrong: up to 15 s on an EKA2L1 without --control
Task 12: Ruling: EmulatorInstance::stop takes the registry too (stop(&entry, &registry)) — it must remove the entry and the plan's signature had no registry — cost if wrong: none
Task 12: Ruling: the socket lives in the registry dir's parent ($XDG_RUNTIME_DIR/symdev/<id>.sock) as the plan says; readiness = info() names a device AND apps.list answers (any error retried until 120 s, child exit reported with its last output lines) — cost if wrong: none
Task 12: Ruling: help test gains `devices` and `emulator`; "created profile <fw>" goes to stderr — cost if wrong: none
Task 12: complete (commits da8ed2c..30cee56, tests: cargo test -p symdev-emulator --offline --lib → test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.50s)
Task 13: Ruling: the fake device is a shell script named eka2l1-fake blocked on `read` of a held pipe (not a link to sleep): this host's sleep is a uutils multicall binary that refuses to run under another name (observed "Security violation: Requested utility `eka2l1-fake`"); its spawn retries ETXTBSY (parallel test threads fork while the script is open) — cost if wrong: none
Task 13: Ruling: tests/common isolates XDG_RUNTIME_DIR too, so no test reads or prunes the developer's real device registry — cost if wrong: none
Task 13: Ruling: tests/run.rs's run_without_emulator_names_symdev_eka2l1 became run_with_a_profile_to_start_and_no_emulator_names_symdev_eka2l1 (+ run_without_any_device_says_to_install_a_firmware): with the device choice, SYMDEV_EKA2L1 is only needed to start a profile — cost if wrong: test names
Task 13: Ruling: the Ctrl+C handler is installed only after the device is picked and connected (a Ctrl+C at the prompt or during an emulator start still ends symdev); the log tail starts after the install, before the launch (the plan said after the launch: lines from the app's first instant would be lost) — cost if wrong: none
Task 13: Ruling: a test fixture defect: a_relative_exe_is_resolved_against_the_working_directory now creates app/src (cargo's cwd always exists; `src/..` needs it) — cost if wrong: none
Task 13: complete (commits 30cee56..d5b164e, tests: cargo test -p symdev-cli --offline --test run_exe → test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s)
Task 14: Ruling: for a test target the runner also ends its wait when the report reads with every case finished (then app.kill), not only on the exit event — the Avkon examples write their report and keep running, so `symdev test --emulator` on them would otherwise wait forever (the old command waited for the file, 180 s) — cost if wrong: a symbian-test binary is killed a moment before it would have exited by itself
Task 14: Ruling: a failing test run exits 1 (as the plan's tests say), not libtest's 101 — cost if wrong: none for cargo (any non-zero fails)
Task 14: Ruling: ExeTarget::installed returns Result and owns the old stale_package check (build/<name>.sisx older than build/<name>.exe is refused) — the plan said stale_package's job is ExeTarget's — cost if wrong: none
Task 14: Ruling: Eka2l1Backend keeps only from_env (run, run_args*, installed, previous, stop and their tests deleted: no caller left); tests/test_cmd.rs's SYMDEV_EKA2L1 test gets a firmware so a profile must be started — cost if wrong: none
Task 14: complete (commits d5b164e..9344eeb, tests: cargo test -p symdev-cli --offline --test run_exe → test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.02s)
Task 14: Ruling: the root fmt gate caught the unformatted examples/async/tests/executor.rs after the Task 14 commit (my gate check had been piped through tail); fixed in a follow-up commit with symbian-rs/Cargo.lock's symbian-test entry; gates now read from a file, never through a pipe — cost if wrong: none
Task 15: Ruling: symdev build keeps its fail-fast link checks before cargo via a new RustBuild::check_link (rust-lld + the SDK's lld script) — two existing tests require them ("refused before cargo runs") and the spec's §4.6 keeps symdev build's provisioning — cost if wrong: none
Task 15: Ruling: tests that pinned the old cargo argv now check what the cargo flow guarantees: rust_sdk asserts build/rust-sdk points at the installed package and that cargo ran `build --release`; rust_linker puts a symdev-ld stub on PATH — cost if wrong: none
Task 15: Ruling: the README the plan names for SYMDEV_RUST_STD_SRC is symbian-rs/rust-src/README.md (the root README never named it); rewritten for the sysroot route — cost if wrong: none
Task 15: Ruling: symdev build errors when build/<name>.{exe,sisx} is missing after cargo found nothing to relink, instead of printing a path that does not exist — cost if wrong: none
Task 15: complete (commits fd622b8..b4c0c1e, tests: cargo test -p symdev-cli --offline --bin symdev cargo_build → test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 58 filtered out; finished in 0.00s)
Task 15: note: commit b4c0c1e's message names a code change that an Edit failed to apply; the change itself is the next commit — cost: one misleading commit message on the branch
Task 16: complete (commits 4a1344f..53e30d6, tests: cargo test -p symdev-cli --offline --test build → test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s)
Task 17: Ruling: install.sh makes the symdev-ld and symdev-rustc links itself (the same check-then-link rules as for symdev) instead of running `symdev setup-linker`: the spec says install.sh makes the link; setup-linker refuses a link to another version's binary (an upgrade would fail), and install.sh's tests use a stub symdev that cannot run setup-linker, so the links would go untested — cost if wrong: two ways to make the same links
Task 17: Ruling: the symdev-packages work is on a worktree ~/worktrees/symdev-packages/cargo-run (branch cargo-run), not a checkout switch of ~/projects/symdev-packages, and it is committed but NOT pushed (the push permission named symdev's cargo-run branch) — cost if wrong: the owner pushes it
Task 17: Ruling: the 0.4.0 recipe's commit is the zeros build.sh already treats as "fill in at release" — the tag does not exist yet — cost if wrong: none
Task 17: complete (commits 53e30d6..f3083f9, tests: python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/ci.yml")); print("ci.yml parses")' → ci.yml parses)
Task 18: Ruling: the run_exe flake (~1 in 40; 2 of 25 under full CPU load) was the fixture: spawn returns when exec closes the CLOEXEC pipe, and the kernel sets the new comm a moment later, so the runner read the test thread's name (observed comm "a_running_app_i") and dropped the fake. The fixture now waits for comm == eka2l1-fake; 0 of 25 under the same load. Production starts register only after the socket answers — cost if wrong: none
Task 18: Ruling: the plan expected the 19 no_std images equal; 6 are, and the 13 report-using ones differ only by Tasks 5–6's SDK changes (Report::record +8, the inlining function +52..+184), which the record explains symbol by symbol — cost if wrong: none
Task 18: Ruling: the runner also prints `[Service.Notifier]: Trying to display:` text (InfoPrint) besides Emulated.Stdout — hello never calls RDebug, and the plan expects its line printed — cost if wrong: system notes of other processes can show up in cargo run's output
Task 18: Ruling: hello's screenshot shows the app list, not the note: EKA2L1 only logs an InfoPrint (exp 114 §1.5 showed a black screen likewise); ui's screenshots prove drawing and keys — cost if wrong: none
Task 18: Ruling: `later` (not run) is the 4th test of broken.rs's module, not a separate tests/later.rs: another file is another program, which a panic does not reach; cargo test ran with --no-fail-fast so executor also ran — cost if wrong: none
Task 18: finding (open): two instances of one profile share its data folder (spec's registry allows it)
Task 18: complete (commits f3083f9..af0607d, tests: bash -c 'grep -q "test result: FAILED. 1 passed; 2 failed; 1 not run" ~/src/cargo-run-scratch/t18/runs.log && grep -q "ui status after Ctrl+C: 130" ~/src/cargo-run-scratch/t18/runs.log && echo "exp 114 runs as recorded"' → exp 114 runs as recorded)
Final: fixed Critical 1 (cargo test in an Avkon project asked for the app's resources) — sisx::tests::a_test_of_an_avkon_project_is_packaged_as_a_console_program RED (no console()) → GREEN
Final: fixed Important 1 (symdev package of a Rust [ui]/locales project missed build/<app>.rsc …) — ld::tests::a_release_link_keeps_the_image_and_its_resources_in_build RED (no keep_in_build) → GREEN
Final: fixed Important 2 (no timeout on the control client; a silent EKA2L1 was dropped from the registry and orphaned; Ctrl+C held during a call) — control::tests::an_emulator_that_never_answers_is_an_error_after_the_timeout RED→GREEN (calls time out: 120 s default, 3 s liveness probe), device::tests::choice::an_eka2l1_that_does_not_answer_stays_registered_but_is_not_live RED→GREEN (registered(); devices lists "(not answering)"; emulator stop uses it), run_exe::a_second_ctrl_c_ends_a_runner_waiting_on_the_emulator RED→GREEN (second Ctrl+C exits 130)
Final: re-graded Minor 1 (panic after the last case exits 0) to Important — a crashing test program reported as a pass is a false pass; fixed — run_exe::a_panic_after_the_last_case_fails_the_run RED (status 0) → GREEN (status 1, "error: the test program ended with panicked: RUST -2 after its last test")
Final: re-graded Minor 2 (OldShape incomplete for rust-std) to Important — the spec promises "the exact edits" and following them breaks a rust-std build; fixed — old_shape::tests::a_0_3_0_rust_std_project_is_also_told_to_name_symdev_rustc RED (no std arg) → GREEN; detect takes `std: bool`
Final: re-graded Minor 4 (predictable /tmp probe folder) to Important — a local user can plant a HOME/config for the EKA2L1 symdev runs; fixed — device::tests::profile::the_help_probe_keeps_its_scratch_folder_private RED → GREEN ($XDG_RUNTIME_DIR/symdev/, exclusive create_dir in /tmp)
Final: minor (deferred): a failed profile creation leaves a broken emulators/<fw>/ that every run uses (stage + rename) — emulator_profile.rs:76-105
Final: minor (deferred): garbled message in ExeTarget::installed's stale branch (missing line continuation) — run/exe_target.rs:83; its branch has no test
Final: minor (deferred): scaffolded .cargo/config.toml still says "Kept in step with symdev build (RustBuild::cargo_args)" — scaffold_rust.rs:154
Final: minor (deferred): the registered PID relies on setsid exec'ing in place (process_group(0) would avoid it) — emulator_instance.rs
Final: minor (deferred): a drive-C symlink written as an absolute path inside C is copied as is and points at the user's file — emulator_profile.rs:131-136
Final: minor (deferred): .sisx freshness by mtime fails on a filesystem where cargo copies instead of hard-linking — exe_target.rs:52-58
Final: Ruling: Minor 7 (symbian-rs and std-example lockfiles still pin symdev-locale 0.3.0) is part of Task 19's version bump, not a review fix: committed with it — cost if wrong: none

## Lead verification (2026-10-03 ~21:15)

- main merged into cargo-run (fab5721): 944 tests, clippy 0, fmt clean, no .rs over 300.
- Draft PR https://github.com/4akloon/symdev/pull/17: CI green (check, examples, examples-key).
- Lead reran accept.sh (empty HOME, file:// staging, no SYMDEV_SIGN_PASSWORD): install rc 0,
  `cargo run` printed "Hello from Rust SDK (19 chars)" after 18.9 s, rc 0; `cargo test`
  "test arithmetic ... ok", rc 0; emulator stopped. accept-ui.sh: Bars screen
  (accept/out/accept-ui.png); SIGINT to the runner's group → runner exited in 1 s with 130,
  emulator stayed until `symdev emulator stop`. accept-ui.sh's own `kill -INT -- -$job` targets
  the `cd … && setsid … &` subshell (no such group) — a script bug, not symdev's.
- Waiting for the owner: C++ `symdev run`/`test` now need the --control EKA2L1 (spec said C++
  unchanged); how users get that EKA2L1 for 0.4.0 (today only the scratch wrapper
  ~/src/cargo-run-scratch/bin/eka2l1-symdev); merge + release 0.4.0 (packages branch
  ~/worktrees/symdev-packages/cargo-run 3fe6a76 unpushed, recipe commit placeholder).
