# Contributing to symdev

Thank you for your interest. The project has a few rules that are stricter than usual, because
it reimplements proprietary tools and targets a device most contributors will not have.

## Before you open a pull request

```bash
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline
```

Both must pass with **no warnings**. For changes to the Rust SDK, also build in `symbian-rs/`
(`cargo clippy --release --workspace --offline`) and run the affected examples with
`symdev test --emulator`.

## Rules

- **Never commit** the S60 SDK, ROM images, `.sis`/`.sisx` packages you did not build for a test,
  or real signing keys and certificates. Test fixtures are the one exception, and they live in
  `testdata/`.
- **No guessing.** Behaviour that was never observed from the original tools, the SDK headers or a
  run returns an error of the form `TODO: … (not observed)`. Cite the header line, the `nm -D`
  line or the experiment a fact comes from.
- **Clean room.** Do not read or copy the source of the SDK's tools, or of GPL projects such as
  EKA2L1, into this repository. Where a tool's behaviour must be derived from its source or
  disassembly, one person writes a prose specification and another implements it without reading
  the source; record who read what in `docs/research/licensing.md`.
- **Emulator is not a device.** Results from EKA2L1 are stated as emulator results.
- **Measure against C++.** A change to the Rust SDK reports image size, heap and behaviour against
  the equivalent C++ program (`docs/research/cpp-parity.md`).
- **Code style.** Library code returns `Result` — no `unwrap`, `expect` or `panic!` outside tests;
  an error names what failed and, when known, the fix. Every `.rs` file is at most 300 lines.
  New API is a domain type with methods, named after the domain concept.
- **Commits.** One full imperative sentence ending with a period, e.g.
  "Add read_dir to the no_std shape, borrowing from the CDir as C++ does."

The fuller working notes used during development are in [CLAUDE.md](CLAUDE.md); experiments are
numbered in [docs/research/experiment-backlog.md](docs/research/experiment-backlog.md).
