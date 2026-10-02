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

## Pending

- Architecture survey (agent): core vs frontend, global state, multi-instance, headless,
  snapshots, platforms, CLI.
- Peripherals/control survey (agent): HLE services list, LDD/drivers, input/screenshot
  paths, scripting/gdbstub, capability table vs Android Emulator / iOS Simulator.
