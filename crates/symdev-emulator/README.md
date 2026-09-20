# symdev-emulator

Launches packages in the [EKA2L1](https://github.com/EKA2L1/EKA2L1) Symbian emulator. EKA2L1 is
GPL-3.0, so it is only ever started as a separate process: it is never linked, vendored or copied
into this repository.

## Usage

```rust,ignore
use symdev_emulator::Eka2l1Backend;

let emulator = Eka2l1Backend::from_env()?;           // SYMDEV_EKA2L1
let pid = emulator.run(sisx, uid3, log_path)?;       // install to E: and launch, in the background
```

`SYMDEV_EKA2L1` is the path of `eka2l1_qt` or of a wrapper script that sets its environment. The
backend passes the observed options `--install <sisx> --run 0x<uid3>`; its output goes to the log
file, and the child runs in its own process group.

`Eka2l1Backend::previous(pid_file)` returns the PID recorded by an earlier run if that process
still exists (Linux `/proc`). EKA2L1 ignores SIGTERM and each run starts a new instance, so
callers warn instead of stopping it.

## Not a device

A successful run shows the app works in the emulator. It says nothing about a real E52. The
crate does not yet implement `symdev_core::EmulatorBackend`.

## Testing

```bash
cargo test -p symdev-emulator --offline
```
