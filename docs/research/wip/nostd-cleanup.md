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

## Decisions

## Dead ends

## Next step

Reproduce the panic from a `no_std` console application before changing anything.
