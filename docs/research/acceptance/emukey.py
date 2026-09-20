#!/usr/bin/env python3
"""Drive a running EKA2L1 instance: send keys to the guest and grab its window.

Why not XTest: this host is GNOME on Wayland, EKA2L1 is an X11 client under Xwayland,
and XTEST input is routed by the compositor to the Wayland-focused surface, so it never
reaches the emulator. A synthetic KeyPress/KeyRelease delivered with XSendEvent straight
to the emulator's toplevel X window bypasses the compositor and Qt dispatches it like a
real key.

    emukey.py keys   <pid> F1 Down Down F2
    emukey.py shot   <pid> out.png
    emukey.py focus  <pid>
"""
import sys
import time

import x11util as X


def resolve(pid):
    d = X.open_display()
    top = X.toplevel_of_pid(pid)
    return d, top


def cmd_keys(pid, names):
    d, top = resolve(pid)
    # Activating makes the toplevel the active Qt window, which is what finally gives the
    # display widget keyboard focus (it has focusPolicy == Qt::NoFocus, so the setFocus()
    # in switch_to_game_display_mode() only takes effect once the window is activated).
    X.activate(d, top)
    for name in names:
        X.send_key_event(d, top, name, press_ms=80, gap_ms=700)
        print("sent", name)


def cmd_shot(pid, path):
    d, top = resolve(pid)
    print("saved %s %dx%d" % ((path,) + X.grab_png(d, top, path)))


def cmd_focus(pid):
    d, top = resolve(pid)
    print("toplevel", hex(top), "X input focus", hex(X.input_focus(d)))


def main(argv):
    if len(argv) < 3:
        raise SystemExit(__doc__)
    cmd, pid = argv[1], int(argv[2])
    if cmd == "keys":
        cmd_keys(pid, argv[3:])
    elif cmd == "shot":
        cmd_shot(pid, argv[3])
    elif cmd == "focus":
        cmd_focus(pid)
    else:
        raise SystemExit(__doc__)


if __name__ == "__main__":
    main(sys.argv)
