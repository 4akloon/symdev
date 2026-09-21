# WIP: command-id design

Task: find the best design that ties a menu item's command number in the generated `.rss` to the Rust source so a mistyped / stale command is a compile error, not a silent no-op. Deliverable: `docs/research/command-id-design.md` + prototype on branch `command-id-design`.

## Findings

## Decisions

## Dead ends

## Next step
- Read current implementation: `crates/symdev-manifest/src/command_id.rs`, `symbian-ui` `Command::named`, `crates/symdev-build/src/ui_resources.rs`, `symbian-rs/crates/symbian-macros`.
