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

- `iDllRefTableCount` (2 on hello, though the ELF has 6 `DT_NEEDED`: counts only libraries with used imports) and the import section — needs versioned-symbol (`.gnu.version` / `verneed`) parsing
- Code relocations (`iCodeRelocOffset`)
- Capability bit packing; deflate of the post-header payload; `Elf2E32::encode`; wiring into `GcceBuild`
