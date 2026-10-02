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
| console hello | 802 | 968 |
| file round trip | 6 058 | 8 288 |
| localised strings | 6 647 | 8 122 |
| Avkon app with a menu | 7 317 | 10 315 |

On the heap the Rust side is at parity or better where it has been measured: reading a
localised string holds the open language file in 1 heap cell (C++: 4) and each string in 1
(C++: 1); listing a directory allocates nothing per entry, as in C++. The remaining size gaps
and their causes are tracked in [docs/research/size-levers.md](docs/research/size-levers.md).

## Requirements

| Needed | Why | Comes from |
|---|---|---|
| GCCE cross compiler (GCC 12.1.0 + binutils 2.29.1, `arm-none-symbianelf`) | C++ projects and the Rust SDK's C++ shims | package `gcce;12.1.0`, or `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` |
| S60 3rd FP2 SDK (headers, `.dso` stubs, static libraries) | compiling and linking | package `sdk;s60-3rd-fp2;1.1`, or `SYMDEV_EPOCROOT` |
| Self-signing password (4+ characters) | `symdev package` | `SYMDEV_SIGN_PASSWORD` |
| Rust SDK (`symbian-rs/`) | Rust projects only | `SYMDEV_RUST_SDK`, else the checkout symdev was built from, else package `rust-sdk;<symdev's version>` |
| Rust nightly, pinned in `symbian-rs/rust-toolchain.toml`, and a host C linker (`cc`, e.g. `build-essential`) | Rust projects only (`-Zbuild-std`; build scripts and the SDK's proc macros link on the host, as for any Rust project); a C++ project needs neither | rustup, your distribution |
| EKA2L1 (optional) | `symdev run`, `symdev test --emulator` | `SYMDEV_EKA2L1` |

### Toolchain packages

`symdev build` installs the GCCE and platform SDK packages it is missing into `SYMDEV_HOME`
(default `~/.local/share/symdev`), and for a Rust project the Rust SDK, printing one line per
download, and touches no network once they are there. `--offline` forbids downloading: a missing package is then an error that names
the install command. `symdev package` installs nothing. Downloads are cached in
`~/.cache/symdev/downloads` and checked against the index's SHA-256 before they are unpacked.

Each `SYMDEV_*` toolchain variable that is set overrides its package path, field by field, so an
environment that sets all of them installs nothing and builds as before. `SYMDEV_AR` overrides
the `ar` that otherwise sits beside the linker, and `SYMDEV_ELF2E32` an external post-linker in
place of the native one.

Packages come from symdev's built-in public source, searched first, and then from the
sources listed in `~/.config/symdev/sources.toml` (`$XDG_CONFIG_HOME/symdev/sources.toml`), in
order. The built-in source is the owner's public bucket,
`https://pub-15670d2771364287b9982e497c29f586.r2.dev/`, and every symdev already queries it; it
carries GCCE, `symdev` and `rust-sdk` from the first release on. Until that release its
`index.toml` does not exist (HTTP 404) and the source is skipped, so for now list a source or set
the variables. The S60 SDK is not redistributable and the built-in source never carries it: it lives in the owner's
private bucket, which a build reads only when that source is listed here **and** its key is in
the environment. Otherwise set `SYMDEV_EPOCROOT` to your own SDK.

```toml
# ~/.config/symdev/sources.toml
[[source]]
name = "private"
url = "https://<account-id>.r2.cloudflarestorage.com/symdev-private/"
auth = "s3"
key = "builtin"   # its index must carry symdev's own signature (symdev 0.2.0 and newer)
```

```bash
# the keys are read only from the environment: SYMDEV_SOURCE_<NAME>_…
export SYMDEV_SOURCE_PRIVATE_ACCESS_KEY_ID=...
export SYMDEV_SOURCE_PRIVATE_SECRET_ACCESS_KEY=...
```

A `url` may also be `file:///<directory>` holding an `index.toml` and its archives (a local
mirror); `builtin = false` at the top of the file turns the built-in source off.

Indexes are signed. The first line of the built-in source's `index.toml` is
`# symdev-signature: ed25519 <base64>`, the project key's Ed25519 signature of the rest of the
file, and symdev carries the public key: an index that is unsigned, or whose signature does not
verify, is not used, and the error names its URL (a warning, even when a later source has the
package). A listed source is held to the same rule when
it has a `key`: `"builtin"` for the project key (the owner's private bucket is signed with it),
or the base64 of another Ed25519 public key (a mirror you sign yourself). Without `key` its index
is read unverified, as before; symdev 0.1.0 refuses a `key` it does not know, so add the line
once 0.2.0 is installed. SHA-256 and size are checked either way, so a signed index protects the
packages too.

| Command | Does |
|---|---|
| `symdev sdk list` | installed packages, and those the sources offer |
| `symdev sdk install ['<id>'…]` | installs the ids; with none, what the current project needs |
| `symdev sdk uninstall '<id>'…` | removes installed packages |

Package ids contain `;` (`gcce;12.1.0`), so quote them in a shell. There is no `update`: a new
version is a new id, installed beside the old one, which stays until it is uninstalled.

A Rust project's SDK is `SYMDEV_RUST_SDK` when it is set; else the `symbian-rs/` of the source
checkout symdev was built from, while that still exists, so a developer working on the SDK
keeps building against their tree; else the `rust-sdk` package of the same version as symdev,
installed like GCCE. A Rust project names its SDK only through the link `build/rust-sdk`
(`build/rust-sdk/symbian-rs/crates/…` in `Cargo.toml`, `build/rust-sdk/symbian-rs/targets/…` in
`.cargo/config.toml`), which `symdev new --lang rust` makes and every `symdev build` points at the
SDK it resolved, so an upgrade or an uninstalled package never leaves a stale path in the project;
when the link moves to another SDK, `build/cargo` is rebuilt from scratch. `symdev build` stops,
with the exact lines to change, when the project still names another SDK by absolute path (as
symdev 0.1.0 scaffolds did) or its `rust-toolchain.toml` names another nightly than the SDK's.

EKA2L1 is GPL-3.0 and runs as a separate process; it is never linked into or copied into this
repository. The SDK, ROM images and real signing keys are never committed. The only key material
in the tree is the throwaway DSA keys, certificate and signed test packages under
[`crates/symdev-sis/src/testdata/`](crates/symdev-sis/src/testdata): fixtures for the signer's
tests, which sign nothing else.

## Quick start

Get `symdev` one of two ways.

**Prebuilt** — available from the first release on; until then, build from source:

```bash
curl -fsSL https://pub-15670d2771364287b9982e497c29f586.r2.dev/install.sh | sh
```

It installs the newest `symdev` package into `SYMDEV_HOME` and links `~/.local/bin/symdev` to
it; running it again updates. A C++ project then needs no Rust at all. A Rust project needs
rustup (the project's `rust-toolchain.toml` names the nightly) and a host C linker `cc`, as any
Rust project with build scripts does, and its first `symdev build` installs the `rust-sdk`
package beside GCCE.

**From source**, with Rust 1.98.1, in a clone of this repository:

```bash
cargo build --release --workspace
export PATH="$PWD/target/release:$PATH"
```

A `symdev` built this way builds Rust projects against the clone's `symbian-rs/` for as long as
it is there.

Then:

```bash
symdev new hello --lang rust              # or --lang cpp; add --template gui for an Avkon app
cd hello
symdev build                              # installs missing toolchain packages; build/hello.exe
symdev package                            # build/hello.sisx, self-signed
symdev run                                # install and launch in EKA2L1
symdev test --emulator                    # run it and read back its test report
```

The first `symdev build` needs the `SYMDEV_*` variables, or a source for each package: the S60
SDK comes only from a source you list in `~/.config/symdev/sources.toml` with its key in the
environment ([Toolchain packages](#toolchain-packages)). The C++ examples: [examples/README.md](examples/README.md).

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
| `symdev sdk list\|install\|uninstall` | manages the toolchain packages ([Requirements](#toolchain-packages)) |

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
