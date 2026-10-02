# WIP: spike — a developer-first emulator from EKA2L1 (2026-10-02)

Question (owner): can we take EKA2L1's core and make our own cross-platform emulator whose
first priority is development, not everyday use — many instances, USB and other peripheral
emulation, a broad CLI/MCP for control? Role model: Android Emulator and iOS Simulator.

Path: spike (answer = recommendation; anything built is throwaway). The toolchain manager
work continues in parallel (agents C, D, E running; see `toolchain-manager.md` on branch
`toolchain-manager`).

## Facts so far

- EKA2L1: GPL-3.0, C++, CMake. `src/emu` = 1 139 files, ≈254k lines. 46 vendored externals
  (`src/external`: dynarmic, capstone, sdl2, glfw, ffmpeg, luajit, libuv/uvw, mbedtls,
  sqlite3, …). Local clone `~/src/EKA2L1` is shallow (origin/master at 2026-09-20).
- Upstream activity (GitHub API, 2026-10-02): commits 2022: 562, 2023: 156, 2024: 52,
  2025: 0, 2026: 412. Since 2026-08 almost all by one new maintainer, `yeatse`
  (Aug 189, Sep 178, Oct 1–2: 26), branches named `codex/…`, focus: game compatibility,
  EKA1 / Series 90 (Nokia 7710), window/fbs rendering. Merged our #725 on 2026-09-20; our
  #724, #726, #727, #728 (opened 2026-09-19) still open. 2 027 stars, 185 forks, 111 open
  issues.
  → "upstream moves slowly" is out of date: it moves fast, in a different direction
  (compatibility for end users, not developer tooling). A hard fork would diverge from a
  very active upstream within weeks.
- Licence: a fork stays GPL-3.0 (separate repo, separate process). symdev (MIT) may talk to
  it only across a process boundary (CLI / socket protocol), never link it. A from-scratch
  emulator written while reading EKA2L1 would need the clean-room two-role split.

## Architecture (survey of `~/src/EKA2L1`, branch `symdev-fixes`, 2026-10-02)

- Size by area (lines): services 80k (HLE servers), cpu 31k (dynarmic JIT x86_64/arm64,
  an ARM-on-ARM recompiler, an interpreter; unicorn removed), drivers 28.5k (OpenGL only —
  a Vulkan stub is not wired; audio cubeb/TSF/miniBAE/ffmpeg; SDL2 input; camera), common 23k,
  kernel 22k (HLE kernel, scheduler, IPC, timers), dispatch 20k (host replacements of
  patched guest DLLs: EGL/GLES/OpenVG/audio/video/camera), android 14k, qt 12k, loader 10k
  (E32/ROM/ROFS/FPSX/SIS/RSC/MBM/MIF/SVGB parsers), ios 10k, system 5k, mem 5k, scripting
  3k (LuaJIT), vfs 2k, gdbstub 1.7k, config 1k, ldd 1k, bridge (121k lines of `.def`).
- Core = `eka2l1::system` (`system/include/system/epoc.h`): owns CPU, memory, kernel, device
  manager, timer, VFS, gdbstub, dispatcher, packages; drivers are borrowed pointers. API:
  startup, set_device/reset (loads `roms/<firm>/SYM.ROM`), mount, pause/unpause,
  install_package, load, loop (one CPU slice). Launch-by-UID goes through the applist server.
  A test (`src/tests/epoc/system/gamecard.cpp`) builds a `system` with no frontend.
- The boot sequence is duplicated in three frontends (qt `state.cpp`, android `state.cpp`,
  ios `IosEmulator.mm`). Core→frontend leaks: UI dialog hooks (`drivers/ui/input_dialog.h`
  defined only by frontends), `launch_browser`, Qt Network linked into core drivers on Linux
  (TLS trust). Frontend→core: raw window-server pointer for input injection, redraw
  callbacks, direct VFS/loader calls.
- Multi-instance: **the process CWD is the data folder** (Qt sets it to
  `~/.local/share/EKA2L1/`, no override flag); config, compat, panic lists, log (rotated each
  start), patch, shaders, sound banks, cache all resolve against it; scripting changes the CWD
  while other threads run (a race even with one instance). Process-wide globals: logger,
  scripting instance, libuv default loop, miniBAE mixer, camera collection, FreeType library,
  SDL scoper, runtime resource root, gdbstub static buffers, CPU stats, UI singletons. No file
  locks; C:/D:/E: are shared by all devices; QSettings shared. Ports: gdbstub 24689 (only
  when enabled; blocks in accept), BT netplay off by default; guest sockets map to host
  sockets. → **N instances in one process: large. One instance per process: medium**
  (`--data-dir`, no CWD change, separate ports) — the Android Emulator's model.
- Timing is real time (`system_clock`), not deterministic; guest threads multiplexed on one
  host thread. Graphics needs OpenGL 3.x (GLES on mobile), no software renderer; headless =
  GLX pbuffer, still needs an X display (Xvfb); no EGL-surfaceless/OSMesa path. Screenshots
  offscreen are feasible (screen texture read-back exists; Android uses it).
- Save state: none (`do_state` empty); blocked by raw-pointer object graphs, host-mmapped
  chunks, JIT caches, GPU state, wall-clock timers, open host files/sockets. Real snapshots:
  large to very large; cold snapshot (copy the data folder): small.
- Frontends: Qt desktop; Android JNI (~40 calls: launchApp, installApp, installDevice,
  pressKey, touchScreen, saveScreenshotTo, runTest); iOS bridge (~50 methods) — the closest
  thing to a clean control API. CI builds Linux, Windows, macOS (Qt6), Android, iOS.
- Device model: `devices.yml` under `data/`; firmware via ROM/RPKG/VPL install; Z: per
  firmware, C:/D:/E: shared across devices.
- `eka2l1_qt` CLI (parsed after boot): help, listapp (bug: prints nothing), listdevices,
  app/run (name, UID or path), device, install (always E:), remove, fullscreen, mount,
  keybindprofile, mmcid, runng. No data-dir, headless or port flags. Control surfaces today:
  gdbstub (TCP), Lua scripts (in-process), BT netplay (peer-to-peer).
- Agent's effort read: control API medium (primitives exist: pause/reset/install/launch,
  input queue, screen read-back, device install; missing: thread-safe command queue on the OS
  thread, app-exit and log event streams, an RPC server).
## Peripherals, services, control (survey, 2026-10-02)

- HLE servers (`services/src/init.cpp`). Substantial: window (~300 ops), fbs, file, esock
  (154), central repository, applist, SIS registry, MMF audio, messaging (SQLite, many TODOs),
  UI servers. Partial: ETel (phone/line only, fixed battery 10 / signal 50, notifications
  parked and never completed, no calls), Bluetooth (L2CAP/RFCOMM/SDP over IP netplay),
  connection management (fake "Host network" IAP), HWRM (light, vibration, fixed battery),
  sensor server (8 ops), DRM, TZ, accessory, RemCon, notifier, feature manager.
  Stub: serial C32 (every op unimplemented), alarm, EKA1 camera, SMS send (moves to Sent).
  Absent: **USB (nothing at all)**, location/LBS/NMEA, EKA2 camera server (a replacement
  guest DLL calls host code instead), guest SWI (install runs on the host), MMS, IrDA.
- Host drivers: OpenGL, audio backends, camera (Qt/Android/iOS/null), vibration, SDL2 game
  controllers, sensors (Android/iOS; desktop always null), TLS, ffmpeg video. No RTC, USB,
  serial, BT hardware, GPS. LDDs: memory card, display HAL, video, stubs for comm and
  keyboard; PDD names ignored.
- Five ways a new peripheral plugs in on the guest side: an HLE server; an LDD
  factory/channel; a replacement guest DLL + host dispatch functions (how ECam, audio, video,
  TLS are done); Publish & Subscribe properties (battery, signal, USB personality); a new
  socket protocol. Host side: a driver interface + backend + a `system` setter + frontend
  wiring.
- I/O a controller needs, all present inside: key/touch injection
  (`window_server::queue_input_from_driver`, raw scan codes too); framebuffer read-back
  (Android already writes PNG screenshots); per-frame redraw callbacks (video hook); guest
  `RDebug` output goes to the log but the default filter turns it **off**; drives C/D/E are
  host directories (push/pull = file copy).
- Control surfaces today: startup-only CLI flags; LuaJIT scripting with hot reload (events,
  kernel/process/thread/memory/IPC hooks — cannot inject input, screenshot or launch);
  gdbstub (breakpoints, watchpoints, threads; advertises but lacks the library list);
  **no socket/RPC control server anywhere**. The iOS bridge is the closest facade (launch by
  UID, close app, install SIS, pointer/raw key, pause/resume, rotation, config snapshot,
  icon PNG).
- Networking: guest TCP/UDP map 1:1 onto host libuv sockets; host-overrides already redirect
  a guest hostname/port to a fake host server; instances can talk over localhost or emulated
  BT; no network-condition shaping.
- Capability matrix vs Android Emulator / iOS Simulator: no control server, video
  recording, audio capture, GPS, network shaping, snapshots, push, headless, USB, serial;
  partial install/launch/terminate/uninstall (startup-only or iOS-only), logcat (off by
  default), push/pull, screenshot (Android only), input injection (internal only), sensors
  (no desktop backend), battery/status bar (fixed values), telephony/SMS (stubs), rotation,
  clock, erase, camera (desktop: Qt camera only); present: fake backend server, debugger.
- Agent's read — most value for least work: (1) control server over the existing primitives,
  (2) guest log stream, (3) controllable desktop sensor backend, (4) settable battery/signal/
  status values + completing ETel notifications, (5) push/pull/erase/fake-server commands.
  Large: real USB, snapshots, calls/SMS done properly, location stack, headless rendering,
  video/audio recording (audio needs a mixer first). Network shaping: medium.

## Recommendation (spike answer, 2026-10-02)

Feasible — as a **thin development fork plus a host-side manager**, the Android Emulator's
shape, not as "take the core and write our own emulator":

1. **One emulator process per instance.** N instances in one process is large (CWD as data
   folder, process-wide globals); one per process is medium: a `--data-dir` that replaces the
   CWD, per-instance ports, the triplicated boot code moved into a core session class.
2. **A control server in the emulator** (local JSON-RPC: launch/kill/install/uninstall,
   key/touch, screenshot, log and app-exit event streams, pause/resume, set sensor/battery/
   signal values). The primitives exist; the server, a command queue on the OS thread and
   the event streams are new. This also retires our X11 XSendEvent/XGetImage workarounds
   (eka2l1-host skill) and works under Wayland, headless and on every OS.
3. **The manager and the MCP server live in symdev (MIT)**, across the process boundary:
   `symdev device create/boot/list/erase/snapshot` (avdmanager/simctl), commands mapped onto
   the protocol, MCP tools over the same calls. This is the toolchain manager's phase 2
   (`emulator;…`, `firmware;…` packages, `symdev device create`).
4. **Upstream strategy:** upstream is very active (one maintainer, ~6 commits a day, focus on
   compatibility). Offer the generic pieces upstream (data dir, session class, headless,
   control server); keep only developer peripherals in the fork; rebase often. An
   independent hard fork would drift from a fast upstream within weeks.
5. **Order:** MVP = items 1–3 with screenshot/input/launch/install/logs (headless on Linux via
   Xvfb as a labelled stopgap); then virtual peripherals (sensors, battery/signal, SMS inject,
   network shaping, GPS), cold snapshots (copy the stopped instance's folder), video capture;
   then real headless rendering (EGL surfaceless/pbuffer), serial (PTY/TCP), USB (start with
   mass storage re-exporting E:), calls. Real save-states: not planned (very large).

Decisions that are the owner's: thin fork + upstream PRs vs independent fork; whether to
talk to upstream's maintainer first; when to start relative to the toolchain manager.

## Next step

Present to the owner; if they go ahead, this becomes an architectural project with its own
brainstorm → spec → plan.
