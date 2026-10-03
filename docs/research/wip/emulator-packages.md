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

## Decisions

## Dead ends

## Next step

Read the inputs.
