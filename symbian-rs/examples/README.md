# Rust SDK examples

Each directory is a binary crate that cargo links through `symdev-ld` (design spec
`docs/superpowers/specs/2026-10-03-cargo-build-run-design.md`, experiment 114). With `symdev`
and its `symdev-ld` link on `PATH` (`symdev setup-linker` makes the link for a checkout build):

```bash
cd hello
cargo build --release   # build/<name>.exe and the signed build/<name>.sisx
cargo run --release     # installs and runs it on a device (symdev run --exe)
```

The workspace's `.cargo/config.toml` names the target, `-Zbuild-std`, the linker and the
runner. `std-hello` and `std-net` are workspaces of their own, built against the patched
`std`.
