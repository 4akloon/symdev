# 96 — localisation of the strings the application itself reads (2026-09-21)

`localedemo.exe`, 9 684 bytes, is `symbian-rs/examples/locale`. It declares three
languages and three keys in `src/strings.rs` through `symbian_std::locale!`, asks
`User::Language()` what the device is set to, and writes every raw number and every
chosen string to `E:\symdev\locale\measured.txt`, which is what the three
`measured-lang*.txt` files here are.

Design note: [`docs/research/locale-rust-design.md`](../../../docs/research/locale-rust-design.md).

## The one thing that had to be measured

`User::Language()` in EKA2L1 **returns what `config.yml`'s `language:` says**, and the
strings follow it. The three files are three runs of the *same* `localedemo.sisx`,
changing nothing but that key:

| `measured-lang1.txt` | `measured-lang2.txt` | `measured-lang3.txt` |
|---|---|---|
| `user_language_raw=1` `ELangEnglish` | `user_language_raw=2` `ELangFrench` | `user_language_raw=3` `ELangGerman` |
| `GREETING=Hello from Rust` | `GREETING=Bonjour depuis Rust` | `GREETING=Hello from Rust` |
| `LANGUAGE_IS=language` | `LANGUAGE_IS=langue` | `LANGUAGE_IS=language` |
| `OK=ok` | `OK=d'accord` | `OK=ok` |

All three `localedemo: 8 passed`. The third column is the fallback rule doing its job:
the table has no German, so the declared default answers.

**The emulator's language can only be set to a language the ROM lists.** EKA2L1 checks
`language:` against the device's `Z:\resource\bootdata\languages.txt` and rewrites the
config to the ROM default otherwise. RM-469's file, UTF-16LE, is `01,d 02 03 14 18 05` —
English (default), French, German, Turkish, Dutch, Italian. `language: 93` came back as
`language: 1`. So **Ukrainian, Russian and every dialect enumerator cannot be reached on
this ROM**; those rows of the chain are exercised through `Text::get_in`, which takes a
language rather than asking for one, and they pass on the device (`chain[ukrainian]`,
`chain[english_apac]`, `chain[none]` in each file) — but that is not evidence about a
device actually *set* to them. No E52 either way.

## What one euser call costs

`calls=100000 ticks_asking_euser=12 ticks_local=0` in all three files: 100 000
`Language::current()` in 12 nanokernel ticks of 1 000 µs, so **0.12 µs** per
`User::Language()`. The same loop through a `static AtomicU32` cache took **16**, because
ARMv5TE has no atomic instruction and `__atomic_load_4` is an `RFastLock` Wait/Signal
pair. That is why nothing is cached.

## Size

`sizes.md` has the table. The short version: a one-language `locale!` is **3 183** bytes
where the same program with a plain `const` is **3 187**, and the `AtomicU32` cache that
was tried first cost **794**.

## Compile errors

`compile-errors.md` is the transcript of all six mistakes the design turns into a
compile error, produced by `cargo build --release` for the phone target, not written
from memory.
