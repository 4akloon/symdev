# no_std parity: CTrapCleanup

Install the per-thread trap handler and cleanup stack in the `no_std` runtime, as `std`
already does, and make the equality between the two paths verifiable rather than claimed.

## Findings

- Only the `std` overlay installs it today: `symbian-rs/rust-src/overlay/library/std/src/sys/pal/symbian/cleanup.rs`
  holds `TrapCleanup` (`CTrapCleanup_New` + `symrs_cleanup_destroy`). The `no_std`
  `entry!` in `symbian-rs/crates/symbian-runtime/src/lib.rs` calls `main` directly.
- `std` depends only on `symbian-sys` (`rust-src/overlay/library/std/Cargo.toml:32`), so a
  shared home for `TrapCleanup` must be at or below `symbian-sys`.
- `symbian-core::fs` has no `read_dir`, so the known cleanup-stack caller (`RFs::GetDir`)
  is not reachable from `no_std` yet — the reproduction needs one.

- **Reproduced, measured, not inferred.** A `no_std` console application calling
  `RFs::GetDir` through `symbian-sys` dies: emulator log line
  `thread.cpp:542 [Kernel]: Thread Main panicked with category: E32USER-CBase and exit
  code: 69` (= `EClnNoTrapHandlerInstalled`). No result file is written. The same call
  under `std` works, because `std` installs `CTrapCleanup`.
- Seeing that line needs `Kernel:trace` in `~/.local/share/EKA2L1/config.yml`; the
  stock `Kernel:Warn` hides it. Config restored after the run.
- Scratch probe used for this: `symbian-rs/examples/dirprobe` (uid3 `0xe00006a1`),
  not a deliverable — delete or turn into the durable test.

## Decisions

## Dead ends

## Next step

Move `TrapCleanup` to a crate both paths can reach (not above `symbian-sys`, which is
all `std` depends on) and install it in `symbian_runtime::entry!`; then re-run the
probe and watch the same call succeed.
