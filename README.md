# symdev — build Symbian S60 apps in Rust and C++ on Linux

[![CI](https://github.com/4akloon/symdev/actions/workflows/ci.yml/badge.svg)](https://github.com/4akloon/symdev/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust 1.98.1](https://img.shields.io/badge/rust-1.98.1-orange.svg)

**symdev** is a Linux toolchain for **Symbian OS 9.3 / S60 3rd Edition Feature Pack 2**
(target device: **Nokia E52**). It builds, packages, signs and runs Symbian applications
without Windows and without Wine: the resource compiler, the E32 post-linker (`elf2e32`), SIS
packaging and signing, and the icon and bitmap converters are reimplemented natively in Rust.

It also ships a **Rust SDK for Symbian**: write S60 applications in Rust — `no_std` first, with
a `std` port — using familiar Rust APIs (`fs`, `io`, `net`, `thread`, `time`, `write!`) and
native Avkon UI components (menus, list boxes, notes, query dialogs), with resource use measured
against the equivalent C++ program at every step.

> **Status.** Applications built by symdev install and launch in the
> [EKA2L1](https://github.com/EKA2L1/EKA2L1) emulator. That is an emulator result only: support
> for a physical Nokia E52 is **not** claimed until a stock device installs and launches an app.

## What it gives you

- **A native replacement for the SDK's Windows tools** — `rcomp`, `elf2e32`, `makesis`, `signsis`,
  `makekeys`, `bmconv`, `mifconv`/`svgtbinencode`, `uidcrc` — each checked against the output of
  the original tool.
- **Existing C++ projects, unchanged.** Point `symdev build` at a `bld.inf` / `.mmp` project and
  it builds with the GCCE cross compiler. The acceptance test takes a real third-party S60 app
  ([Simon Tatham's Puzzles for S60](https://github.com/tdionizio/puzzless60)) through build,
  package and run with **no edit** to the project
  ([docs/research/acceptance/puzzles.sh](docs/research/acceptance/puzzles.sh)).
- **Rust on Symbian** for new code — see below.
- **One command loop**: `symdev new` → `build` → `package` → `run` / `test --emulator`.

## Rust on Symbian in one screen

```rust
#![no_std]
use symbian_std::ui::prelude::*;

struct Bars { bars: u8 }

impl App for Bars {
    fn new() -> Self { Bars { bars: 3 } }

    fn draw(&self, gc: &mut Gc<'_>, area: Rect) { /* … */ }

    // The Options menu: labels next to the code that acts on them. No ids, no resources.
    fn menu(&self, m: &mut Menu<Self>) {
        m.item("More bars", |app| app.bars += 1);
        m.item("Fewer bars", |app| app.bars -= 1);
        m.exit("Exit");
    }
}

#[symbian_std::main(gui)]
fn main() -> Bars { Bars::new() }
```

Localised text lives in `locales/default.toml`, `locales/french.toml`, … beside `Cargo.toml`, and
`symbian_std::strings!()` turns its keys into constants (`strings::GREETING.get()?`). symdev
compiles each file into a per-language resource exactly as the C++ SDK does (`<app>_strings.r02`,
`.r93`, …); the platform picks the device's language and only that file is ever read. A missing
translation or a misspelt key is a compile error. The launcher caption is translated the same way.

What is there today: the `#[symbian_std::main]` entry point; `fs` with `read_dir`, `io`, `net`
(TCP and UDP), `thread`, `sync`, `time`; an async executor over the active scheduler; Avkon
applications with drawing, keys, menus, list boxes, notes and query dialogs; localisation; named
panics (`RUST` category, like a C++ `User::Panic`); a `write!` that formats without `core::fmt`;
and a `std` port for programs that want it. Each is a small example under
[`symbian-rs/examples/`](symbian-rs/examples) with a test that runs in the emulator. Design:
[docs/superpowers/specs/2026-09-20-rust-sdk-design.md](docs/superpowers/specs/2026-09-20-rust-sdk-design.md).

### Measured against C++

Every Rust slice is compared with the same program written in C++ against the SDK
([docs/research/cpp-parity.md](docs/research/cpp-parity.md)). E32 image sizes, in bytes:

| Example | C++ | Rust (`no_std`) |
|---|---|---|
| console hello | 802 | 1 245 |
| file round trip | 6 058 | 9 341 |
| localised strings | 6 647 | 8 202 |
| Avkon app with a menu | 7 317 | 11 645 |

On the heap the Rust side is at parity or better where it has been measured: reading a
localised string holds the open language file in 1 heap cell (C++: 4) and each string in 1
(C++: 1); listing a directory allocates nothing per entry, as in C++. The remaining size gaps
and their causes are tracked in [docs/research/size-levers.md](docs/research/size-levers.md).

## Requirements

symdev bundles none of these; you point it at your own copies.

| Needed | Why | Variable |
|---|---|---|
| S60 3rd FP2 SDK | headers, libraries, `.dso` stubs | `SYMDEV_EPOCROOT` |
| GCCE cross compiler (`arm-none-symbianelf-g++`) | C++ projects and the Rust SDK's C++ shims | `SYMDEV_GXX` |
| binutils `arm-none-symbianelf-ld` | linking | `SYMDEV_LD` |
| GCC runtime libraries | linking | `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` |
| Self-signing password (4+ characters) | `symdev package` | `SYMDEV_SIGN_PASSWORD` |
| Rust nightly, pinned in `symbian-rs/rust-toolchain.toml` | the Rust SDK (`-Zbuild-std`) | — |
| EKA2L1 (optional) | `symdev run`, `symdev test --emulator` | `SYMDEV_EKA2L1` |

EKA2L1 is GPL-3.0 and runs as a separate process; it is never linked into or copied into this
repository. The SDK, ROM images and real signing keys are never committed. The only key material
in the tree is the throwaway DSA keys, certificate and signed test packages under
[`crates/symdev-sis/src/testdata/`](crates/symdev-sis/src/testdata): fixtures for the signer's
tests, which sign nothing else.

## Quick start

```bash
cargo build --release --workspace
export PATH="$PWD/target/release:$PATH"

symdev new hello --lang rust              # or --lang cpp; add --template gui for an Avkon app
cd hello
symdev build                              # build/hello.exe
symdev package                            # build/hello.sisx, self-signed
symdev run                                # install and launch in EKA2L1
symdev test --emulator                    # run it and read back its test report
```

Full environment setup and the C++ examples: [examples/README.md](examples/README.md).

## Commands

| Command | Does |
|---|---|
| `symdev new <name> [--lang cpp\|rust] [--template console\|gui]` | scaffolds a project |
| `symdev build` | compiles, links and post-links into `build/` |
| `symdev package` | builds `build/<name>.sisx`, self-signed |
| `symdev run` | installs the `.sisx` into EKA2L1 and launches it by UID3 |
| `symdev test --emulator` | runs the app in EKA2L1 and reports the result file it wrote |
| `symdev freeze` | appends a DLL's new exports to its frozen `.def` so ordinals stay fixed |
| `symdev deploy` | prints the path of the `.sisx`; no device transport exists yet |

A project is described by `symdev.toml` — package, target, `[symbian]` UID3 / capabilities /
icon, `[signing]`, and `[ui]` for an Avkon app — plus `bld.inf` / `.mmp` for C++ or `Cargo.toml`
for Rust. Self-signed packages can only use user-grantable capabilities and a UID3 in
`0xA0000000..0xAFFFFFFF` or `0xE0000000..0xEFFFFFFF`.

## Repository layout

| Path | What |
|---|---|
| [`crates/`](crates) | the host toolchain: `symdev-cli` (the `symdev` command) and the native tools |
| [`symbian-rs/`](symbian-rs) | the Rust SDK — `symbian-sys`, `symbian-core`, `symbian-std`, `symbian-ui`, the runtime, the C++ shims — and its examples |
| [`examples/`](examples) | C++ `hello` and `gui` projects |
| [`docs/research/`](docs/research) | specifications of every file format and tool, and the numbered experiment log |
| [`docs/superpowers/`](docs/superpowers) | design specs and implementation plans |
| [`docker/`](docker) | a fallback Ubuntu image |

| Crate | Role | Binary |
|---|---|---|
| [`symdev-cli`](crates/symdev-cli) | the `symdev` command | `symdev` |
| [`symdev-build`](crates/symdev-build) | `bld.inf`/`.mmp` front end, GCCE and Rust build drivers, packaging | – |
| [`symdev-manifest`](crates/symdev-manifest) | `symdev.toml` parsing and validation | – |
| [`symdev-locale`](crates/symdev-locale) | `locales/*.toml`, shared by the build and the `strings!` macro | – |
| [`symdev-core`](crates/symdev-core) | shared types, errors and backend traits | – |
| [`symdev-emulator`](crates/symdev-emulator) | EKA2L1 launcher and result reader | – |
| [`symdev-sis`](crates/symdev-sis) | SIS encoding, signing, `makesis` | `makesis`, `signsis` |
| [`symdev-makekeys`](crates/symdev-makekeys) | self-signed DSA certificate and key | `makekeys` |
| [`symdev-rcomp`](crates/symdev-rcomp) | `.rss` → `.rsc` resource compiler | `rcomp` |
| [`symdev-elf2e32`](crates/symdev-elf2e32) | ELF → E32 image post-linker | `elf2e32` |
| [`symdev-mif`](crates/symdev-mif) | SVG Tiny → SVGB, `.mif` icon container | – |
| [`symdev-mbm`](crates/symdev-mbm) | BMP → `.mbm` bitmap store | – |
| [`symdev-uidcrc`](crates/symdev-uidcrc) | UID checksum | `uidcrc` |

## How the native tools were made

The SDK's tools are reimplemented from specifications and golden output, never by copying their
source. Each format was pinned down by experiments against the original tools; behaviour that
was never observed returns an error of the form `TODO: … (not observed)` rather than a guess.
Where a specification had to be written from a tool's source or disassembly, one party wrote a
prose specification and another implemented it without reading the source. The clean-room
record is in [docs/research/licensing.md](docs/research/licensing.md), and every experiment is in
[docs/research/experiment-backlog.md](docs/research/experiment-backlog.md).

## Development

Rust 1.98.1, edition 2024, for the host tools; the pinned nightly in `symbian-rs/` for the SDK.

```bash
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline
```

Both are kept free of warnings. Working conventions are in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

[MIT](LICENSE). Symbian, S60 and Nokia are trademarks of their respective owners; this project
is not affiliated with or endorsed by them.
