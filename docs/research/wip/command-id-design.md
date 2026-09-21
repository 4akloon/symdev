# WIP: command-id design

Task: find the best design that ties a menu item's command number in the generated `.rss` to the Rust source so a mistyped / stale command is a compile error, not a silent no-op. Deliverable: `docs/research/command-id-design.md` + prototype on branch `command-id-design`.

## Findings

## Decisions

## Dead ends

## Next step
- Read current implementation: `crates/symdev-manifest/src/command_id.rs`, `symbian-ui` `Command::named`, `crates/symdev-build/src/ui_resources.rs`, `symbian-rs/crates/symbian-macros`.
- Status quo read: `CommandId::of` (host) and `Command::named` (target) = `0x4000 | FNV1a & 0x3fff`; manifest `ui.rs::menu` refuses duplicate ids / hash clashes; `ui_resources.rs::menu_rss` writes `command = 0x….`; `run_cargo_in` already passes `SYMDEV_UID3` env to cargo (precedent for symdev→cargo data flow, but a `cargo build` without symdev does not get it).
- symbian-rs is a separate workspace (nightly-2026-09-19, build-std, target JSON); `symbian-macros` is dependency-free by design (exp. 81); proc macros build for host with no change to the build. Offline registry has toml 0.8/1.1, serde, syn 2, quote, proc-macro2; **no trybuild**.
- Scratch proc macro (host, stable 1.98.1): `CARGO_MANIFEST_DIR` IS set at expansion; reading `symdev.toml` there works; `menu::FEWRE` typo → E0425 "cannot find value `FEWRE` in module `menu`" + "help: a constant with a similar name exists: FEWER" pointing at the macro call as the definition site.
- Rebuild: with NO tracking, editing symdev.toml does NOT recompile (`Finished` in 0.00s, stale consts) — a naive macro is a silent-staleness trap. Emitting `const _: &str = include_str!("<abs path>/symdev.toml");` in the expansion puts the file in dep-info (`app-*.d`) and cargo recompiles on change: rename fewer→less gives "cannot find value `FEWER`".
- Nightly-2026-09-19 has `proc_macro::tracked::path(&path)` behind `#![feature(proc_macro_tracked_path)]` (issue 99515; `tracked_path::path`/`track_path` are the OLD names and fail). Verified: with it, editing symdev.toml recompiles the app. Since symbian-rs is nightly-pinned anyway, both the stable `include_str!` trick and the nightly `tracked::path` work; `include_str!` is toolchain-independent.
- `rust-analyzer diagnostics` CLI prints nothing even for a plain undefined fn — not a usable probe for IDE behaviour; need LSIF/LSP hover for positive evidence.
- rust-analyzer 1.98.1 DOES expand the file-reading macro with `CARGO_MANIFEST_DIR` set: `rust-analyzer lsif` hover on `menu::MORE` = "pub const MORE: i32 = 16864 (0x41E0)". IDE goto/hover works.
- Unhandled manifest item (const emitted by macro but never used) gives NO dead_code warning: lints in proc-macro expansions are suppressed. Same as C++ (an unused enum value is silent). An enum shape (`Menu::of(raw) -> Option<Menu>`) makes it a hard E0004 "non-exhaustive patterns: `Some(Menu::Reset)` not covered" — verified on host.
- Rust-as-source-of-truth feasibility: `#[used] #[link_section = ".symdev.menu"] static` survives into a host staticlib under lto=true/opt-level=s/cgu=1 (readelf shows `.symdev.menu` PROGBITS with the bytes). Target proof still pending.
- Baseline `cargo build --release -p uidemo --offline` in the worktree: 8.2 s wall.
- build.rs candidate (scratch, host): `cargo::rerun-if-changed=symdev.toml` + `OUT_DIR/menu.rs` + `include!` gives the identical E0425/help message and rebuilds on manifest edits. Cost: a `build.rs` and an `include!` line in EVERY app (scaffold must write them); the macro route has neither.
- notes and query examples are `main(gui)` with no `[[ui.menu]]`; only `examples/ui` uses `Command::named`.
- Prototype in place: `symbian-macros/src/menu.rs` (`Menu::load` via `symdev_manifest::load`, `Menu::module()` emits `mod menu` + `include_str!` tracker), hooked into the `gui` shape in `lib.rs`; `symdev-manifest` `MenuItem::constant()` + id rule (lower-case ASCII word) + constant-clash refusal. symdev-manifest tests 47 ok, symbian-macros 22 ok. Adding symdev-manifest to the proc macro = 19 more lock entries, 3.4 s clean build of the macro and its deps.
