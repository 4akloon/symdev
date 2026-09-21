# Sending a key to the guest in EKA2L1

How an automated test presses a key on the emulated phone from this Linux host, why the
obvious way silently does nothing, and what was measured. Written 2026-09-20 after
[avkon-rust-spec.md](avkon-rust-spec.md) §5.2 recorded "no key of any kind could be
delivered to the emulated device from this session".

## The short version

**XTEST is dropped on this host. `XSendEvent` is not.** Deliver a synthetic
`KeyPress`/`KeyRelease` pair straight to EKA2L1's **toplevel** X window and the key runs
the whole path — Qt, the keybind profile, the window server, the application's
`OfferKeyEventL`. No emulator change was needed.

```
launch  ->  find the toplevel X window by _NET_WM_PID  ->  activate it
        ->  XSendEvent KeyPress + KeyRelease to that window  ->  XGetImage to see the effect
```

## Why XTEST does nothing

This host is **GNOME on Wayland**: `Xwayland` plus `mutter-x11-frames`, with
`_NET_SUPPORTING_WM_CHECK` naming "GNOME Shell". EKA2L1 is an ordinary X11 client inside
Xwayland.

`XTestFakeKeyEvent` is not refused and not diagnosed. Measured, in this order:

- `XTestQueryExtension` reports the extension present, version 2.2.
- `XSetInputFocus` on the emulator toplevel sticks; `XGetInputFocus` names that window
  immediately before and immediately after every key.
- Qt draws a text caret in EKA2L1's own Search box, so Qt believes the field is focused.
- Nothing arrives. Not in the guest, and not in EKA2L1's own Qt widgets: three
  `XTestFakeKeyEvent` letters left the Search box empty.

Under Xwayland a synthetic XTEST event is handed to the compositor, and mutter routes it
to whatever surface holds *Wayland* keyboard focus. An X client that merely owns the X
input focus is not that surface, so the event is delivered somewhere else entirely. Left
unattended the X focus also drifts back to mutter's own no-focus window (`0x400003` here)
within a second or two of a client setting it.

This is the whole of the earlier "not the shim" finding. The stock Exit softkey did
nothing for the same reason the shim's `InfoPrint` never fired: no key was being sent.

## Why XSendEvent works

`XSendEvent` puts the event on the wire for one specific window; the compositor is not
involved. Qt's xcb plugin dispatches on `response_type & ~0x80`, so it treats a
`send_event` key exactly like a real one. Sending `x`, `y`, `z` that way typed "xyz" into
EKA2L1's Search box with the window neither raised nor activated.

Send to the **toplevel**, not to a child. EKA2L1's display widget sets
`Qt::WA_NativeWindow`, so the window tree under the toplevel has eight or nine native
child windows; keys addressed to them went nowhere. Qt forwards a key that arrives on the
toplevel to the focus widget, which in game display mode is the display widget.

## The path a key takes, once it is in

Confirmed with temporary `LOG_INFO` probes compiled into the emulator (reverted
afterwards; `~/src/EKA2L1` is clean):

| Stage | File | Observed |
|---|---|---|
| Qt key event | `src/emu/qt/src/displaywidget.cpp` `keyPressEvent` | `key=16777264` (`Qt::Key_F1`) |
| touch-overlay first refusal | `src/emu/qt/src/mainwindow.cpp` `deliver_key_event` | returns false, key falls through |
| host->Symbian mapping | `src/emu/services/src/window/window.cpp` `handle_input_from_driver` | `mapped to scancode 164 type 3` |
| delivery to the app | `src/emu/services/src/window/io.cpp` `start_shipping` | `1 events, focus group id 2` |

`init_key_mappings` loaded **22** keybinds from `~/.local/share/EKA2L1/bindings/default.yml`
(`config.yml` has `current-keybind-profile: default`). The bindings were never the
problem; the table in avkon-rust-spec.md §5.2 is correct.

## The recipe

Needs `xwininfo`, `xprop`, `libX11`, and Python with PIL. `xdotool` also exists on this
host at `~/.local/eka2l1-tools/bin/xdotool`; `xdotool key --window <id>` is the same
`XSendEvent` mechanism, and its default (no `--window`) is XTEST, which does nothing here.

1. **Launch and wait.** `symdev run`, or `~/.local/bin/eka2l1 --run 0x<uid3>`. Wait for
   the application, not a fixed sleep: poll `build/eka2l1.log` for
   `Status pane redrawed`, then give it a few more seconds. Cold start is 30–45 s with
   software GL.
2. **Find the window.** Walk `xwininfo -root -tree`, keep the windows whose
   `xprop _NET_WM_PID` equals the emulator's pid, take the largest — that is the toplevel.
   Bind on the pid: never assume there is only one emulator running.
3. **Activate it.** `_NET_ACTIVE_WINDOW` to the root plus `XSetInputFocus`. Keys also
   arrived without this step, because mutter focuses the window when it is first mapped
   and the display widget keeps Qt focus from then on — but the emulator's display widget
   never calls `setFocusPolicy`, so its policy is `Qt::NoFocus` and
   `switch_to_game_display_mode()` runs `displayer_->setFocus()` while there is no active
   window at all (probe: `hasFocus=false focusWidget=(null) activeWindow=(null)`). The
   widget only really holds focus once the toplevel has been activated once. Activating is
   cheap insurance; do it.
4. **Send the key.** One `XSendEvent` with an `XKeyEvent` of type `KeyPress` (2), then one
   of type `KeyRelease` (3), ~80 ms apart, `send_event = 1`, `same_screen = 1`, a
   monotonically rising `time`, `keycode` from `XKeysymToKeycode(XStringToKeysym(name))`,
   `state` carrying any modifier mask. Mask `KeyPressMask` / `KeyReleaseMask`.
5. **Observe.** `XGetImage` on the toplevel (ZPixmap, BGRX) into a PNG and diff it against
   the frame taken before the key. There is no ImageMagick on this host; PIL's
   `Image.frombytes(..., "raw", "BGRX", bytes_per_line)` reads an `XImage` directly.

A working implementation lives in the session scratchpad as `x11util.py` + `emukey.py`
(`emukey.py keys <pid> F1 Down Down`, `emukey.py shot <pid> out.png`). It is scratch, like
the screenshot helper the `eka2l1-host` skill describes; rebuild it from this section.

## What it was verified against

Clean `symdev-fixes` build, no probes, 2026-09-20.

- **A symdev-built Avkon application sees the keys.** `examples/gui` copied to the
  scratchpad with an `OfferKeyEventL` that counts presses and draws
  `keys <n> code <iCode> scan <iScanCode>`, built and installed with
  `symdev build/package/run`. `Down Left Left` produced **`keys 3 code f807 scan 0e`** —
  three events, the last `EKeyLeftArrow` / `EStdKeyLeftArrow`, matching §5.2's table
  exactly. An earlier run with `Down Down Up Return` ended at
  `keys 4 code f845 scan a7` (`EKeyDevice3`, the selection key).
- **A ROM application reacts.** Notes (`0x10005907`): F1 opens the Avkon Options menu, two
  `Down` presses move the highlight from "New note" to "Help", F2 cancels. Reproduced on
  three cold launches, 3/3.

## Softkeys: answered 2026-09-21 (experiment 91)

This section used to say that softkeys "do nothing in our own applications" and that
something swallowed them. **That was too strong and it is now known to be wrong.** The
keys always arrived, Avkon always dispatched them, and the button group always turned
them into a command. Two things hid it.

**First, a guest print was being filtered out of the log.** EKA2L1's stock profile has
`Emulated.Stdout:off` in `log-filter` (`~/.local/share/EKA2L1/config.yml`), and
`RDebug::Print` lands in exactly that class (`kernel/src/svc.cpp` `debug_print`,
`LOG_TRACE(EMULATED_STDOUT, …)`). Set it to `Emulated.Stdout:trace` before instrumenting
anything in the guest, or every probe will look like it never ran.

**Second, a softkey does not reach `OfferKeyEventL`, and that is correct.** The button
group container is on the control stack at `ECoeStackPriorityCba` = 60 and the
application's view at `ECoeStackPriorityDefault` = 0 (`coeaui.h:47-61`), so the CBA is
offered the key first and consumes it. "F2 never reaches `OfferKeyEventL`" was evidence
that the CBA was *working*, not that it was missing.

With the print visible, one F2 in `symbian-rs/examples/ui` traces end to end:

```
ws type=3 code=0    scan=a5      EStdKeyDevice1 down — the view is offered this one
view OfferKeyEventL type=3       … and declines it
ws type=1 code=f843 scan=a5      EKeyDevice1 — consumed by the CBA, the view never sees it
HandleCommandL 3001              EAknSoftkeyBack
```

**3001 is `EAknSoftkeyBack`** (`avkon.hrh:330-339`: `Options`=3000, `Back`=3001, `Mark`,
`Unmark`, `Insert`, `Yes`, `No`, `Done`, `Close`, **`Exit`=3009**). The shim compares
against the symbols `EEikCmdExit` and `EAknSoftkeyExit`, so it was looking for 0x100 and
3009, and neither is 3001. Nothing was swallowed; the command simply had a name nobody
was listening for. *Why* the ROM's `R_AVKON_SOFTKEYS_EXIT` (`0x8cc002a` in the SDK's
`avkon.rsg`; the ROM's `avkon.r01` does carry offset `0x8cc0`) draws "Exit" and sends
`EAknSoftkeyBack` was **not** determined — that resource file is dictionary-compressed
and was not decoded.

**The fix is to own the resource, not to add a constant.** A GUI project's generated
`.rss` now declares its own `CBA`, with `EEikCmdExit` on the right button and
`EAknSoftkeyOptions` on the left, plus the `MENU_BAR`/`MENU_PANE` the left button opens.
Command ids then come from symdev and no ROM resource's numbering can change them.
(Experiment 95 took the menu out of `symdev.toml` altogether: the pane is compiled
empty and filled from Rust, and the id is a line's position. The softkey path below is
unchanged.)
F1 opens the menu, `Down` moves the highlight, `Return` selects, F2 ends the process —
`symbian-rs/corpus/91-ui-menu/`.

So: **acceptance tests may use the softkeys.** The earlier advice to write them against
arrows, the selection key and digits was sound while the cause was unknown; it no longer
applies.

Related, from experiment 93 on the same emulator: F1 and F2 also confirm and cancel an
Avkon query dialog, whose CBA comes from the ROM's own
`R_AVKON_DIALOG_QUERY_VALUE_TEXT`. Both halves now agree — the delivery path was never
the problem.
## One thing that is still open

- **`uiprobe`'s `OfferKeyEventL` never fired.** The Rust-shim probe from experiment 76
  took `Down`, F1 and F2 without printing its `User::InfoPrint`. Two of those three are
  now explained — F1 and F2 are consumed by the CBA above it — and the `User::InfoPrint`
  may well have worked and been invisible for the same reason a `RDebug::Print` was.
  `Down` is not explained, and that probe has not been rebuilt since; experiment 86's
  shim, which replaced it, sees arrows.

## The 22 key bindings

`init_key_mappings` loads 22 binds from `bindings/default.yml`: F1–F4, Return, the four
arrows, 0–9, `*`, `/` and Backspace. **`Escape` is bound to nothing**, so a test that
presses it proves nothing (measured 2026-09-21, experiment 93).

## Related

- [eka2l1.md](eka2l1.md) — emulator appendix, install/uninstall behaviour
- [avkon-rust-spec.md](avkon-rust-spec.md) §5.2 — what a key looks like on the guest side
- `.claude/skills/eka2l1-host/SKILL.md` — running, screenshotting and killing the emulator
- Backlog [experiment 83](experiment-backlog.md) (delivery), [91](experiment-backlog.md)
  (the softkey answer and the Options menu), [93](experiment-backlog.md) (query dialogs)

## The driver, in the repository

`docs/research/acceptance/emukey.py` (with `x11util.py` beside it):

```
emukey.py keys  <pid> Down Left Left
emukey.py shot  <pid> out.png
emukey.py focus <pid>
```

It is committed rather than left in a scratch directory, against the skill's usual rule
for helper scripts, because it took a full investigation to derive and because the UI
acceptance test of step 75 will call it. Independently reproduced 2026-09-20: three
arrows into `examples/gui` with a counting `OfferKeyEventL` drew
`keys 3 code f807 scan 0e` — `EKeyLeftArrow` / `EStdKeyLeftArrow`, the last key of
`Down Left Left`.
