# T4 elf2e32: E32 layout from ELF

Slice: fill the ELF-derived E32 header fields from the frozen experiment-6 `hello.elf` instead of literals.

## Done

- `ElfImage` (`crates/symdev-elf2e32/src/elf.rs`): little-endian ELF32 ARM reader — header, `PT_LOAD` segments, `.dynsym` lookup by name. `Result` on every read; rejects non-ELF / non-LE32 / non-ARM.
- `E32Layout::from_elf` (`e32.rs`): code base/size, data base/size, BSS, entry, exception descriptor; `import_offset()`.
- Fixture `testdata/hello.elf.hex` (experiment-6 `hello.elf`, 24244 bytes). The frozen `hello.exe` header test now builds these fields from the ELF and still byte-equals the golden.

| Field | Derivation | hello |
|---|---|---|
| `iCodeBase` / `iCodeSize` / `iTextSize` | executable `PT_LOAD` vaddr / filesz | 0x8000 / 0x144c |
| `iEntryPoint` | `e_entry − code base` | 0x1098 |
| `iDataBase` / `iDataSize` / `iBssSize` | writable `PT_LOAD` vaddr / filesz / memsz−filesz | 0x400000 / 0 / 4 |
| `iExceptionDescriptor` | `.dynsym Symbian$$CPP$$Exception$$Descriptor − code base`, low bit set | 0x10f5 |
| `iImportOffset` | `156 + code + data` | 0x14e8 |

## Not observed → TODO errors (no invented behaviour)

ELF with no writable `PT_LOAD`; ELF without the exception-descriptor symbol.

## Remaining (next T4 slices)

- ~~`iDllRefTableCount` and the import section~~ done in the imports slice (experiment 44): `E32ImportSection::from_elf`, byte-equal to the uncompressed golden
- ~~Code relocations~~ done (experiment 44): `E32RelocSection::code_from_elf`, byte-equal
- ~~Code words at import slots~~ done (experiment 44): `E32CodeSection::from_elf` byte-equal; `Elf2E32::ordinals` reads `--libpath` DSOs; tests pin the 33 used ordinals
- ~~Full uncompressed image~~ done: `E32Image::exe(...).uncompressed()` byte-equals experiment-44 `hello_u.exe`; `Elf2E32::encode` + bin write it for `--uncompressed` (caps via shared `symdev_core::Capabilities`, live `E32Time`). Experiment 45: native build installs and runs in EKA2L1.
- Next: Symbian deflate for the default (compressed) output; wire native encode into `GcceBuild` instead of `SYMDEV_ELF2E32`
- Capability bit packing; deflate of the post-header payload; `Elf2E32::encode`; wiring into `GcceBuild`
