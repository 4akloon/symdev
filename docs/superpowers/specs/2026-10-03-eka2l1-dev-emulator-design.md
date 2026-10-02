# A developer-first EKA2L1: what symdev needs, and the upstream pull requests (2026-10-03)

Status: **direction approved by the owner on 2026-10-03** ("collect everything we need and
make pull requests to the original repository"). Source of the facts below: the spike note
[eka2l1-dev-emulator.md](../../research/wip/eka2l1-dev-emulator.md) (two code surveys of
`~/src/EKA2L1`, 2026-10-02). Role model: the Android Emulator (`emulator` + `adb` + console,
many AVDs, headless) and the iOS Simulator (`xcrun simctl`).

## 1. Shape

- **One emulator process per instance** (Android's model). N instances inside one process
  would need the CWD-as-data-folder and a dozen process-wide globals removed; one per process
  needs only a selectable data folder and per-instance ports.
- **A control server inside the emulator** speaking **JSON-RPC 2.0** over a local socket
  (Unix domain socket on Linux/macOS, named pipe on Windows; TCP on 127.0.0.1 with a token
  as the portable fallback). It is built on what EKA2L1 already vendors — libuv/uvw for I/O,
  rapidjson for JSON — so upstream gains no dependency. gRPC (Android's choice) would add
  protobuf and gRPC to an already large build; rejected.
- **The manager and the MCP server live in symdev** (MIT), across the process boundary:
  `symdev device create/boot/list/erase/snapshot`, commands mapped onto the protocol, MCP
  tools over the same calls (toolchain manager phase 2). Never linked to EKA2L1 (GPL-3.0).

## 2. Everything symdev needs, in order of value

| # | Need | Today in EKA2L1 | Work |
|---|---|---|---|
| N1 | A selectable data folder (`--data-dir`), nothing resolved against the CWD | Qt forces CWD = `~/.local/share/EKA2L1/`; config, compat, log, patch, shaders, cache, `devices.yml` all CWD-relative; scripting `chdir`s while threads run | medium |
| N2 | Per-instance ports (gdbstub), no shared mutable files across instances | gdbstub port only in config; QSettings shared | small |
| N3 | Control server: list apps, launch by UID, kill, install/uninstall SIS at runtime, key/touch input, screenshot (PNG), pause/resume/exit, app-exit events | primitives exist (`applist_server::launch_app`, `queue_input_from_driver`, `read_bitmap` of the screen texture, `install_package`, the iOS bridge's facade); no server | medium |
| N4 | Guest log stream (RDebug/`Emulated.Stdout`) to the controller and a CLI flag | logged to `EKA2L1.log`, filtered off by default | small |
| N5 | CLI fixes: `--listapp` prints nothing; flags act only after boot | bug | small |
| N6 | Controllable virtual sensors on desktop, settable battery/signal/status values, ETel notifications that complete | null sensor backend on desktop; fixed values; parked notifications | small–medium |
| N7 | Headless rendering (no window, no X display) | needs OpenGL + a native window; GLX pbuffer still needs X | large |
| N8 | Network conditions (latency/loss/offline), incoming SMS injection, GPS/location, serial (PTY/TCP), USB mass storage | absent or stubs | medium–large each |
| N9 | Cold snapshots (copy a stopped instance's data folder) | — (the data folder is the state) | small once N1 exists; real save-states not planned |
| N10 | gdbstub: implement or stop advertising `qXfer:libraries:read` | advertised, not implemented | small |

Plus our four open fixes: #724 (command list growth), #726 (`--install` with `--run`), #727
(property cancel during wipeout), #728 (registrations without a localisable resource file).

## 3. Upstream strategy

- Upstream is very active (maintainer `yeatse`, ~6 commits/day since August, mostly his own
  `codex/…` PRs) and our September PRs have had no review yet. So every PR is **small, one
  concern, based on the current `origin/master`, with its own verification**, and nothing in
  it is symdev-specific: each change must make sense to a user of EKA2L1 who never heard of
  symdev.
- Our fork `4akloon/EKA2L1` keeps one branch per PR (`dev/<topic>`) plus an integration
  branch **`symdev`** = `origin/master` + every open PR of ours, rebuilt and re-tested on each
  upstream move. symdev's future `emulator;<ver>` package is built from `symdev`, so symdev
  never waits for an upstream review.
- A PR is opened only after: a clean build of that branch alone on `origin/master`; the
  upstream unit tests (`ekatests`); the symdev examples (console, Avkon, DLL) installed and
  launched with `--install … --run …` and checked by PID-bound screenshot; a review by a
  separate agent. The PR text says what was broken or missing, how it was verified, and what
  it does not change.

## 4. The PR series

| PR | Branch | Covers | Depends on |
|---|---|---|---|
| P1 | `dev/data-dir` | N1 (+ the scripting `chdir` race), N2 for gdbstub's port | — |
| P2 | `dev/listapp` | N5 `--listapp` | — |
| P3 | `dev/gdb-libraries` | N10 | — |
| P4 | `dev/control-server` | N3 core: the server, `apps.list`, `app.launch`, `app.kill`, `package.install`, `package.remove`, `emulator.pause/resume/exit`, `screen.capture`, `input.key`, `input.touch`, `events.subscribe` (app exit) | — (P1 for a per-instance endpoint default, not required) |
| P5 | `dev/guest-log` | N4 over the control server and a CLI flag | P4 |
| P6 | `dev/virtual-sensors` | N6 | P4 |
| P7 | `dev/headless` | N7 | P4 |
| P8+ | … | N8, N9 | P1, P4 |

Wave 1 runs P1–P4 in parallel, each in its own copy of the EKA2L1 tree and its own build
directory. symdev's side (the manager, `symdev device`, MCP) starts once P1 and P4 are on the
`symdev` branch.

## 5. Licence boundary

EKA2L1 changes are GPL-3.0 contributions to EKA2L1, made in `~/src/EKA2L1*`, never copied
into symdev. symdev talks to the emulator only through its command line and the control
protocol; the protocol is documented in EKA2L1's tree by the PR that adds it, and symdev's
client is written from that documentation.
