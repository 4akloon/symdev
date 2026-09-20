# Experiment 65 (WIP): `symdev new --language rust` → build → package → run on EKA2L1

Task: make the hand-built 65a Rust `E32Main` a product path — `symdev new hello --language rust && symdev build && symdev package && symdev run` prints the `[Service.Notifier]: Trying to display: Hello from Rust SDK` line in `build/eka2l1.log` with no hand steps.

## Findings

## Decisions

## Dead ends

## Next step

- Step 1: create `symbian-rs/` workspace, `rust-toolchain.toml`, derive `targets/arm-symbian-e32.json`.
