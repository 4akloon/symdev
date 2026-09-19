# symdev — Claude Code

Shared agent rules (behavior, host, workflow):

@AGENTS.md

Code rules (shared with Cursor, always apply):

@.cursor/rules/production-quality.mdc
@.cursor/rules/rust-types-and-methods.mdc

## Claude-specific

- Before reporting done: `cargo test --workspace --offline` and `cargo clippy --workspace --offline` with no new warnings.
- Cursor session history (for context only, not truth): `~/.cursor/projects/home-genius-*symdev*/agent-transcripts/*/*.jsonl`.
- Reply in the user's language (usually Ukrainian).
