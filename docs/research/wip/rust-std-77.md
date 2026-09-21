# WIP: step 77 — a real `std` for `target_os = "symbian"`

Task: make `std` build for `arm-symbian-e32` so an app can drop `#![no_std]` and `use std::fs::File`.

## Findings

## Decisions

## Dead ends

## Next step
- Read spec §6a/§3/§4/§9/§11, experiments 80/85/86/87/88, and all of `symbian-rs/crates/`.

### Session 1 orientation (2026-09-21)
- Spec §11 step 77 is the last row; every gate (68 alloc, 72 atomics, 76 TLS+time, 71 fs, 74 net) is done.
- `RustBuild::cargo_args` (`crates/symdev-build/src/driver/rust_build.rs`) is the single cargo invocation: `-Zbuild-std=core,alloc -Zjson-target-spec --target-dir build/cargo`.
- Link ordering to preserve: `-L<lib> -l:euser.dso -l:drtaeabi.dso` inserted immediately *before* the Rust archive (`rust_link.rs`), `-u _Z7E32Mainv`, `--gc-sections`.
- Target JSON `symbian-rs/targets/arm-symbian-e32.json`: `metadata.std=false`, `has-thread-local=false`, `max-atomic-width=32`, `panic-strategy=abort`, `executables=false`.
- `symbian-runtime` owns `#[global_allocator]`, `#[panic_handler]`, `#[alloc_error_handler]`, `entry!`; these all collide with a real `std` and must become conditional.

### How to build a patched `std` — settled
- **Works:** `__CARGO_TESTS_ONLY_SRC_ROOT=<tree>/library` (note: the **`library/`** subdir, not the `src/rust` root — cargo reads `<root>/Cargo.toml`). Proved with a `compile_error!` at the top of the copied `core/src/lib.rs`: the build failed inside the copy. `cp -a` of the 82 MB tree takes 0.4 s.
- **Dead end:** `rustup toolchain link` to a directory of symlinks. `bin/rustc` as a symlink makes rustc resolve `/proc/self/exe` to the *real* toolchain, so `rustc --print sysroot` returns the original and the patched src is never seen. It would need `bin` hard-linked or copied on the same filesystem — more machinery for the same result.
