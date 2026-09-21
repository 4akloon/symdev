# WIP: Avkon notes (`symbian-ui::note`)

Task: wrap the Avkon note popups (information / confirmation / warning / error) as
`note::info/confirm/warn/error(&str) -> Result`, via a new `shims/s60/symrs_note.cpp`,
so examples stop using `User::InfoPrint`.

## Findings

## Decisions

## Dead ends

## Next step

- Read `docs/research/avkon-rust-spec.md` §3, §5; experiment 86; `symbian-ui/`; `symrs_avkon.cpp`.
