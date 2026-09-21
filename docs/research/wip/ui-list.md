# WIP: S60 list box for symbian-ui (branch `ui-list`)

Task: add an S60 Avkon list box to the symdev Rust SDK — `symbian-ui/src/list.rs` +
`shims/s60/symrs_list.cpp` — with a safe `List::new/set_items/selected/on_select`
surface, proven in the emulator with arrow keys and Return.

## Findings

## Decisions

## Dead ends

## Next step

- Read the spec (§3 forwarding ABI, §4 control/key contracts, §5 per-virtual leave rule),
  `eka2l1-input.md`, experiments 83 and 86, then `symbian-ui/` and `symrs_avkon.cpp` 198-200.
