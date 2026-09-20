# WIP: attribute-macro entry point for the Rust SDK (step 81)

Replace `symbian_runtime::entry!(main)` with `#[symbian_std::main]` on a plain `fn main() -> Result<()>`.

## Findings

- A proc-macro crate works in `symbian-rs` with **no change to the build**: cargo builds it for the host even though `.cargo/config.toml` forces `build.target = arm-symbian-e32.json` and `-Zbuild-std=core,alloc`. Spike: trivial `#[symbian_macros::main]` on `examples/hello`, `cargo +nightly-2026-09-19 build --release -p hello` clean, `nm libhello.a` shows `T _Z7E32Mainv`.
- `cargo test -p symbian-macros` also works in that workspace (unit tests are built for the host), so the macro's logic can have ordinary `#[test]`s. `trybuild` is not in the offline registry cache; `syn`/`quote`/`proc-macro2` are.
- BUT the `proc_macro` API panics outside a proc-macro invocation ("procedural macro API is used outside of a procedural macro"), so tests cannot build a `TokenStream`. Logic must therefore be pure (`&str -> Result<String, String>`), with the shell stringifying `item` for inspection and re-emitting the original token stream unchanged (user spans survive).
- `#![no_main]` **can go**: `examples/hello` builds clean without it (the crate is a `staticlib` via `[lib] path = "src/main.rs"`, so rustc never looks for a `main`).

## Decisions

## Dead ends

## Next step

Build `crates/symbian-macros` for real (pure logic + tests), re-export as `symbian_std::main`. Old next step: read the spec (§6a, §5, §9, §11), experiments 65/69/79, and `symbian-rs/`.
