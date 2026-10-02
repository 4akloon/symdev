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
## Pending

- Peripherals/control survey (agent): HLE services list, LDD/drivers, input/screenshot
  paths, scripting/gdbstub, capability table vs Android Emulator / iOS Simulator.
