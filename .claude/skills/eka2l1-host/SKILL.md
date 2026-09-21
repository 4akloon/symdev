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

Verification loop that works here: launch, `sleep ~25`, screenshot the window that belongs to *your* PID, then close it like a user and check the process exited. Throwaway probes are scratch, not in git; a driver that took real work to derive is not (the key sender lives in `docs/research/acceptance/`) — rebuild them under the session scratchpad when needed: a screenshot script that resolves the X11 window via `xwininfo -root -tree` plus `xprop _NET_WM_PID` (filter on `GRAB_PID`, capture with `XGetImage`), and a close script that sends `WM_DELETE_WINDOW` to the windows of `CLOSE_PID`.

There is no ImageMagick here. Read the `XImage` with ctypes and hand it to PIL:
`Image.frombytes("RGB", (w, h), raw, "raw", "BGRX", bytes_per_line)`.

## Pressing a key

**XTEST does nothing on this host and says nothing about it.** The session is GNOME on
Wayland; EKA2L1 runs under Xwayland, and a synthetic XTEST event is routed by the
compositor to the Wayland-focused surface, so it never reaches the emulator — not the
guest, not even EKA2L1's own Qt widgets. `XGetInputFocus` will happily name the emulator
window the whole time. `xdotool key` without `--window` is XTEST, so it is dead too.

What works: **`XSendEvent` a `KeyPress` then a `KeyRelease` straight to the emulator's
toplevel X window** (the largest window with your `_NET_WM_PID`, not one of its
`WA_NativeWindow` children). Qt dispatches a `send_event` key like a real one, and it runs
the whole path into the guest.

```
launch (symdev run, or --run 0x<uid3>)
  -> poll build/eka2l1.log for "Status pane redrawed", then a few seconds more
  -> toplevel = largest window whose xprop _NET_WM_PID is your pid
  -> _NET_ACTIVE_WINDOW to the root + XSetInputFocus on it
  -> XSendEvent KeyPress (type 2) then KeyRelease (type 3), ~80 ms apart,
     send_event=1, same_screen=1, rising time, keycode from XKeysymToKeycode
  -> XGetImage before and after, diff the PNGs
```

Verified end to end on a clean build: an Avkon application's `OfferKeyEventL` counted
`Down Left Left` and drew `keys 3 code f807 scan 0e`; ROM Notes opens its Options menu on
F1. **Softkeys work too** (experiment 91): F1 and F2 reach `HandleCommandL`, never
`OfferKeyEventL` — the button group container is above the view on the control stack and
consumes the key. An acceptance test may press them. Full write-up and the measured
evidence: `docs/research/eka2l1-input.md`. The 22 bound keys are F1–F4, Return, the
arrows, 0–9, `*`, `/` and Backspace; `Escape` is bound to nothing.

## Seeing anything the guest prints

**Check `log-filter` in `~/.local/share/EKA2L1/config.yml` before you believe a probe
did not run.** The stock profile is
`*:trace Emulated.Stdout:off Service.EFsrv:warn Service.Cenrep:off Kernel:Warn Service.Track:error`,
and it has hidden the answer twice:

| Class | Hides | Cost |
|---|---|---|
| `Emulated.Stdout:off` | every `RDebug::Print` from the guest (`svc.cpp` `debug_print`) | three sessions of "softkeys do nothing" (experiment 91) |
| `Kernel:Warn` | the panic line for `E32USER-CBase 69` and friends | three hours of the wrong hypothesis (experiment 90) |

Set the class you need to `trace`, run, and **put the file back** — the emulator reads it
at startup and rewrites it on exit, so a stale edit leaks into the user's own sessions.

## Killing

EKA2L1 ignores SIGTERM. Stop only the PIDs you started, with `kill -9`. Never `pkill`/`killall` by name, never `wineserver -k`: the user may have their own emulator or Wine programs open. When matching processes with `pgrep -f`, anchor the pattern (`pgrep -f '^winedbg'`) — and the same for `pkill -f`, which I used unanchored on 2026-09-21 and killed my own shell — an unanchored pattern also matches your own shell and kills it.

## Known behaviour

- The S60 context pane and the Menu grid render a placeholder icon for every app, including ROM ones: they cannot confirm an app icon.
- Wine `rcomp`/`mifconv` crashes leave a `winedbg --auto` process holding the Wine session alive; kill those specific PIDs.
- `~/Downloads/EKA2L1-Linux-x86_64.AppImage` was replaced by an earlier session with a stub that execs the local build; the original is `EKA2L1-Linux-x86_64.AppImage.unpatched`.
