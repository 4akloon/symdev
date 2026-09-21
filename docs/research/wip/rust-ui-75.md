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

- **The s60 shim compiles.** `shims/s60/symrs_avkon.cpp` with the recorded GCCE argv
  plus `-I <sdk-include-casefold>` and `-DSYMRS_UID3=…`: `.o` is 31 516 bytes with
  **186 undefined symbols** (the C++ `examples/gui` has 182, experiment 76's shim 185 —
  subclassing cost is fixed). Without the case-fold overlay it fails at
  `fbs.h` → `FbsMessage.h`, so the s60 shim needs the overlay the C++ path already
  builds; `shims/common` does not.
- Link order problem found by reading: the shim archive follows the Rust archive, so
  the shim's reference to `symrs_app_vtbl` would never be resolved (ld does not
  rescan). `-u symrs_app_vtbl` before the Rust archive is the fix.
- **`examples/ui` builds and the size cliff did not come back: `uidemo.exe` is 7 542
  bytes**, against experiment 76's 107 028 for a mixed C++/Rust UI probe. Eleven
  `NEEDED`: the six of the recorded line plus apparc, cone, eikcore, avkon, gdi.
  `uidemo.rsc` 140, `uidemo_reg.rsc` 91, `uidemo_aif.mif` 268.

## Decisions

- **Forwarding is shape B** of the spec, unchanged: one `.cpp`, one `.h`, two tables
  with a `size` word. `set_brush` gained a third argument (`solid`) so one Rust call
  covers `SetBrushStyle` + `SetBrushColor`; everything else is the spec's ABI.
- **`Draw` hands Rust a rect whose origin is (0,0)** (`TRect(TPoint(0,0), Size())`),
  not `Rect()`. Drawing through a window gc is window-relative, so this removes the
  spec's open item C from the application's view entirely — to be confirmed in pixels.
- **`#[symbian_std::main(gui)]` writes no `E32Main`.** The shim owns it. The attribute
  exports `symrs_app_vtbl` and reads the application type from `fn main`'s **return
  type**, so the type is never written twice.
- `symbian-ui` is reached as `symbian_std::ui` (one re-export line), so an application
  still names one crate.

## Dead ends

## Next step

- Read `symbian-macros`, `symbian-std/src/lib.rs`, `shims/common/symrs_shim.h`, an
  example's `main.rs` and `Cargo.toml`; then design the `[ui]` section and the crate.
