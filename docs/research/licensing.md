# Licensing and repo hygiene

Bootstrap note for M0. Promoted from spec §19. No `LICENSE` file in this cycle.

## Repo license

Repo license: **MIT** (`LICENSE`), chosen by the repository owner on 2026-09-22 when the
repository was prepared for publication. Every crate declares `license.workspace = true`.

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

### Clean-room record: the remaining SDK tools (2026-09-20)

Same two-role split, approved by the repository owner, for the tools symdev still shells out to or refuses cases of: separate agents disassembled `bmconv.exe`, `svgtbinencode.exe` + `mifconv.exe`, `makesis.exe` and `elf2e32.exe` (for the latter they could also read the EPL-1.0 `elf2e32_next` sources) and wrote prose specifications — [bmconv-spec.md](bmconv-spec.md), [svgb-mif-spec.md](svgb-mif-spec.md), [makesis-spec.md](makesis-spec.md), [elf2e32-options-spec.md](elf2e32-options-spec.md). The native implementations are written from those specs and the golden bytes only; their author read neither the disassembly nor the spec writers' scratch work.

### Clean-room record: the `bld.inf` / `.mmp` front end (2026-09-20)

Same two-role split for the last piece symdev still has no native equivalent of. A separate agent
read the SDK's own build system — the Perl programs and modules under
`/home/genius/sdk/S60_3rd_FP2/epoc32/tools/` that preprocess and parse `bld.inf` and `.mmp`, the
GCCE compilation-configuration file, and the shipped variant header — and wrote
[mmp-frontend-spec.md](mmp-frontend-spec.md) in prose and tables: directive grammars, defaults,
path resolution, the two macro namespaces, the resource and bitmap block rules, and the exact GCCE
ARMV5 UREL flag order, with the preprocessor's behaviour re-confirmed by black-box runs of the
SDK's own `cpp.exe` under Wine. No Perl, no identifiers from those sources, no copied comments or
string literals; scratch probes were kept outside the repository. The Rust front end is written
from that specification alone, and its author read neither the SDK's Perl nor the spec writer's
scratch work.
