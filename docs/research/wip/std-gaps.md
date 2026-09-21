# WIP: closing the gaps left by step 77 in `std` for `target_os = "symbian"`

Task: implement `sys/path` drive prefixes, `read_dir` + `args`, a real `sys/net` backend
and a partial `process` backend in the `symbian-rs/rust-src` overlay, keep `env`
`Unsupported`, and prove it all through `symdev test --emulator`.

## Findings

## Decisions

## Dead ends

## Next step

- Read the context: CLAUDE.md, overlay README, experiment 89, spec §6a and §11 step 77,
  the overlay's `sys/`, and `symbian-rs/crates/symbian-std/src/net/`.
