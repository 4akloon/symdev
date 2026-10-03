# `cargo build`, `cargo run`, `cargo test` for symdev Rust projects — design

Status: approved in conversation with the owner on 2026-10-03; target release symdev 0.4.0
(with `rust-sdk;0.4.0`). Role model: the esp-rs toolchain (cargo links through a custom linker,
`runner = "espflash flash --monitor"`), with Flutter's device model (`flutter devices`,
`flutter run` picks or asks).

## 1. Goal

In a Rust project made by `symdev new --lang rust`, plain cargo does the whole loop:

- `cargo build` produces the signed, installable `.sisx` (and the E32 `.exe`);
- `cargo run` picks a device the way `flutter run` does, installs, launches, streams the app's
  log, and ends with the app;
- `cargo test` runs the project's tests on the chosen device and prints `libtest`-style results.

`symdev.toml` stays the only place for what the phone needs (UID3, capabilities, vendor, UI
resources, signing); nothing of it moves into `Cargo.toml`, and nothing else moves into it.
C++ projects are unchanged.

## 2. Decisions taken with the owner

| Question | Decision |
|---|---|
| What `cargo run` targets | A device chosen like Flutter: one → it, several → numbered prompt |
| Choosing among several | `flutter run` rules; `SYMDEV_DEVICE=<id>`; no terminal + several → error listing ids |
| What `cargo build` produces | A signed `.sisx`, not just the `.exe` |
| Emulator lifecycle | Like Flutter: the emulator outlives `cargo run`; Ctrl+C stops the app only |
| `cargo test` | In scope |
| Approach | A: symdev as cargo's linker (`symdev-ld`) plus a runner (`symdev run --exe`) |

Rejected: B, a `cargo symdev …` subcommand (plain `cargo run` would still not work); C, a
runner without a linker (`cargo run` needs a `bin`, and `cargo build` would not give a `.sisx`).

## 3. Project shape

- The crate is an ordinary binary: `[[bin]]` from `src/main.rs`, `test = false` (no `libtest`
  without `std`). Today's `[lib] crate-type = ["staticlib"]` goes.
- The target spec `symbian-rs/targets/arm-symbian-e32.json` gets `"executables": true`, so rustc
  builds binaries for it. Its `linker-flavor` stays `gnu-lld`.
- `.cargo/config.toml` written by `symdev new`:
  - `[build] target = "build/rust-sdk/symbian-rs/targets/arm-symbian-e32.json"`,
    `target-dir = "build/cargo"`;
  - `[unstable] build-std`, `build-std-features`, `json-target-spec` as today;
  - `[target.arm-symbian-e32] linker = "symdev-ld"`, `runner = "symdev run --exe"`.
- Compile-time values that `symdev build` passes through the environment today
  (`SYMDEV_UID3`) come from `symdev.toml` instead, read by a build script or macro of the Rust
  SDK through `CARGO_MANIFEST_DIR`, so a plain `cargo build` sees the same values.
- `symdev build` on a Rust project runs `cargo build --release` in the project root; there is
  one build path.

## 4. `symdev-ld`, the linker

`symdev-ld` is the `symdev` binary started through a link of that name (the name selects the
role; `install.sh` makes the link, `symdev setup-linker` makes it for a checkout build).

1. rustc calls it with ld-style arguments: object files, `.rlib` archives, search paths, `-o`.
   The exact argv is recorded in the first spike and becomes test fixtures; an argument it does
   not know is refused by name, never passed on or guessed.
2. It finds the project through `CARGO_MANIFEST_DIR` (inherited from cargo through rustc) and
   reads `symdev.toml` there; no `symdev.toml` is an error naming the directory.
3. It links exactly as `symdev build` 0.3.0 does after cargo: `LldLine`, the linker script, the
   shims (prebuilt or compiled), libcalls, two rust-lld links around `ImportStubs`, the
   `R_ARM_JUMP_SLOT` check, then elf2e32 (experiments 109, 112, 113).
4. It writes the E32 image at the `-o` path, then the resources, the icon and the signed
   `.sisx` next to it (`<out>.sisx`), and copies the main binary's `.sisx` to
   `build/<name>.sisx` as today.
5. A test binary (§7) is linked as a console program even in an Avkon project. How
   `symdev-ld` tells it from the main binary (expected: `CARGO_CRATE_NAME` and an output in
   `deps/`) is observed in the spike, not assumed.
6. Packages the build needs (SDK, rust-sdk) are installed the way `symdev build` installs them.

## 5. Devices

A device is, in phase 1, an emulator; a phone is a later device kind.

- **Running emulator:** an EKA2L1 process symdev started with `--control` (EKA2L1#770–#772)
  and its own `--data-dir` (EKA2L1#766). Id `emulator-1`, `emulator-2`, …; name from the
  firmware, e.g. "Nokia E52 (RM-469)". An EKA2L1 the user started without `--control` is not
  listed and never touched.
- **Emulator profile** (Android's AVD): `~/.local/share/symdev/emulators/<name>/`, its own
  drives C: and E: and config, the ROM referenced rather than copied. Phase 1 creates one
  profile per firmware already installed in the user's EKA2L1 (today `rm-469`); later
  firmware comes as packages from the private source.
- **Registry:** each running instance writes `$XDG_RUNTIME_DIR/symdev/devices/<id>.toml`
  (PID, profile, socket, log). An entry is live when its PID exists and its socket answers;
  dead entries are removed. symdev stops only registered instances, with `kill -9`.
- **Commands:** `symdev devices` lists running devices and available profiles;
  `symdev emulator start <profile>`, `symdev emulator stop <id>`.

**Choice** (`cargo run`, `cargo test`, `symdev run`, `symdev test`):

1. `SYMDEV_DEVICE=<id or profile>` wins; a profile with no running instance is started.
2. Exactly one running device → it.
3. None running and exactly one profile → that profile is started.
4. Otherwise a numbered prompt on the terminal (`[1]: Nokia E52 (emulator-1)`, a digit picks,
   `q` quits). Without a terminal: an error listing the ids and naming `SYMDEV_DEVICE`.

## 6. The runner (`symdev run --exe <path>`)

Cargo runs `symdev run --exe <exe> [args…]` for `cargo run` and for each test binary.

1. It takes `<exe>.sisx` next to the image and reads UID3 from the E32 header. No `.sisx` is an
   error: the binary was not linked by `symdev-ld`; check `.cargo/config.toml`.
2. It picks a device (§5). Starting an instance: EKA2L1 in its own session (`setsid`), so the
   terminal's Ctrl+C never reaches it, with `--data-dir <profile>` and `--control <socket>`;
   ready when `apps.list` answers.
3. If the UID3 is running: `app.kill`. Then `package.install` (an upgrade over the old one) and
   `app.launch`. With an instance already up this takes seconds.
4. Log: the app's `RDebug` output and the panic lines appear in the terminal. Phase 1 reads the
   instance's log file; the profile is symdev's, so its `config.yml` enables
   `Emulated.Stdout` without touching the user's EKA2L1 config. When the guest-log PR (P5 of
   the emulator design) lands, the log comes over the control server instead.
5. End, from `event.app_exited`:

   | The app | Exit code | Printed |
   |---|---|---|
   | exits normally | 0 | — |
   | panics | 101 (as Rust) | `panicked: <category> <reason>` |
   | is killed | non-zero | the exit type and reason |
   | its emulator window is closed | non-zero | "emulator-1 was closed" |
   | the user presses Ctrl+C | 130 | — (runner sends `app.kill`; the emulator stays) |

6. Arguments after `--` are an error for now: passing a command line to an app was never
   observed on the real system (CLAUDE.md: no guessed behaviour).

**Dependency:** an EKA2L1 with `--control` and `--data-dir`. Until EKA2L1#766 and #770–#772
are merged upstream it is a build of our fork's `symdev` integration branch, reached through
`SYMDEV_EKA2L1` and later as an `emulator;<version>` package. An EKA2L1 without `--control` is
an error naming what to install.

## 7. Tests (`cargo test`)

- Tests live in `tests/*.rs` with `harness = false` (as esp-rs's `embedded-test`). A module
  marked `#[symbian_test::tests]` holds `#[test] fn name() -> Result<(), Evidence>`; the macro
  (new crate `symbian-test` in the Rust SDK) generates `E32Main`, runs the tests in order and
  writes the existing report `E:\symdev\results\<uid3>.json` (schema 1, the one
  `symdev test` reads; `symbian-std::test_report`).
- Before each test the harness appends "started <name>" to the report, so a panic (which aborts
  the process) is attributed: the runner marks that test FAILED with the panic's category and
  reason, and the rest as not run.
- The runner waits for `event.app_exited`, reads the report from the profile's drive E:,
  prints `test <name> ... ok|FAILED` and `test result: ok. N passed; M failed`, and exits
  non-zero on any failure or on a report with no cases.
- Test binaries use the project's UID3: they replace the app on the emulator; the next
  `cargo run` installs it back.
- No `#[cfg(test)]` unit tests inside `src/` in phase 1 (`test = false` on the bin). Host tests
  of the SDK crates stay plain `cargo test` in `symbian-rs`.
- `symdev test --emulator` stays and does what `cargo test` does.

## 8. Migration

- symdev 0.4.0 with `rust-sdk;0.4.0` (target `executables: true`, `symbian-test`).
- `symdev new` writes the new shape; the same change moves all 21 examples in `symbian-rs`.
- A project in the old shape (`staticlib`) is not built the old way: `symdev build` stops with
  the exact edits to `Cargo.toml` and `.cargo/config.toml`, so there is one build path.
- CI's `examples` job builds with `cargo build`.

## 9. Errors

Each names what failed and the fix: `symdev-ld` without `symdev.toml`; an unknown rustc
argument (refused by name); no `.sisx` next to the `.exe`; EKA2L1 without `--control`; several
devices and no terminal; the emulator closed during a run; an old-shape project.

## 10. Verification

- **Unit:** `symdev-ld` argv parsing on the recorded rustc invocations; the device choice as a
  type, table-tested over §5's rules; registry cleanup of dead entries; exit-code mapping;
  the `libtest`-format printer.
- **Integration:** the runner against a fake control server (JSON-RPC 2.0 over a Unix socket):
  install and launch, Ctrl+C, a panic in `event.app_exited`, the window closed.
- **Real (experiment 114):** `cargo build --release` gives the same `.exe` bytes as
  `symdev build` 0.3.0 on all 21 examples, or each difference is explained; `cargo run` on
  hello and an Avkon app with PID-bound screenshots, and a second `cargo run` in seconds;
  `cargo test` on `async` with a deliberately failing test and a panicking one; the
  several-devices-no-terminal error.
- **Acceptance:** an empty HOME, `install.sh`, `symdev new --lang rust`, `cargo run` shows the
  app, `cargo test` passes.

## 11. First spike (before the plan's tasks)

The plan starts with a spike that answers, by observation:

1. rustc's exact linker argv for this target with `linker-flavor: gnu-lld`, for the main binary
   and a `harness = false` test, in `dev` and `release` profiles;
2. whether libcalls can be an ordinary dependency (today a separate cargo run with
   `-Zdefault-visibility=hidden`, experiment 111) without changing any `.exe` byte;
3. whether `language = "rust-std"` projects can point `-Zbuild-std` at the patched `std` from
   configuration alone (today symdev sets a variable cargo itself reads). If not, `rust-std`
   projects stay on `symdev build` in 0.4.0 and the spec records it;
4. what the `dev` profile produces (debug info, size) and whether elf2e32 accepts it;
5. how `symdev-ld` tells a test binary from the main one.
