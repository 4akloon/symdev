# WIP: deliver a key press to the guest in EKA2L1

Task: find out why no host key press reaches the emulated Symbian guest on this host, and
make a reproducible way for an automated test to press a key and observe the effect.

## Findings

- Starting point: `docs/research/avkon-rust-spec.md` §5.2 and §9 — XTest with the window
  activated and focused delivered nothing, not even the stock Exit softkey (F2).
- `~/.local/share/EKA2L1/bindings/default.yml` **does** have key bindings (F1→164, F2→165,
  Enter→167, arrows→14..17, digits, `*`, `#`), and `config.yml` has
  `current-keybind-profile: default`. So "a device with no bindings" is not the cause.

- **The session is GNOME on Wayland**: `Xwayland` (pid 4588) + `mutter-x11-frames`,
  `_NET_SUPPORTING_WM_CHECK` names "GNOME Shell". EKA2L1 is an X11 client under Xwayland.
- **XTEST is the reason keys never arrived.** `XTestQueryExtension` says the extension is
  present (2.2), `XSetInputFocus` on the emulator toplevel sticks (`XGetInputFocus` agrees
  before and after each key), and Qt even paints a caret in its own Search box — yet
  `XTestFakeKeyEvent` produces *nothing*, not in the guest and not in the emulator's own
  Qt widgets. Under Xwayland synthetic XTEST input is handed to the compositor, and mutter
  delivers it to whatever surface has *Wayland* keyboard focus; an X client that merely
  owns the X focus never sees it. So the previous session's conclusion ("not the shim")
  was right, and it was never EKA2L1's fault either.
- **`XSendEvent` of a KeyPress/KeyRelease straight at the emulator's toplevel X window
  works.** Typing `x`,`y`,`z` that way put "xyz" in EKA2L1's Search box with the window
  neither raised nor activated. It bypasses the compositor, and Qt's xcb plugin dispatches
  `send_event` key events like real ones.

## Decisions

## Dead ends

## Next step

Launch an app in the emulator and check whether an `XSendEvent` key reaches the *guest*
(display widget focus, `on_ui_window_key_press`, window server). Test with F2, the stock
Exit softkey (`EStdKeyDevice1`).
