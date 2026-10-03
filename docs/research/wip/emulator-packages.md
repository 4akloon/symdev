# WIP: emulator and firmware packages plan

Task: write the implementation plan for
`docs/superpowers/specs/2026-10-03-emulator-firmware-packages-design.md` as
`docs/superpowers/plans/2026-10-03-emulator-firmware-packages.md` on branch `cargo-run`
(worktree `~/worktrees/symdev/cargo-run`). Push only `cargo-run`.
Scratch: `~/src/emu-pkg-scratch/`. Read-only observations go to experiment 115 §1 in
`docs/research/experiment-backlog.md`. No emulator runs, CI triggers, fork pushes.

## Status

- [ ] read inputs (spec, cargo-run spec/plan/wip, toolchain spec §12 §15, real code)
- [ ] read symdev-packages (publish/, recipes/gcce/12.1.0, workflows, install.sh)
- [ ] read EKA2L1 copies and fork CI build.yml
- [ ] experiment 115 §1 observations
- [ ] plan written, self-reviewed, committed, pushed

## Facts
- Fork CI (`integration` copy, branch symdev d07d5ac, `.github/workflows/build.yml`): job
  `build-desktop`, matrix label `linux`, `ubuntu-latest`, Qt 6 + SDL2 from apt, cmake Release
  `-DCI=ON -DEKA2L1_ENABLE_UNEXPECTED_EXCEPTION_HANDLER=ON -DEKA2L1_NO_TERMINAL=ON
  -DEKA2L1_ENABLE_DISCORD_RICH_PRESENCE=ON`; `scripts/generate_appimage.sh` (linuxdeploy +
  plugin-qt + plugin-appimage, all `continuous`, wget'd unpinned) with APPIMAGE_EXTRACT_AND_RUN=1;
  artifact `eka2l1-<short sha>-linux` = `build/eka2l1-qt-x64.AppImage`; no `retention-days`
  (repo default). Triggers: push, pull_request, workflow_dispatch.
- Local AppImage `~/Downloads/EKA2L1-Linux-x86_64.AppImage.unpatched` (94 452 216 B, sha256
  d8f6c8fe…fb85, Sep 18; the non-.unpatched file is a 51-byte sh stub → ~/.local/bin/eka2l1).
  Extracted WITHOUT running it: `unsquashfs -o 944632` (ELF runtime size = e_shoff+shnum*shentsize)
  into `~/src/emu-pkg-scratch/appimage-upstream/`. squashfs zstd, 786 inodes.
- Layout: `AppRun -> usr/bin/eka2l1_qt` (a SYMLINK, no apprun-hooks), `.DirIcon`, desktop+icon
  links; `usr/bin/{eka2l1_qt, compat/, patch/, resources/, scripts/, tools/, panic.json,
  libscripting.a, qt.conf, …}`, `usr/lib/` 187 libs (203 MB), `usr/plugins/` 16 Qt plugins,
  `usr/translations/`, `usr/share/doc/<deb pkg>/copyright` ×167. Extracted 254 MB.
  `usr/bin/qt.conf` (linuxdeploy-plugin-qt): Prefix=../, Plugins=plugins. eka2l1_qt RUNPATH
  `$ORIGIN/../lib` ⇒ starting usr/bin/eka2l1_qt directly finds libs and plugins.
- AppRun is a symlink ⇒ exec'ing AppRun gives /proc/<pid>/comm "AppRun", which
  `device::is_eka2l1` (comm contains "eka2l1") rejects ⇒ symdev must exec usr/bin/eka2l1_qt.
- Qt 6.4.2 (Ubuntu 24.04 apt, GCC 13.2.0). glibc floor GLIBC_2.38 (many Ubuntu libs:
  libxml2, libxkbcommon, libx264…); this host glibc 2.43. Not bundled: libc, libstdc++,
  libgcc_s, libGL/GLX/EGL, libX11, libxcb, fontconfig/freetype, libz (host must provide).
- Bundle carries Ubuntu ffmpeg (libavcodec60…) via Qt's multimedia ffmpeg plugin, x264, x265,
  libzvbi etc.: copyleft beyond Qt ⇒ corresponding source for MORE than Qt (spec §3 gap).

- Fork 4akloon/EKA2L1 (public fork of EKA2L1/EKA2L1): build.yml workflow state `active`, but
  ZERO runs of it ever (only one Dependency Graph run, master c396ac8); `symdev` d07d5ac pushed
  09:02Z made no run ⇒ a push may not start CI; lead must check a run appears, else dispatch.
  Artifacts: 0. Upstream master fbf0060 (2026-10-03 13:45Z); fork master stale c396ac8.
- Our 11 open PRs → fork heads: #724 fix/command-list-overflow 7ff9a13, #726
  fix/cli-install-then-run 2338a37, #727 fix/property-cancel-during-wipeout e836a07, #728
  fix/applist-no-localisable-rsc f7b7888, #766 dev/data-dir a3ec972, #767
  dev/anim-window-lifetime 681a9ef, #768 dev/applist-reload 25de6ec, #769 dev/applist-lock
  c597988, #770 dev/control-server d1cdb4a, #771 dev/control-input 89e61e2, #772
  dev/control-events c323b64. Stacked: 770 ⊂ 771 ⊂ 772; others independent; data-dir on master.
  Known conflict (cargo-run copy): data-dir × control-events in qt/src/thread.cpp and
  qt/include/qt/cmdhandler.h option lists — keep both.
- EKA2L1 with --data-dir copies shipped patch/, resources/, scripts/ (compat/ if missing) from
  the exe dir into the data folder at start (data-dir.PR.md); Qt settings go to
  <folder>/EKA2L1/EKA2L1.ini. Config: each key defaults if missing (get_yaml_value), so a
  config.yml holding only `log-filter:` is valid by reading; `device: 0`, `data-storage: data`.
- EKA2L1 WRITES Z: at every start for epoc93fp1+: moves Z:\sys\bin\avkonfep.dll to .bak (if no
  .bak) then copies patch\avkonfep_general.dll (fails on Linux: backslash) — qt/src/state.cpp.
  Owner's Z has avkonfep.dll.bak (Sep 18 17:02) and no avkonfep.dll ⇒ a package made from it
  carries the .bak state, and EKA2L1 then only attempts the failing copy.
- Owner data (read-only): devices.yml 124 B (RM-469: platver epoc93fp2, manufacturer Nokia,
  firmcode RM-469, model N00, machine-uid 0, isolated-drives false; rewritten by EKA2L1 at
  start, mtime 18:36 = the incident); roms/rm-469/SYM.ROM 51 MB; drives/z/rm-469 208 MB,
  15 595 files, 0 symlinks, has a dir literally named `z:`; drives/c 17 MB (user state).
- symdev side today: `Devices::profiles()` makes profiles from `EmulatorData::from_env()`
  (SYMDEV_EKA2L1_DATA else ~/.local/share/EKA2L1 default!) when none exist;
  `EmulatorProfile::create(from, firmware)` links data/roms/<fw> and the WHOLE data/drives/z,
  copies devices.yml, drive C (copy_tree), config.yml (+ symdev log filter).
  `eka2l1_with_control()` = Eka2l1Backend::from_env (SYMDEV_EKA2L1 only) + has_control.
  `is_eka2l1(pid)` = /proc comm contains "eka2l1". Provision::install_missing/manager =
  how symdev build installs SDK. Pins: gcce, platform_sdk(device), rust_sdk. Installed files
  0644/0755 (tar_gz.rs), relative in-package symlinks allowed (reproducible.rs packs them).
- Packages repo: `publish public <id> --from <prefix> --source-code <tar.gz> --recipe`;
  `publish private` needs a pinned sha256 and takes `include` lists; source key
  `src/<id path>/<sha>.tar.gz`. Workflows: build.yml (gcce, PR), publish.yml (main), symdev.yml
  (recipes/symdev/**), tests.yml. Supported host floor: glibc 2.28 (gcce on AlmaLinux 8).

## Decisions

## Dead ends

## Next step

Read the inputs.
