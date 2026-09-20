# symdev

A Rust toolchain that builds, packages, signs and runs Symbian S60 3rd Edition FP2 applications
(target device: Nokia E52) from Linux. It replaces the SDK's Windows-only tools with native code:
resource compiler, icon and bitmap converters, E32 post-linker, SIS packaging and signing are all
implemented in this repository. No Wine is needed to build.

> **Status.** Apps built by symdev install and launch in the [EKA2L1](https://github.com/EKA2L1/EKA2L1)
> emulator. That is an emulator result only: E52 support is not claimed until a stock device
> installs and launches an app.

## What is still external

symdev does not bundle any of these; you point it at your own copies.

| Needed | Why | Variable |
|---|---|---|
| S60 3rd FP2 SDK | headers, libraries, `.dso` stubs | `SYMDEV_EPOCROOT` |
| GCCE cross compiler (`arm-none-symbianelf-g++`) | compiling C++ | `SYMDEV_GXX` |
| binutils `arm-none-symbianelf-ld` | linking | `SYMDEV_LD` |
| GCC runtime libraries | linking | `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` |
| Self-signing password (4+ characters) | `symdev package` | `SYMDEV_SIGN_PASSWORD` |
| EKA2L1 (optional) | `symdev run` | `SYMDEV_EKA2L1` |

`SYMDEV_ELF2E32` is optional: set it only to use an external `elf2e32` instead of the native one.
EKA2L1 is GPL-3.0 and runs as a separate process; it is never linked into or copied to this repo.
SDK, ROM, `.sis`, `.sisx`, `.cer` and `.key` files are never committed.

## Quick start

```bash
cargo build --release --workspace
export PATH="$PWD/target/release:$PATH"

symdev new hello --target nokia-e52     # or: --template gui
cd hello
symdev build                            # build/hello.exe
symdev package                          # build/hello.sisx, self-signed
symdev run                              # install + launch in EKA2L1
```

Full environment setup and the two example projects: [examples/README.md](examples/README.md).

## Commands

| Command | Does |
|---|---|
| `symdev new <name> --target nokia-e52 [--template console\|gui]` | scaffolds a project (`symdev.toml`, `group/`, `src/`) |
| `symdev build` | compiles, links and post-links every MMP of `group/bld.inf` into `build/` |
| `symdev package` | builds `build/<name>.sisx`, self-signed |
| `symdev run` | installs the `.sisx` into EKA2L1 and launches it by UID3 |
| `symdev freeze` | appends a DLL's new exports to its frozen `.def` so ordinals stay fixed |
| `symdev deploy` | only prints the path of the `.sisx`; no device transport exists yet |

Projects are described by `symdev.toml` (package, target, `[symbian]` UID3 / capabilities / icon,
`[signing]`, `[[install]]`, `[[icons]]`) plus the usual `bld.inf` and `.mmp` files. Self-signed
packages can only use user-grantable capabilities and a UID3 in `0xA0000000..0xAFFFFFFF` or
`0xE0000000..0xEFFFFFFF`.

## Workspace

`symdev-cli` is the only product binary. It drives `symdev-build` (which orchestrates the tool
crates `sis`, `makekeys`, `rcomp`, `elf2e32`, `mif`, `mbm`), `symdev-manifest` and
`symdev-emulator`. `symdev-sis`, `symdev-rcomp` and `symdev-elf2e32` use `symdev-uidcrc`; every
crate except `symdev-manifest` and `symdev-uidcrc` uses `symdev-core` for errors and shared types.

| Crate | Role | Binary |
|---|---|---|
| [`symdev-cli`](crates/symdev-cli) | the `symdev` command | `symdev` |
| [`symdev-build`](crates/symdev-build) | `bld.inf`/`.mmp` front end, GCCE build driver, packaging | – |
| [`symdev-manifest`](crates/symdev-manifest) | `symdev.toml` parsing and validation | – |
| [`symdev-core`](crates/symdev-core) | shared types, errors and backend traits | – |
| [`symdev-emulator`](crates/symdev-emulator) | EKA2L1 launcher | – |
| [`symdev-sis`](crates/symdev-sis) | SIS encoding, signing, `makesis` | `makesis`, `signsis` |
| [`symdev-makekeys`](crates/symdev-makekeys) | self-signed DSA certificate and key | `makekeys` |
| [`symdev-rcomp`](crates/symdev-rcomp) | `.rss` → `.rsc` resource compiler | `rcomp` |
| [`symdev-elf2e32`](crates/symdev-elf2e32) | ELF → E32 image post-linker | `elf2e32` |
| [`symdev-mif`](crates/symdev-mif) | SVG Tiny → SVGB, `.mif` icon container | – |
| [`symdev-mbm`](crates/symdev-mbm) | BMP → `.mbm` bitmap store | – |
| [`symdev-uidcrc`](crates/symdev-uidcrc) | UID checksum | `uidcrc` |

Other directories: [`examples/`](examples) (`hello`, `gui`), [`docker/`](docker) (fallback Ubuntu
image), [`symbian-rs/`](symbian-rs) (experimental Rust-on-Symbian corpus), [`docs/`](docs)
(research notes, specs, plans).

## How the native tools were made

The SDK tools are reimplemented against specifications and golden output, never by copying their
source. Each format was pinned down by experiments run against the original tools; behaviour that
was never observed returns an error of the form `TODO: … (not observed)` rather than a guess. The
clean-room process, and the record of who read what, is in
[docs/research/licensing.md](docs/research/licensing.md); the experiments are in
[docs/research/experiment-backlog.md](docs/research/experiment-backlog.md).

## Development

Rust 1.98.1, edition 2024.

```bash
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline
```

Both are kept free of warnings. Working conventions (file size limits, error style, branches) are
in [CLAUDE.md](CLAUDE.md).

## License

Undecided; the repository has no `LICENSE` file yet. See
[docs/research/licensing.md](docs/research/licensing.md).
