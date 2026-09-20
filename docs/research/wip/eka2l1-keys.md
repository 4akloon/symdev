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

- **Keys do reach the guest.** With temporary probes compiled into
  `display_widget::keyPressEvent`, `main_window::deliver_key_event`,
  `window_server::handle_input_from_driver` and `window_key_shipper::start_shipping`, an
  `XSendEvent` F1 shows the whole chain: Qt key 16777264 -> scancode 164 -> shipped to
  focus window group 2. Observed effect: in the ROM **Notes** app (uid `0x10005907`) F1
  opens the Avkon Options menu, two `Down` presses move the highlight from "New note" to
  "Help", F2 cancels. So the emulator, the bindings and the guest were never the problem.
- **`gui` (`examples/gui`, uid `0xe7351c20`) does not visibly react** to F2 even though the
  scancode 165 is shipped: its `R_AVKON_SOFTKEYS_EXIT` CBA did not turn the press into
  `EAknSoftkeyExit`. Separate question; it is not an input-delivery problem.
- **The one real fragility is Qt focus.** `display_widget` never calls
  `setFocusPolicy`, so its policy is `Qt::NoFocus` (probe printed `focusPolicy=0`), and
  `main_window::switch_to_game_display_mode()` calls `displayer_->setFocus()` at a moment
  when the window is not active yet (probe: `hasFocus=false focusWidget=(null)
  activeWindow=(null)`). The widget only ends up with focus later, when the toplevel is
  activated. Runs where that never happened swallowed every key.

## Decisions

## Dead ends

## Next step

Check how reliable the activate-then-XSendEvent recipe is over repeated cold launches;
if the `Qt::NoFocus` policy makes it flaky, prepare the one-line `setFocusPolicy` fix as
a seventh upstream PR. Then write `docs/research/eka2l1-input.md`, backlog 83 and the
skill section, and remove the probes from `~/src/EKA2L1`.
