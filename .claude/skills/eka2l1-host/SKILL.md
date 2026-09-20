---
name: eka2l1-host
description: Use when running, patching or rebuilding the EKA2L1 emulator for symdev — where the patched source and build live, how to launch an app, take a PID-bound screenshot, close the window, and how our fixes are exported and sent upstream.
---

# EKA2L1 on this host

EKA2L1 is GPL-3.0 and always a separate process: never copy its source into symdev.

## Where it lives (outside git)

| What | Path |
|---|---|
| Source with our fixes | `~/src/EKA2L1`, branch `symdev-fixes` (base: upstream `master`, currently `bbbf621`; pre-rebase stack on `symdev-fixes-e169852`) |
| Build directory | `~/src/EKA2L1-build` |
| Exported patches + what each one is verified by | `~/src/EKA2L1-upstream-patches/` (`README.md` first) |
| Whole-stack diff | `~/src/EKA2L1-econs-heap.patch` |
| Launch wrapper used by `symdev run` | `~/.local/bin/eka2l1` (`SYMDEV_EKA2L1`), software GL |
| Emulator data (installed apps) | `~/.local/share/EKA2L1/data/drives/e/` |
| Notes | `docs/research/eka2l1-bringup.md`, `docs/research/eka2l1.md` |

Our fork for pull requests: `4akloon/EKA2L1`. Upstream PRs #724–#729 carry the fixes worth
upstreaming (0002 and 0003 in the patch directory are local-only, they are not bug fixes).
**#725 is merged**, so `origin/master` has moved past the base `symdev-fixes` was cut from.

Branch a PR off `origin/master`, never off `symdev-fixes`: a build from a bare upstream
branch lacks our unmerged fixes, and `symdev run` then fails with "Installation of SIS
failed" because `--install` with `--run` is still only in PR #726. Build and test from
`symdev-fixes` with the new commit cherry-picked onto it.

## Rebuild and test

```bash
PATH=~/.local/eka2l1-tools/bin:~/.local/eka2l1-tools/cmake/bin:$PATH \
LIBRARY_PATH=~/.local/eka2l1-sysroot/usr/lib/x86_64-linux-gnu \
ninja -C ~/src/EKA2L1-build eka2l1_qt ekatests
```

Run `ekatests` from `~/src/EKA2L1-build/src/tests`. The build rewrites `src/emu/qt/translations/*.ts` — `git checkout` them, never commit them.
`git add -A` before a commit sweeps them in; stage the source file by name instead.

After changing a fix: commit it on `symdev-fixes` (one fix per commit), then

```bash
git -C ~/src/EKA2L1 format-patch origin/master..symdev-fixes -o ~/src/EKA2L1-upstream-patches
git -C ~/src/EKA2L1 diff origin/master symdev-fixes > ~/src/EKA2L1-econs-heap.patch
```

## Running an app and checking what the user sees

`symdev run` installs `build/<name>.sisx` and launches UID3, writing `build/eka2l1.pid` and `build/eka2l1.log`. Or directly: `~/.local/bin/eka2l1 --install <sisx> --run 0x<uid3>`; with no arguments the Qt window shows the app list (that list, not the S60 menu, is where a MIF icon is visible).

Verification loop that works here: launch, `sleep ~25`, screenshot the window that belongs to *your* PID, then close it like a user and check the process exited. Helper scripts are scratch, not in git — rebuild them under the session scratchpad when needed: a screenshot script that resolves the X11 window via `xwininfo -root -tree` plus `xprop _NET_WM_PID` (filter on `GRAB_PID`, capture with `XGetImage`), and a close script that sends `WM_DELETE_WINDOW` to the windows of `CLOSE_PID`.

## Killing

EKA2L1 ignores SIGTERM. Stop only the PIDs you started, with `kill -9`. Never `pkill`/`killall` by name, never `wineserver -k`: the user may have their own emulator or Wine programs open. When matching processes with `pgrep -f`, anchor the pattern (`pgrep -f '^winedbg'`) — an unanchored pattern also matches your own shell and kills it.

## Known behaviour

- The S60 context pane and the Menu grid render a placeholder icon for every app, including ROM ones: they cannot confirm an app icon.
- Wine `rcomp`/`mifconv` crashes leave a `winedbg --auto` process holding the Wine session alive; kill those specific PIDs.
- `~/Downloads/EKA2L1-Linux-x86_64.AppImage` was replaced by an earlier session with a stub that execs the local build; the original is `EKA2L1-Linux-x86_64.AppImage.unpatched`.
