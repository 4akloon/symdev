# WIP: attribute-macro entry point for the Rust SDK (step 81)

Replace `symbian_runtime::entry!(main)` with `#[symbian_std::main]` on a plain `fn main() -> Result<()>`.

## Findings

- `TokenStream::to_string()` returns the **original source text**, comments and doc comments included (not `#[doc = "…"]`), so the header reader has to skip `//`, `///` and nested `/* */`. Found by a build failure on `examples/files`.

- A proc-macro crate works in `symbian-rs` with **no change to the build**: cargo builds it for the host even though `.cargo/config.toml` forces `build.target = arm-symbian-e32.json` and `-Zbuild-std=core,alloc`. Spike: trivial `#[symbian_macros::main]` on `examples/hello`, `cargo +nightly-2026-09-19 build --release -p hello` clean, `nm libhello.a` shows `T _Z7E32Mainv`.
- `cargo test -p symbian-macros` also works in that workspace (unit tests are built for the host), so the macro's logic can have ordinary `#[test]`s. `trybuild` is not in the offline registry cache; `syn`/`quote`/`proc-macro2` are.
- BUT the `proc_macro` API panics outside a proc-macro invocation ("procedural macro API is used outside of a procedural macro"), so tests cannot build a `TokenStream`. Logic must therefore be pure (`&str -> Result<String, String>`), with the shell stringifying `item` for inspection and re-emitting the original token stream unchanged (user spans survive).
- `#![no_main]` **can go**: `examples/hello` builds clean without it (the crate is a `staticlib` via `[lib] path = "src/main.rs"`, so rustc never looks for a `main`).

## Decisions

- The attribute lives in **`symbian-std`** (`#[symbian_std::main]`), re-exported from a dependency-free proc-macro crate `symbian-macros`. No umbrella `symbian` crate: an application's first lines are then `use symbian_std::prelude::*;` + `#[symbian_std::main]`, one crate name for the whole std-shaped surface, and `symbian-runtime` (panic handler, heap, `entry!`) arrives underneath it without being named.
- No `syn`/`quote`: the grammar is one function header, and a pure `&str` parser is what makes the logic testable at all.
- `IntoExitCode` gains `Result<T, E>` (generic) in `symbian-runtime`, `SymbianError` there (new `symbian-core` dependency, runtime is the top layer) and `io::Error` in `symbian-std`.
- `symbian_std::io::Result` gains a defaulted error parameter so the prelude can export `Result` without taking `Result<T, E>` away from a file that globs it.

- Verified end to end: `symdev new hello --language rust && symdev build && symdev package && symdev run` from an empty directory → `build/hello.exe` 3 187 B, `[Service.Notifier]: Trying to display: Hello from Rust SDK (19 chars)`, emulator exited on its own. `examples/files` through `symdev test --emulator`: `filesdemo: 16 passed`, exit 0.
- E32 sizes: hello 3 187 (=), hello-raw 752 (=), alloc 4 320 (=), files 10 423 (=), shim 4 474 vs 4 475 (−1). The shim ELF `.text` went 6 596 → 6 604 (+8 B: the `Result` conversion now happens in the exported wrapper instead of the example's own `fn main() -> i32`); the E32 is deflate-compressed, hence the −1.
- `__rt` was dropped: `ExitCode`/`IntoExitCode` are re-exported at the `symbian_std` root, so a wrong return type reads ``the trait bound `&str: IntoExitCode` is not satisfied`` and an application can implement the trait for its own error type.

## Dead ends

## Next step

Backlog entry 81, spec §9, fold this note in and delete it. Old: read the spec (§6a, §5, §9, §11), experiments 65/69/79, and `symbian-rs/`.
