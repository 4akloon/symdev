# locale-rust — a Rust-side localisation system for in-application strings

Task: `no_std`, zero-allocation, compile-time-checked localisation of strings read by our own
Rust (not the launcher's application name — that half belongs to `ui_resources.rs`/SIS and is
not mine). Key must be a compile error when missing/mistyped; a key missing from one language
must be caught at build time. Language read once from `User::Language()`.

## Findings

## Decisions

## Dead ends

## Next step
- Read the repo: symbian-sys euser statics, symbian-macros shape, examples and their sizes.
