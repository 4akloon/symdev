# WIP: experiments 68 (`alloc`) and 69 (`symbian-core`)

Task: give the Rust SDK a `GlobalAlloc` over `User::Alloc`/`Free`/`ReAlloc` (68) and a safe
`symbian-core` with errors and the descriptor family (69), with `examples/alloc` and a
rewritten, `unsafe`-free `examples/hello` proving both in EKA2L1's notifier log.

## Findings

## Decisions

## Dead ends

## Next step

- Read the design spec §5–§7, §11 and backlog 65/65a; then measure `User::Alloc` alignment.
