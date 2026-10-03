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
NEXT: Task 11 — `task-start <plan> 11` (device registry + choice).
