# symdev-elf2e32

The post-linker: turns the ELF that `arm-none-symbianelf-ld` produces into a Symbian E32 image
(`.exe` or `.dll`), and writes the import stubs (`.dso`) and definition files (`.def`) DLLs need.
Native replacement for `elf2e32`.

## What it does

- Reads a 32-bit little-endian ARM ELF (`ElfImage`: segments, symbols, relocations).
- Lays out and writes the E32 image: header, code, data and relocation sections, import blocks
  and export table (`E32Image`, `E32ImageHeader`, `E32Exports`, …).
- Compresses the image with the E32 deflate variant (`E32Deflate`), or leaves it as is with
  `--uncompressed`.
- For DLLs, writes the import library `.dso` with its symbol-hash table (`E32Dso`) and the
  `.def` file (`E32DefFile`). With `--definput` it reads a frozen `.def` so existing ordinals
  keep their numbers and new exports get the next ones.
- Encodes capabilities and UIDs, with the UID checksum from `symdev-uidcrc`.

## Binary

```bash
elf2e32 --uid1=0x1000007a --uid3=0xef9f2cab --fpu=softvfp --targettype=EXE \
    --output=hello.exe --elfinput=hello.elf --linkas='hello{000a0000}[ef9f2cab].exe' \
    --libpath=$SYMDEV_EPOCROOT/epoc32/release/armv5/lib
```

Extra DLL options: `--targettype=DLL --uid2 --sid --dso --defoutput --definput --dlldata
--ignorenoncallable`. The binary prints a warning for every export not yet frozen. `Elf2E32` is the
parsed job; `Elf2E32Tool` builds argv for an external `elf2e32` binary (`SYMDEV_ELF2E32`).

Only options observed from the original tool are accepted: `--targettype` is `EXE` or `DLL`,
`--fpu` is `softvfp`, `--sid` must equal the UID3, and ELF features never seen in the recorded
corpus (unusual relocations, ELF without a writable segment, …) fail with
`TODO: … (not observed)`.

## Specs

Written down in [e32-deflate-spec.md](../../docs/research/e32-deflate-spec.md),
[dso-hash-spec.md](../../docs/research/dso-hash-spec.md) and
[elf2e32-options-spec.md](../../docs/research/elf2e32-options-spec.md); the process that produced
them is described in [licensing.md](../../docs/research/licensing.md).

## Testing

```bash
cargo test -p symdev-elf2e32 --offline
```
