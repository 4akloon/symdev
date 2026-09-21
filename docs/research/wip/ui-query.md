# WIP: Avkon query dialogs (branch `ui-query`, backlog 93)

Task: add modal Avkon query dialogs (text / number / confirmation) to `symbian-ui` as
`symbian-rs/crates/symbian-ui/src/query.rs` + `symbian-rs/shims/s60/symrs_query.cpp`,
with a Rust surface that names no Symbian type, and drive one in EKA2L1 as evidence.

## Findings

## Decisions

## Dead ends

## Next step

Read the SDK's `aknquerydialog.h`, `nm -D` the right `.dso`, and settle the resource ids.
