# Licensing and repo hygiene

Bootstrap note for M0. Promoted from spec §19. No `LICENSE` file in this cycle.

## Repo license

Repo license: **undecided**. Do not add `LICENSE` in this cycle.

## Never bundle

Never commit, COPY, curl, scrape, or otherwise bundle:

- S60 SDK
- WTK
- ROM / firmware dumps
- certificates (`.cer`)
- private keys (`.key`)

SDK, WTK, and ROM stay on the operator’s machine. They are user-supplied paths, never downloaded by this repository.

## EKA2L1

EKA2L1 is GPL-3.0. Invoke it as a **separate process** only. Do not copy EKA2L1 source into this tree.

## Legacy tool licenses (Verified)

- Original `elf2e32` is EPL-1.0.
- `rcomp` / `bmconv` / `petran` / `uidcrc` use the Symbian Example Source Code License (Verified).

Any future C++ ports live in **separate modules/submodules** with notices intact — not in §17.

Future Rust reimplementations are **clean-room** from format specs and golden behaviour.

### Clean-room record: E32 deflate (2026-09-19)

`crates/symdev-elf2e32/src/deflate.rs` was written only from [e32-deflate-spec.md](e32-deflate-spec.md). A separate agent read elf2e32_next (EPL-1.0) and wrote that spec in prose and tables (no code, no source identifiers, no copied comments); the implementer did not read the elf2e32_next source or the spec writer's throwaway scripts. Keep this split for future ports: spec from the source by one party, code from the spec by another.

Same split for the DSO `.hash` bucket rule: [dso-hash-spec.md](dso-hash-spec.md) (2026-09-19) was written by a separate agent from elf2e32_next; `crates/symdev-elf2e32/src/dso.rs` implements it without reading the source.

### Clean-room record: rcomp (2026-09-19)

The repository owner approved disassembling the SDK's `rcomp.exe` (8.1, build 004) to pin the parts black-box goldens leave open (compressed-Unicode encoder choices, when text stays uncompressed, defaults). Same two-role split: a separate agent disassembled the binary and wrote [rcomp-spec.md](rcomp-spec.md) in prose and tables; the native compiler in `crates/symdev-rcomp` is written from that spec, the experiment-56 golden corpus and black-box probes, and its author did not read the disassembly or the spec writer's scratch files.
