# EKA2L1 bring-up (where to get it)

Pointers only. Official EKA2L1 URLs. No ROM download. Does not lift experiment 10/11 skips. Does not authorize claiming emulator or E52 support. See [eka2l1.md](eka2l1.md).

## Linux x86_64 AppImage

GitHub still ships CI artifacts under the **`continous`** tag (spelling as published; not “continuous”).

- Releases index: https://github.com/EKA2L1/EKA2L1/releases
- Current CI tag page: https://github.com/EKA2L1/EKA2L1/releases/tag/continous
- Linux x86_64 AppImage (observed 2026-09-18 via GitHub API `releases/tags/continous`):  
  https://github.com/EKA2L1/EKA2L1/releases/download/continous/EKA2L1-Linux-x86_64.AppImage

README: [Download Builds/Artifacts](https://github.com/EKA2L1/EKA2L1) → that same `continous` tag. Note there: no official maintainer for Linux; report breakage on GitHub issues. Do not commit the AppImage into this tree.

Install on the host (outside git), then:

```sh
chmod +x /absolute/path/EKA2L1-Linux-x86_64.AppImage
export SYMDEV_EKA2L1=/absolute/path/EKA2L1-Linux-x86_64.AppImage
```

CLI flags to install/launch a `.sisx` stay **Unknown** until experiment 10 observes them. Do not invent argv.

## Build from source (if you skip the AppImage)

Official instructions: https://github.com/EKA2L1/EKA2L1/blob/master/BUILDING.md

- Clone: `git clone --recurse-submodules https://github.com/EKA2L1/EKA2L1`
- Linux: CMake + Qt5/Qt6 + GCC 10.1+ (or Clang 10+, not CI-tested) + Python. Build `eka2l1_qt` (RelWithDebInfo recommended). Point `SYMDEV_EKA2L1` at that UI executable.

## Patched host build (this machine, 2026-09-18)

The `continous` AppImage is fine for Dictionary. Launching `E:\sys\bin\hello.exe` loads `econs`, overflows `command_list` past `MAX_CAP_COMMAND_COUNT`, and the host ABRTs. Use the patched UI instead of the AppImage for hello.

- Source: `/home/genius/src/EKA2L1` at `e169852` + `/home/genius/src/EKA2L1-econs-heap.patch` (grows `command_list` in `retrieve_next()`; stubs `CancelTextCursor` / `SetTextCursorClipped` on the window group; `scale_rectangle` derives size from scaled edges so econs' per-cell box-text fills tile at non-integer scale instead of leaving a 1px grid; Qt present path honours `integer-scaling`; regression test `scale_rectangle_keeps_adjacent_cells_watertight` in `ekatests`).
- Binary: `/home/genius/src/EKA2L1-build/bin/eka2l1_qt` (out of tree; RelWithDebInfo). Data dir is still `~/.local/share/EKA2L1` (chdir from `QStandardPaths`).
- Qt: official 6.8.3 under `/home/genius/.local/Qt/6.8.3/gcc_64`. Extra xcb runtime (including `libxcb-cursor.so.0`) extracted to `/home/genius/.local/eka2l1-sysroot`.
- User unit: `eka2l1-patched.service` (`KillMode=process`). Env: `DISPLAY=:0 QT_QPA_PLATFORM=xcb QT_OPENGL=software LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe MESA_LOADER_DRIVER_OVERRIDE=llvmpipe __GLX_VENDOR_LIBRARY_NAME=mesa QT_PLUGIN_PATH=…/gcc_64/plugins LD_LIBRARY_PATH=sysroot-lib:qt-lib`. Do **not** set `MESA_GL_VERSION_OVERRIDE`.
- GNOME `.desktop` Exec points at that binary + the same env (not the AppImage). Rollback: AppImage path above, unit `eka2l1.service`.
- After RM-469 boot: Emulation → Launch process → `E:\sys\bin\hello.exe`. Guest loads `econs` and hits the window-group text-cursor stubs; host stays up. GUI Launch process does not print `Trying to summon: hello` (that TRACE is the guest loader service); equivalent is `Loaded library: econs` plus `Set cursor text is mostly a stubbed now`.

## ROM / firmware (legal only)

Never curl, vendor, or commit ROM/firmware. An E52 RM-469 dump is **not** in the Nokia S60 SDK IA folder. User-supplied path only (`SYMDEV_ROM`).

Legal sources:

1. **Dump a phone you own.** Official GitHub wiki: [Dumping the ROM and ROFS](https://github.com/EKA2L1/EKA2L1/wiki/Dumping-the-ROM-and-ROFS) — ROMPatcher+ **Dump ROM** on-device; [Dumber](https://github.com/EKA2L1/Dumber) **Dump RPKG** for the Z: archive. Miraheze [How To Dump Device Roms](https://eka2l1.miraheze.org/wiki/How_To_Dump_Device_Roms) exists but is a stub; use the GitHub wiki.
2. **Firmware you already have**, with a **VPL**. Official quickstart: [Install a device](https://eka2l1.github.io/quickstart/basic/installdevice/) — S60v3+ can use **Firmware (VPL)** instead of ROM+RPKG. GUI: Files → Install device. Companion: [Using the emulator](https://github.com/EKA2L1/EKA2L1/wiki/Using-the-emulator).

Do not use firmware-dump / torrent / warez sites.

## 5320 vs E52

The official install-device page uses **5320** as the S60v3 screenshot example. This project’s hardware target is **Nokia E52 (RM-469)**. A 5320 (or other) image is a different device. Emulator success on 5320 ≠ E52 supported.

## `symdev run`

`symdev run` spawns `$SYMDEV_EKA2L1 --install build/<name>.sisx --run 0x<uid3>` in the background (experiment 48). On this host `SYMDEV_EKA2L1=~/.local/bin/eka2l1-patched`, a wrapper (outside git) that exports the same environment as `eka2l1-patched.service` and execs `~/src/EKA2L1-build/bin/eka2l1_qt`. The patch also fixes `--install` treating success (`installation_result_success == 0`) as failure.

