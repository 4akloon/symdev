# rust-ui-75

Step 75 of the Rust SDK: an Avkon GUI application whose logic is Rust — `shims/s60/` C++
subclasses forwarding virtuals to a Rust vtable, `crates/symbian-ui`, `#[main(gui)]`, the
`.rsc`/`_reg.rsc`/`.mif` resource stage in `symdev-build`, and `examples/ui`.

## Findings

- Reading done: avkon-rust-spec.md in full, design spec §6a/§7/§9/§11, eka2l1-input.md,
  backlog 76/78/80/81/83.
- `RustBuild::build_shims` walks `RustSdk::shim_sources()` = `shims/common/*.cpp` only;
  `shim_dir()` is hard-coded to `common`. To add `s60` conditionally the SDK type needs a
  second directory and `RustBuild` a flag that says "this project asks for the UI".
- `RustBuild::link_args` inserts `-L…/lib -l:euser.dso -l:drtaeabi.dso` **before** the
  Rust archive (the 107 KB fix) and `--as-needed <RustSdk::LIBRARIES> --no-as-needed`
  before `-lsupc++`. The six Avkon libraries can join `sdk_libraries()`'s as-needed group
  without touching the ordering.
- `GcceBuild::compile_resource` is MMP-shaped (takes `&MmpResource`, `&Mmp`). The Rust
  path must call `symdev_rcomp::CPreprocessor::for_rss(&includes, &defines).run(&rss)` +
  `Rcomp::compile` directly — same byte-verified pair, no MMP.
- `AppIcon::of` requires `AppTarget::from_mmps`, so it cannot be used for a Rust project;
  only `AppIcon { source, app }` construction plus `GcceBuild::compile_icon` is reusable
  (`compile_icon` takes `&AppIcon` and is `pub(super)`).
- `examples/gui/data/{gui.rss,gui_reg.rss}` is the exact resource text to generate, and
  `crates/symdev-cli/templates/gui/*.rss` is its `{{NAME}}`/`{{UID3}}` template.
- `SisPackage` generates the fallback `_reg.rsc` only when the project has none
  (`package.rs:113`), so emitting a real one suppresses it.
- Manifest: `RawManifest` is `deny_unknown_fields`, so `[ui]` must be added there,
  in `Manifest`, and in `validate`.

## Decisions

## Dead ends

## Next step

- Read `symbian-macros`, `symbian-std/src/lib.rs`, `shims/common/symrs_shim.h`, an
  example's `main.rs` and `Cargo.toml`; then design the `[ui]` section and the crate.
