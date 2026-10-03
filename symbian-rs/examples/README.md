# Rust SDK examples

Each directory is a binary crate that cargo links through `symdev-ld` (design spec
`docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`, experiment 114). With `symdev`
and its `symdev-ld` link on `PATH` (`symdev setup-linker` makes the links for a checkout build,
`install.sh` for an installed symdev):

```bash
cd hello
cargo build --release   # build/<name>.exe and the signed build/<name>.sisx
cargo run --release     # on a device: SYMDEV_DEVICE, the running emulator, or a profile it starts
cd ../async
cargo test --release    # tests/executor.rs on the device, printed like libtest
```

A device needs `SYMDEV_EKA2L1`: an EKA2L1 with `--control` and `--data-dir` (see the root
README). The workspace's `.cargo/config.toml` names the target, `-Zbuild-std`,
`panic-abort-tests`, the linker and the runner. `std-hello` and `std-net` are workspaces of their
own, built against the patched `std`: run `symdev build` once in them (it makes
`build/sysroot` and the `build/symdev-rustc` link), then cargo as above.

Most examples also write a test report (`symbian_std::report!`, schema 1) that `symdev test
--emulator` prints the same way.
