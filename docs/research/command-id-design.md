# Command ids: how a menu item's number reaches the Rust source

Research note, 2026-09-21. Branch `command-id-design`. Cites: `docs/research/avkon-rust-spec.md`
§6 and its `[91]` paragraphs, experiment 81 (a proc macro in this build) and 91 (the menu) in
[experiment-backlog.md](experiment-backlog.md), `crates/symdev-manifest/src/command_id.rs`,
`symbian-rs/crates/symbian-ui/src/command.rs`, `crates/symdev-build/src/ui_resources.rs`,
`symbian-rs/crates/symbian-macros`, and the SDK's
`S60_3rd_FP2_examples/cpp_examples/helloworldbasic`.

## The question

An Avkon menu item carries a number that appears in two artefacts made by two compilers:
`MENU_ITEM { command = 0x41e0; … }` in the `.rss` symdev generates from `[[ui.menu]]` and
compiles with the native `rcomp`, and the `match` in `App::command`. The two must agree.

C++ never had to *match* them. `helloworldbasic.hrh` holds one `enum` with hand-picked
values (`EHelloWorldBasicCommand1 = 0x6001`), the `.rss` and the `.cpp` both `#include` it,
and `rcomp` runs the same `cpp` over the resource that the C++ compiler runs over the source
— one definition, and a typo in `EHelloWorldBasicCommandX` is a compile error. Labels live
in per-language `.rls` files and `LANG SC 01 09 31 32` in the `.mmp` compiles the resource
once per language.

symdev today (experiment 91): `[[ui.menu]] id = "more"` in the manifest,
`Command::named("more")` in Rust, and both sides compute `0x4000 | (FNV-1a-32(id) & 0x3fff)`
with a shared vector table pinning the two implementations together. The number is written
nowhere; the *word* is written twice. **The hole:** `Command::named("mroe")`, a renamed
manifest item, or a deleted one all compile cleanly and silently never fire. Nothing at
compile time links the manifest's set of ids to the names the source uses.

## Verdict

**Read `symdev.toml` from the `#[symbian_std::main(gui)]` attribute and emit a `menu` module
of constants** (`menu::MORE` for `id = "more"`). Prototyped on this branch, built for the
phone target, run through `symdev build`, and every wrong spelling demonstrated as a compile
error below. The status quo's number derivation is untouched — each constant is still
`Command::named("<id>")` — so the emitted E32 image is byte-identical in code to experiment
91's (header CRC and timestamp aside). What is new is that a name has to exist.

Its honest cost: `symbian-macros` stops being dependency-free — it depends on
`symdev-manifest` (path dependency across the two workspaces), which pulls `serde`, `toml`,
`thiserror`, `syn` and their support crates into the host-side build of every GUI
application (19 more `Cargo.lock` entries, 3.4 s wall for a clean build of the macro and
its dependencies, measured below); and the `gui` shape now *requires* a `symdev.toml` with
`[ui]` next to `Cargo.toml`, so a GUI crate can no longer be compiled from a bare cargo
checkout without one (it could not have linked anyway — the shim is symdev's).

## The candidates, and what was measured

Everything below was run, not reasoned about. Host scratch work lives in the session
scratchpad (`pm-exp/`); the target-side runs are in `symbian-rs/examples/ui` on this branch.

### A. Status quo: a word hashed on both sides

Keeps: no build step, no file, no dependency, `cargo build` works anywhere, `no_std` trivial.
Loses: the guarantee. A mistyped, renamed or deleted item is a menu line that never fires,
found only by pressing it in the emulator. The manifest refuses two ids that hash alike,
and the shared vector table catches the two hash functions drifting — but neither checks
that the Rust source names an id that exists. Rejected on the strength of the evidence for
C; nothing else about it is wrong.

### B. A build script writes `OUT_DIR/menu.rs`

Scratch (`pm-exp/brs`, host, stable 1.98.1): `build.rs` prints
`cargo::rerun-if-changed=<dir>/symdev.toml`, reads the file and writes `mod menu { pub const
… }` to `OUT_DIR`; the crate does `include!(concat!(env!("OUT_DIR"), "/menu.rs"))`.

- Works; same E0425 and the same "similarly named constant `FEWER` defined here" help as C.
- Rebuilds on a manifest edit (`Compiling brs` then `cannot find value FEWER`).
- Stable Rust, canonical mechanism.
- **Cost:** a `build.rs` *and* an `include!` line in every application, which the `symdev
  new` scaffold would have to write and which every hand-made project can forget. A build
  script is also a separate compiled and executed program per application, where a macro is
  one crate compiled once per workspace.

Ruled out only because C gives the same result for one line less per application.

### C. The `main(gui)` attribute reads the manifest and writes `mod menu` — chosen

Scratch first (`pm-exp/pm` + `pm-exp/app`, host), then the real thing in `symbian-macros`.

**Does a proc macro see the manifest?** Yes. `CARGO_MANIFEST_DIR` is set in rustc's
environment and therefore in the macro's; `std::fs::read_to_string("<it>/symdev.toml")`
works at expansion time.

**Does the crate rebuild when `symdev.toml` changes?** Not by itself — and this is the
trap. With the macro reading the file and nothing else:

```
$ sed -i 's/id = "fewer"/id = "less"/' app/symdev.toml && cargo build -p app
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s      # stale
```

A proc macro has no `rerun-if-changed`. Two mechanisms were verified:

1. **`include_str!` in the output** (stable): the macro emits
   `const _: &str = ::core::include_str!("<abs>/symdev.toml");`. rustc records every
   `include_str!` file in dep-info, cargo consults dep-info, and the crate recompiles:

   ```
   $ sed -i 's/id = "fewer"/id = "less"/' app/symdev.toml && cargo build -p app
      Compiling app v0.1.0
   error[E0425]: cannot find value `FEWER` in module `menu`
   ```
   `target/arm-symbian-e32/release/libuidemo.d` names `examples/ui/symdev.toml`; the
   archive does not contain the manifest text (an unused `const` is not codegen'd).
2. **`proc_macro::tracked::path(&path)`** (nightly, `#![feature(proc_macro_tracked_path)]`,
   tracking issue 99515). Verified on nightly-2026-09-19 with the same rename: recompiles,
   same error. Note the current name — the older `proc_macro::tracked_path::path` /
   `feature(track_path)` fail with `unknown feature`. Not used: `include_str!` does the same
   without tying the macro crate to nightly.

**Error messages** (all on the phone target, `cargo build --release -p uidemo --offline` in
`symbian-rs`, the source edited as shown):

Typo in the source (`menu::FEWRE`):
```
error[E0531]: cannot find unit struct, unit variant or constant `FEWRE` in module `menu`
   --> examples/ui/src/main.rs:124:19
    |
124 |             menu::FEWRE if self.bars > 1 => self.bars -= 1,
    |                   ^^^^^ not found in `menu`
    |
note: similarly named constant `FEWER` defined here
   --> examples/ui/src/main.rs:160:1
    |
160 | #[symbian_std::main(gui)]
    | ^^^^^^^^^^^^^^^^^^^^^^^^^
help: a constant with a similar name exists
    |
124 -             menu::FEWRE if self.bars > 1 => self.bars -= 1,
124 +             menu::FEWER if self.bars > 1 => self.bars -= 1,
```

Manifest renames `fewer` to `less`, source untouched — cargo recompiles (this is the
`include_str!` at work) and:
```
   Compiling uidemo v0.1.0 (…/symbian-rs/examples/ui)
error[E0531]: cannot find unit struct, unit variant or constant `FEWER` in module `menu`
   --> examples/ui/src/main.rs:124:19
```

Manifest deletes the `reset` item:
```
error[E0531]: cannot find unit struct, unit variant or constant `RESET` in module `menu`
   --> examples/ui/src/main.rs:125:19
```

Manifest has an id that cannot be a constant (`id = "Fewer bars"`) — the manifest crate's
own refusal, at the attribute:
```
error: `#[symbian_std::main(gui)]` cannot read …/examples/ui/symdev.toml: ui.menu.id
"Fewer bars" must be a lower case ASCII word (a letter, then letters, digits, `-` or `_`):
it becomes the Rust constant `menu::FEWER BARS`
   --> examples/ui/src/main.rs:160:1
    |
160 | #[symbian_std::main(gui)]
```

Manifest has two items named `more`:
```
error: `#[symbian_std::main(gui)]` cannot read …/symdev.toml: ui.menu has two items named
"more" ("More bars" and "Fewer bars")
```

Two ids that become one constant (`new-note` and `new_note`) are refused by
`symdev-manifest` with "would both be the constant `menu::NEW_NOTE`; rename one" (unit
test); a `[ui]`-less manifest under `main(gui)` and a missing manifest are refused naming
the path (unit tests in `symbian-macros/src/menu.rs`).

**What it does not catch:** a manifest item the source never handles. The constant is
emitted but unused, and rustc suppresses `dead_code` inside a proc-macro expansion, so
there is no warning. C++ is no better (an unused enumerator is silent). The way to make it
an error is to emit an `enum Menu` with `Menu::of(Command) -> Option<Menu>` and let the
`match` be exhaustive: verified on the host that a manifest gaining `reset` then gives
`error[E0004]: non-exhaustive patterns: `Some(Menu::Reset)` not covered`. Left as the
obvious extension — the constants are what the existing source shape (`match command {
MORE => …}`) wants, and the enum can be added to the same module later.

**`no_std`:** the target crate is `#![no_std]`; the module is `const` items typed
`::symbian_std::ui::Command`, nothing else. `cargo build --release -p uidemo --offline`
for `arm-symbian-e32` with `-Zbuild-std=core,alloc`: 2.98 s, no warnings.

**rust-analyzer** (1.98.1): expands the file-reading macro with `CARGO_MANIFEST_DIR` set.
`rust-analyzer lsif` on the scratch crate gives hover for `menu::MORE` =
`pub const MORE: i32 = 16864 (0x41E0)`, so goto-definition, hover and completion work.
(The `rust-analyzer diagnostics` CLI reports nothing even for a plain undefined function,
so it is not evidence either way.) Not verified: whether an open editor session re-expands
after a `symdev.toml` edit without a reload — the `include_str!` is a rustc dep-info fact,
not something rust-analyzer watches.

**`cargo build` without symdev:** works, because the macro reads the manifest itself; that
is what every measurement above used. `symdev build` (release CLI, the environment from
the brief) on `examples/ui`: 16.1 s wall, the generated `.rss` carries the four
`MENU_ITEM`s at `0x41e0`/`0x7612`/`0x73c0`/`0x4736` and is byte-identical to the copy in
`symbian-rs/corpus/91-ui-menu/`; `uidemo.exe` is 12 844 bytes, differing from that corpus
copy only at offsets 20–23 (`iHeaderCrc`) and 36–40 (`iTimeLo`/`iTimeHi`) — the code is
the same code, which is what "the constants cost nothing" means.

**Cost, measured:** `symbian-macros` gains `symdev-manifest = { path =
"../../../crates/symdev-manifest" }`. `cargo build -p symbian-macros --offline` from clean
under the nightly compiles proc-macro2, quote, unicode-ident, serde_core, serde, thiserror,
winnow, toml_writer, toml_parser, syn, toml_datetime, serde_spanned, toml, serde_derive,
thiserror-impl, symdev-manifest, symbian-macros in 3.36 s wall; `symbian-rs/Cargo.lock`
grows from 23 to 42 packages. The alternative — a hand-written `[[ui.menu]]` scanner in the
macro to stay dependency-free — was rejected on purpose: two readers of one file is the
"two compilers must agree" problem again, and a reader that sees an item symdev's does not
(a commented-out block, a table the real parser rejects) brings back the silent case.

**Why the attribute and not a separate `menu!()` macro:** `[ui]` present means an Avkon
application, and an Avkon application has this menu; putting it on the attribute the
application already writes means there is nothing to forget and nothing new to learn. The
constants land in `mod menu` beside `fn main`, which is the crate root in every example.
A separate function-like macro is the same code and a one-line change if a project ever
wants the module elsewhere.

### D. Rust as the source of truth: an ELF section symdev reads back

Verified feasible on the real target: a `#[used] #[unsafe(link_section = ".symdev.menu")]
static` in `examples/ui` survives `lto = true`, `opt-level = "s"`, `codegen-units = 1` into
`target/arm-symbian-e32/release/libuidemo.a`:

```
$ arm-none-symbianelf-objdump -h libuidemo.a | grep -A1 symdev.menu
 97 .symdev.menu  00000016  00000000  00000000  00002a24  2**0
                  CONTENTS, ALLOC, LOAD, READONLY, DATA
$ arm-none-symbianelf-objdump -s -j .symdev.menu libuidemo.a
 0000 6d6f7265 3d4d6f72 65206261 72730066  more=More bars.f
```

So symdev *could* run cargo, open the archive, read the menu out of the section and only
then write the `.rss`. Ruled out on design, not feasibility:

- It inverts the dependency the rest of `[ui]` already has. Caption, softkeys, icon and
  (later) languages are manifest facts read before cargo runs; the menu would be the one
  thing read *after* cargo, from a binary, through a serialisation format symdev and the
  macro must both know — a third artefact to keep in agreement.
- The manifest could no longer validate the menu (duplicate ids, the softkey/menu
  contradiction of experiment 91) before a build; those checks move into the macro or into
  a post-cargo step whose errors point at an archive.
- The section has to be kept out of the E32 image (or stripped) on the recorded link line
  — one more thing observed behaviour has to cover.
- Localisation gets harder, not easier: per-language labels would live in Rust tables and
  travel through the section, while the C++ design keeps labels in per-language resource
  files the compiler compiles once per language. The manifest side is where that extension
  is natural (below).

### E. symdev generates a Rust file before cargo

Not prototyped. It is candidate B with symdev instead of `build.rs` as the writer, and it
fails the "`cargo build` invoked directly" requirement by construction: the file is stale or
absent unless symdev ran first, and cargo has no way to know. A generated file checked into
the tree is worse (a second copy of the manifest to keep in sync). Ruled out.

### F. A symdev-side lint over the Rust source

Not prototyped. `grep`-ing `Command::named("…")` out of `src/` and comparing with the
manifest catches the literal case only, misses a name built any other way, and is a
symdev-time check rather than a compile-time one, so `cargo build` alone still passes.
Ruled out; C gives the compiler's own resolution instead.

## Localisation, in this design

The macro reads ids only and never sees a label, so labels can move without touching it.
The C++ layout — one `.rls` per language, `LANG` in the `.mmp`, the resource compiled once
per language into `.r01`/`.r09`/… — maps onto the manifest as labels keyed by id:

```toml
[[ui.menu]]
id = "more"
label = "More bars"          # the default, as today
label.fi = "Lisää palkkeja"  # or a `labels/fi.toml` keyed by id
```

`ui_resources.rs` would then write one `.rss` per language with the same `command =` numbers
and different `txt =`, and `symdev-manifest` would refuse a language file that names an id
the menu does not have — the same "one definition" check, at the same place. Nothing in the
Rust side changes: `menu::MORE` is language-independent, as `EHelloWorldBasicCommand1` was.
Candidate D would have had to carry every language's strings through the ELF section.

## What is on the branch

- `symbian-rs/crates/symbian-macros/src/menu.rs` — `Menu::load` (via
  `symdev_manifest::load`) and `Menu::module()`; five unit tests.
- `symbian-rs/crates/symbian-macros/src/lib.rs` — the `gui` shape appends the module;
  `entry.rs` exposes `Entry::shape`.
- `crates/symdev-manifest/src/ui.rs` — `MenuItem::constant()`, the id rule (lower case
  ASCII word: letter, then letters, digits, `-`, `_`) and the constant-clash refusal; three
  tests in `ui/tests.rs`.
- `symbian-rs/examples/ui/src/main.rs` — matches `menu::MORE` … `menu::QUIT`; the four
  `const … = Command::named(…)` lines are gone.
- `symbian-rs/crates/symbian-ui/src/app.rs` — `App::command` docs point at the module.
  `symbian-ui/src/lib.rs` still shows `command.is("more")` in its crate docs and was left
  alone because the `ui-list` branch is editing it.

Gates: `cargo test --workspace --offline` and `cargo clippy --workspace --all-targets
--offline` clean on the main workspace; `cargo test -p symbian-macros --offline` 22 passed,
`cargo clippy -p symbian-macros --all-targets --offline` clean, `cargo build --release -p
uidemo --offline` clean.

## If this is adopted: follow-ups, not done here

1. `Command::named` and `Command::is` can go, together with the target-side FNV and the
   shared vector table: with the macro emitting the number `symdev-manifest` computed
   (`MenuItem::command`), the hash has one implementation and one origin. It was kept in
   the prototype so the emitted image stays byte-comparable with experiment 91.
2. The `enum Menu` shape for exhaustive matches (verified above; same module).
3. `symdev new --language rust` with `[ui]` should scaffold a `match` over `menu::…`.
4. Per-language labels as sketched, once a second language is actually wanted.
