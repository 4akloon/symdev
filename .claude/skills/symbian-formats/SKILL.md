---
name: symbian-formats
description: Use when implementing or changing any Symbian binary or text format in symdev (E32 images, SIS packages, .rsc resources, .dso/.def, MIF icons, signing) — how we derive bytes from goldens and experiments, where fixtures and records live, and the clean-room rule.
---

# Byte-exact reimplementation

symdev replaces SDK tools (`elf2e32`, `makesis`, `signsis`, `rcomp`, `uidcrc`, `makekeys`). The bar is **byte-identical output**, not "works". Everything below exists so the next agent can check a byte and know where it came from.

## Derive, never invent

Every byte, flag, offset and default comes from one of:

- a **golden** produced by the original tool on this host (keep the input that produced it);
- a recorded **experiment** in `docs/research/experiment-backlog.md`;
- a **clean-room spec** in `docs/research/*-spec.md`.

If behaviour was never observed, do not guess it: return an error that says so.

```rust
// DO — the shape we use for anything unobserved
return Err(Error::Other(format!(
    "TODO: native elf2e32 --fpu={fpu} (only softvfp observed)"
)));
```

Grep for `TODO:` to see the honest edge of the implementation. Removing one means adding the golden that pins it.

## Cite the source in the code

A constant, layout rule or heuristic carries a short doc comment naming the experiment or spec section it came from — `(experiment 52)`, `(spec §1.3)`. Comments explain *why the bytes are these*, not what the line does.

```rust
/// Export directory appended to the code: `u32 count`, then one link address per
/// ordinal (experiment 52).
```

## Record what you ran

New behaviour discovered by running a tool → a numbered entry in `docs/research/experiment-backlog.md` with **Procedure**, **Outcome** and **Evidence** (date, host, exact argv, byte counts). Scratch work lives outside git (`~/src/symdev-experiment-NN/`); never commit SDK inputs, ROMs, `.sis`, `.cer`, `.key`.

## Tests pin bytes, and run offline

- Golden bytes live as hex or text fixtures in the crate's `src/testdata/`, with the source that produced them next to them (`exp56_double.rss` + `exp56_double.rsc.hex`).
- `cargo test --workspace --offline` never spawns Wine, the emulator, or a network call. Comparison runs against the real tools are scripts outside git.
- One test per rule, named after the rule (`compress_only_when_shorter`, `name_value_is_base_27_letters_with_digits_zero`), so a failure names the rule that broke.

## Clean-room

Reimplementations are written from specs and goldens. When a spec was written by reading foreign source or a disassembly, the spec writer and the implementer are different agents, and the record goes in `docs/research/licensing.md`. Never copy code or identifiers out of the original tool.

## Errors say what to do next

An error names the file or member that failed and, when the fix is known, the fix: `"DLL contains initialized writable data; add EPOCALLOWDLLDATA to the MMP (elf2e32 --dlldata)"`.

## Rust API shape for a format type

A format is a type with methods, not a module of functions: `UidCrc::checked`, `SisField::bytes`, `E32Image::new`, `RscCompiled::rsc_bytes`. Parsing returns the type; encoding is a method on it. Repeated SIS KIND + payload goes through `SisEncode`, not copy-paste. Crate-root `pub use` is append-only and alphabetical where it already is.

Adapters carry the host: `UidCrcTool::args`, `MifConvTool::args`, `Toolchain::from_env`. A value type never learns about Wine, EPOCROOT or argv.

## Where things live

- Experiment records and format notes: `docs/research/experiment-backlog.md`, `docs/research/*-spec.md`, `docs/research/hardcoded-values.md`.
- Golden fixtures: `crates/<crate>/src/testdata/`.
- Comparison scripts against the real tools and their corpora: outside git, under `~/src/symdev-experiment-NN/`.
