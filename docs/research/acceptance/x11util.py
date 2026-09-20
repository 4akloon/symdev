"""Minimal X11 helpers over ctypes: find a window by PID, screenshot it, send keys."""
import ctypes
import ctypes.util
import re
import subprocess
import time

x11 = ctypes.CDLL(ctypes.util.find_library("X11") or "libX11.so.6")
xtst = ctypes.CDLL(ctypes.util.find_library("Xtst") or "libXtst.so.6")

x11.XOpenDisplay.restype = ctypes.c_void_p
x11.XOpenDisplay.argtypes = [ctypes.c_char_p]
x11.XDefaultRootWindow.restype = ctypes.c_ulong
x11.XDefaultRootWindow.argtypes = [ctypes.c_void_p]
x11.XInternAtom.restype = ctypes.c_ulong
x11.XInternAtom.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
x11.XStringToKeysym.restype = ctypes.c_ulong
x11.XStringToKeysym.argtypes = [ctypes.c_char_p]
x11.XKeysymToKeycode.restype = ctypes.c_ubyte
x11.XKeysymToKeycode.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
x11.XFlush.argtypes = [ctypes.c_void_p]
x11.XSync.argtypes = [ctypes.c_void_p, ctypes.c_int]
x11.XRaiseWindow.argtypes = [ctypes.c_void_p, ctypes.c_ulong]
x11.XSetInputFocus.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong]
x11.XGetInputFocus.argtypes = [ctypes.c_void_p, ctypes.POINTER(ctypes.c_ulong), ctypes.POINTER(ctypes.c_int)]
xtst.XTestFakeKeyEvent.argtypes = [ctypes.c_void_p, ctypes.c_uint, ctypes.c_int, ctypes.c_ulong]
xtst.XTestQueryExtension.argtypes = [ctypes.c_void_p] + [ctypes.POINTER(ctypes.c_int)] * 4


class XImage(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_int), ("height", ctypes.c_int), ("xoffset", ctypes.c_int),
        ("format", ctypes.c_int), ("data", ctypes.c_void_p), ("byte_order", ctypes.c_int),
        ("bitmap_unit", ctypes.c_int), ("bitmap_bit_order", ctypes.c_int), ("bitmap_pad", ctypes.c_int),
        ("depth", ctypes.c_int), ("bytes_per_line", ctypes.c_int), ("bits_per_pixel", ctypes.c_int),
        ("red_mask", ctypes.c_ulong), ("green_mask", ctypes.c_ulong), ("blue_mask", ctypes.c_ulong),
    ]


x11.XGetImage.restype = ctypes.POINTER(XImage)
x11.XGetImage.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_int,
                          ctypes.c_uint, ctypes.c_uint, ctypes.c_ulong, ctypes.c_int]


def open_display():
    d = x11.XOpenDisplay(None)
    if not d:
        raise SystemExit("cannot open DISPLAY")
    return d


def windows_of_pid(pid):
    """[(id_int, geometry_text)] for every X window whose _NET_WM_PID is pid, biggest first."""
    pid = str(pid)
    tree = subprocess.check_output(["xwininfo", "-root", "-tree"]).decode()
    out = []
    for line in tree.splitlines():
        line = line.strip()
        if not line.startswith("0x"):
            continue
        wid = line.split()[0]
        try:
            p = subprocess.check_output(["xprop", "-id", wid, "_NET_WM_PID"],
                                        stderr=subprocess.DEVNULL).decode()
        except Exception:
            continue
        if "=" in p and p.strip().split("=")[-1].strip() == pid:
            out.append((int(wid, 16), line))

    def area(t):
        m = re.search(r"(\d+)x(\d+)\+", t)
        return int(m.group(1)) * int(m.group(2)) if m else 0

    out.sort(key=lambda c: -area(c[1]))
    return out


def toplevel_of_pid(pid):
    w = windows_of_pid(pid)
    if not w:
        raise SystemExit("no X window for pid %s" % pid)
    return w[0][0]


def activate(d, win, settle=0.6):
    """Ask the WM to activate the window, then also set X input focus directly."""
    root = x11.XDefaultRootWindow(d)
    atom = x11.XInternAtom(d, b"_NET_ACTIVE_WINDOW", 0)

    class C(ctypes.Structure):
        _fields_ = [("type", ctypes.c_int), ("serial", ctypes.c_ulong), ("send_event", ctypes.c_int),
                    ("display", ctypes.c_void_p), ("window", ctypes.c_ulong),
                    ("message_type", ctypes.c_ulong), ("format", ctypes.c_int),
                    ("l", ctypes.c_long * 5)]

    class E(ctypes.Union):
        _fields_ = [("type", ctypes.c_int), ("xclient", C), ("pad", ctypes.c_long * 24)]

    ev = E()
    ev.xclient.type = 33  # ClientMessage
    ev.xclient.send_event = 1
    ev.xclient.display = d
    ev.xclient.window = win
    ev.xclient.message_type = atom
    ev.xclient.format = 32
    ev.xclient.l[0] = 2  # source: pager
    x11.XSendEvent(ctypes.c_void_p(d), ctypes.c_ulong(root), 0,
                   ctypes.c_long((1 << 20) | (1 << 19)), ctypes.byref(ev))
    x11.XRaiseWindow(d, win)
    x11.XSetInputFocus(d, win, 1, 0)
    x11.XFlush(d)
    time.sleep(settle)


def input_focus(d):
    w = ctypes.c_ulong()
    r = ctypes.c_int()
    x11.XGetInputFocus(d, ctypes.byref(w), ctypes.byref(r))
    return w.value


def send_key(d, keysym_name, press_ms=120, gap_ms=400):
    ks = x11.XStringToKeysym(keysym_name.encode())
    if ks == 0:
        raise SystemExit("unknown keysym %s" % keysym_name)
    kc = x11.XKeysymToKeycode(d, ks)
    if kc == 0:
        raise SystemExit("keysym %s is not on the current keymap" % keysym_name)
    xtst.XTestFakeKeyEvent(d, kc, 1, 0)
    x11.XFlush(d)
    time.sleep(press_ms / 1000.0)
    xtst.XTestFakeKeyEvent(d, kc, 0, 0)
    x11.XFlush(d)
    time.sleep(gap_ms / 1000.0)
    return kc


def grab_png(d, win, path):
    from PIL import Image
    info = subprocess.check_output(["xwininfo", "-id", hex(win)]).decode()
    w = int(re.search(r"Width:\s+(\d+)", info).group(1))
    h = int(re.search(r"Height:\s+(\d+)", info).group(1))
    img = x11.XGetImage(d, win, 0, 0, w, h, 0xFFFFFFFF, 2)  # ZPixmap
    if not img:
        raise SystemExit("XGetImage failed")
    im = img.contents
    raw = ctypes.string_at(im.data, im.bytes_per_line * im.height)
    pic = Image.frombytes("RGB", (im.width, im.height), raw, "raw", "BGRX", im.bytes_per_line)
    pic.save(path)
    return w, h


class XKeyEvent(ctypes.Structure):
    _fields_ = [
        ("type", ctypes.c_int), ("serial", ctypes.c_ulong), ("send_event", ctypes.c_int),
        ("display", ctypes.c_void_p), ("window", ctypes.c_ulong), ("root", ctypes.c_ulong),
        ("subwindow", ctypes.c_ulong), ("time", ctypes.c_ulong),
        ("x", ctypes.c_int), ("y", ctypes.c_int), ("x_root", ctypes.c_int), ("y_root", ctypes.c_int),
        ("state", ctypes.c_uint), ("keycode", ctypes.c_uint), ("same_screen", ctypes.c_int),
        ("pad", ctypes.c_long * 8),
    ]


KeyPressMask = 1 << 0
KeyReleaseMask = 1 << 1


def send_key_event(d, win, keysym_name, state=0, press_ms=80, gap_ms=250):
    """Deliver a synthetic KeyPress/KeyRelease straight to `win` with XSendEvent."""
    root = x11.XDefaultRootWindow(d)
    ks = x11.XStringToKeysym(keysym_name.encode())
    kc = x11.XKeysymToKeycode(d, ks)
    now = int(time.time() * 1000) & 0xFFFFFFFF
    for typ, mask in ((2, KeyPressMask), (3, KeyReleaseMask)):
        ev = XKeyEvent()
        ev.type = typ
        ev.send_event = 1
        ev.display = d
        ev.window = win
        ev.root = root
        ev.subwindow = 0
        ev.time = now
        ev.x = ev.y = 1
        ev.x_root = ev.y_root = 1
        ev.state = state
        ev.keycode = kc
        ev.same_screen = 1
        x11.XSendEvent(ctypes.c_void_p(d), ctypes.c_ulong(win), 0, ctypes.c_long(mask),
                       ctypes.byref(ev))
        x11.XFlush(d)
        time.sleep(press_ms / 1000.0)
        now += press_ms
    time.sleep(gap_ms / 1000.0)
    return kc
