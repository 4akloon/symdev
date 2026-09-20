# symdev-build

The orchestrator behind `symdev build` and `symdev package`: it reads a project's `bld.inf` and
`.mmp` files, drives the GCCE compiler and linker, post-links with the native `symdev-elf2e32`,
compiles resources and icons with the native tool crates, and packs and signs the `.sisx`.

## What it does

- **Front end**: `BldInf` and `Mmp` model and parse `bld.inf` / `.mmp` (targets, sources, libraries,
  capabilities, resources, bitmaps, exports), using its own C preprocessor for the SDK's
  conditional directives. A `bld.inf` makefile hand-off is skipped with a warning.
- **Build**: `GcceBuild` (an `impl BuildBackend`) compiles every module with the recorded GCCE
  flag order, links it with `SYMDEV_LD`, and runs `symdev-elf2e32` on the recorded argv. `.cpp`
  and `.c` sources are compiled; other source kinds are an error.
- **Resources and icons**: `START RESOURCE` blocks go through `symdev-rcomp`; the app icon and
  `[[icons]]` containers go through `symdev-mif` / `symdev-mbm`; `START BITMAP` blocks produce
  `.mbm` files and their `.mbg` headers.
- **DLLs**: `DllExports` / `FrozenExports` track exports against frozen `.def` files
  (`eabi/<name>u.def`) so ordinals stay stable.
- **Packaging**: `SisPackage` (an `impl PackageBackend`) writes a `.pkg`, encodes the SIS with
  `symdev-sis` and signs it with a self-signed DSA certificate from `symdev-makekeys` (or an
  existing `cert` / `key` pair).
- **Toolchain**: `Toolchain::from_env` and `Epocroot::from_env` read the `SYMDEV_*` variables. The
  compiler, the linker and the SDK stay external.

## Example

```rust,ignore
use symdev_build::{GcceBuild, Toolchain};
use symdev_core::{BuildBackend, LocalEnv, Project};

let artifacts = GcceBuild {
    env: LocalEnv,
    tools: Toolchain::from_env()?,
    uid3: 0xef9f2cab,
    capabilities: vec![],
    icon: None,
    icons: vec![],
}
.build(&Project { root: std::env::current_dir()? })?;
```

## Behaviour that is not implemented

Anything the original tools were never observed doing is refused with a
`TODO: … (not observed)` error rather than guessed: for example MMP `VENDORID`, unknown
`START` blocks, an absolute `DEFFILE`, and bitmap icons in the `[symbian] icon` shorthand.

## Testing

```bash
cargo test -p symdev-build --offline
```
