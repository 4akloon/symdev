# symdev-cli

The `symdev` command: scaffold, build, package and run Symbian S60 3rd FP2 projects (Nokia E52).
It is a thin front end; the work is done by `symdev-build`, `symdev-manifest` and
`symdev-emulator`.

## Install

```bash
cargo install --path crates/symdev-cli --offline
```

## Commands

```
symdev new <name> --target nokia-e52 [--lang cpp] [--template console|gui]
symdev build
symdev package
symdev run
symdev freeze
symdev deploy
```

`new` creates `<name>/` with `symdev.toml`, `group/bld.inf`, `group/<name>.mmp` and `src/`; the
`gui` template adds an Avkon application, its resources and an SVG icon. The other commands run
in a project directory and read `./symdev.toml`.

| Command | Reads | Writes |
|---|---|---|
| `build` | `symdev.toml`, `group/bld.inf`, the MMPs | `build/<app>.exe` / `.dll`, `.dso`, `.rsc`, icons |
| `package` | the `build/` output, `[[install]]`, `[signing]` | `build/<name>.sisx` |
| `run` | `build/<name>.sisx` | `build/eka2l1.log`, `build/eka2l1.pid` |
| `freeze` | the DLL MMPs | `eabi/<name>u.def` |
| `deploy` | `build/<name>.sisx` | nothing; prints the path (no device transport yet) |

`build`, `package` and `run` need `symbian.uid3` in `symdev.toml`. Each command prints the path of
what it produced, so `symdev package` can be used in scripts.

## Environment

| Variable | Needed by |
|---|---|
| `SYMDEV_EPOCROOT`, `SYMDEV_GXX`, `SYMDEV_LD`, `SYMDEV_GCC_LIB`, `SYMDEV_GCC_TARGET_LIB` | `build` |
| `SYMDEV_EPOCROOT` | `package` and `freeze`, only for projects with a `bld.inf` |
| `SYMDEV_SIGN_PASSWORD` (4+ characters) | `package` |
| `SYMDEV_EKA2L1` | `run` |
| `SYMDEV_ELF2E32` | optional: use an external post-linker instead of the native one |

`symdev run` starts EKA2L1 in its own process group and does not stop it. EKA2L1 ignores SIGTERM,
so close its window yourself; a second `run` warns when the previous instance is still alive.

## Testing

Integration tests drive the built binary with `assert_cmd`. They also assert that
`examples/hello` and `examples/gui` equal what `symdev new` generates.

```bash
cargo test -p symdev-cli --offline
```
