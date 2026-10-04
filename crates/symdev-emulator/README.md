# symdev-emulator

The devices `cargo run` and `cargo test` run on: [EKA2L1](https://github.com/EKA2L1/EKA2L1)
emulators that symdev starts, and the control protocol it drives them with. EKA2L1 is
GPL-3.0, so it is only ever started as a separate process: it is never linked, vendored or
copied into this repository.

## Usage

- `device::Eka2l1`: the EKA2L1 to start, the user's own (`User`, started with the user's
  whole environment) or an installed `emulator` package's `usr/bin/eka2l1_qt` (`Package`,
  started without the host's `LD_LIBRARY_PATH`, `QT_PLUGIN_PATH` and
  `QT_QPA_PLATFORM_PLUGIN_PATH`).
- `device::Firmware`: what a profile is made from, a firmware installed in the user's EKA2L1
  data folder or an installed `firmware;<name>;<n>` package.
- `device::EmulatorProfile`: an emulator's own data folder under
  `~/.local/share/symdev/emulators/<name>/`; `create` makes it from a `Firmware` (ROM and
  drive Z linked, never written), `check` refuses one whose firmware is gone.
- `device::EmulatorInstance`: starts an `Eka2l1` on a profile with `--data-dir <profile>
  --control <socket>`, waits until it answers, registers it, and stops it (`kill -9`).
- `device::DeviceRegistry`: the emulators symdev started
  (`$XDG_RUNTIME_DIR/symdev/devices/<id>.toml`) and which of them still run.
- `device::DeviceChoice`: which device a run uses (`SYMDEV_DEVICE`, the one running emulator,
  the one profile, or a prompt).
- `control::ControlClient`: the JSON-RPC client of EKA2L1's `--control` server (install,
  launch, events, screen capture).

This crate reads no environment for the emulator or the firmware. `symdev-cli`'s `Provision`
resolves them: `SYMDEV_EKA2L1` and `SYMDEV_EKA2L1_DATA` first, the pinned `emulator` and
`firmware` packages second, installed on first need.

## Not a device

A successful run shows the app works in the emulator. It says nothing about a real E52. The
crate does not yet implement `symdev_core::EmulatorBackend`.

## Testing

```bash
cargo test -p symdev-emulator --offline
```
